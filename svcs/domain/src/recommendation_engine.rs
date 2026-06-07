//! Pure decision logic for translating forecasts into scaling recommendations.
//!
//! The engine has no I/O dependencies; callers feed it a forecast summary and a
//! [`ScalingTarget`] and receive a [`RecommendationDraft`] to persist alongside
//! the forecast.

use crate::models::{RecommendationAction, RecommendationStatus, ScalingTarget};
use crate::provider::Capacity;

/// Default scale-up multiplier on the target value (predicted > target * 1.2 → scale up).
pub const DEFAULT_SCALE_UP_MULTIPLIER: f64 = 1.2;

/// Default scale-down multiplier on the target value (predicted < target * 0.8 → scale down).
pub const DEFAULT_SCALE_DOWN_MULTIPLIER: f64 = 0.8;

/// Tunable thresholds for the recommendation engine.
#[derive(Debug, Clone, Copy)]
pub struct RecommendationPolicy {
    /// Predicted-value multiplier above which we recommend scaling up.
    pub scale_up_multiplier: f64,
    /// Predicted-value multiplier below which we recommend scaling down.
    pub scale_down_multiplier: f64,
    /// Minimum delta (in replicas) required to emit a non-`None` recommendation.
    pub min_replica_delta: i32,
}

impl Default for RecommendationPolicy {
    fn default() -> Self {
        Self {
            scale_up_multiplier: DEFAULT_SCALE_UP_MULTIPLIER,
            scale_down_multiplier: DEFAULT_SCALE_DOWN_MULTIPLIER,
            min_replica_delta: 1,
        }
    }
}

/// Inputs to the recommendation engine.
#[derive(Debug, Clone)]
pub struct ForecastSummary {
    /// Maximum predicted value over the forecast horizon (used for scale-up).
    pub peak_predicted: f64,
    /// Mean predicted value over the horizon (used for scale-down).
    pub mean_predicted: f64,
    /// Current replica count for the resource group.
    pub current_replicas: i32,
}

/// Capacity-aware inputs to the recommendation engine.
#[derive(Debug, Clone)]
pub struct RecommendationInput<'a> {
    /// Forecast summary.
    pub forecast: &'a ForecastSummary,
    /// Current provider capacity for the resource group.
    pub capacity: &'a Capacity,
    /// Scaling target for bounds and threshold value.
    pub target: &'a ScalingTarget,
    /// Recommendation policy.
    pub policy: RecommendationPolicy,
    /// Optional forecast/model confidence score.
    pub forecast_confidence: Option<f64>,
}

/// Output of the recommendation engine: the fields we will persist as a
/// `Recommendation` row, modulo the IDs and timestamps the database fills in.
#[derive(Debug, Clone, PartialEq)]
pub struct RecommendationDraft {
    /// The action to take.
    pub action: RecommendationAction,
    /// Current replica count (echoed from the input).
    pub current_count: i32,
    /// The recommended new replica count.
    pub recommended_count: i32,
    /// Human-readable explanation.
    pub reason: String,
    /// Confidence score for the recommendation, from 0.0 to 1.0.
    pub confidence: f64,
    /// Status the recommendation should be created with.
    pub status: RecommendationStatus,
}

/// Decide whether a scaling action is warranted given a forecast summary.
///
/// Logic:
/// * If `peak_predicted > target * scale_up_multiplier` → recommend scaling up.
///   New count is computed proportionally: `ceil(current * peak / target)`,
///   bounded to `[min_replicas, max_replicas]`.
/// * Else if `mean_predicted < target * scale_down_multiplier` → recommend scaling down.
///   New count is `ceil(current * mean / target)`, bounded the same way.
/// * Otherwise → `None` action with the current count.
///
/// A non-`None` action is downgraded back to `None` if the new replica count
/// is identical to (or differs by less than `min_replica_delta` from) the
/// current count, since there is nothing to apply.
#[must_use]
pub fn evaluate(
    summary: &ForecastSummary,
    target: &ScalingTarget,
    policy: RecommendationPolicy,
) -> RecommendationDraft {
    let current = u32::try_from(summary.current_replicas).unwrap_or(0);
    let capacity = Capacity {
        current,
        desired: current,
        min: u32::try_from(target.min_replicas).unwrap_or(0),
        max: u32::try_from(target.max_replicas).unwrap_or(0),
    };
    evaluate_capacity(&RecommendationInput {
        forecast: summary,
        capacity: &capacity,
        target,
        policy,
        forecast_confidence: None,
    })
}

/// Decide whether a scaling action is warranted from forecast and provider capacity.
#[must_use]
pub fn evaluate_capacity(input: &RecommendationInput<'_>) -> RecommendationDraft {
    let summary = input.forecast;
    let target = input.target;
    let policy = input.policy;
    let current_replicas = i32::try_from(input.capacity.current).unwrap_or(i32::MAX);
    let desired_replicas = i32::try_from(input.capacity.desired).unwrap_or(i32::MAX);
    let min_replicas = target
        .min_replicas
        .max(i32::try_from(input.capacity.min).unwrap_or(0));
    let max_replicas = target
        .max_replicas
        .min(i32::try_from(input.capacity.max).unwrap_or(i32::MAX));
    let confidence = recommendation_confidence(summary, target, policy, input.forecast_confidence);

    if !target.target_value.is_finite() || target.target_value <= 0.0 {
        return RecommendationDraft {
            action: RecommendationAction::None,
            current_count: current_replicas,
            recommended_count: current_replicas,
            reason: "scaling target value is non-positive; no recommendation".to_string(),
            confidence,
            status: RecommendationStatus::Pending,
        };
    }

    let scale_up_threshold = target.target_value * policy.scale_up_multiplier;
    let scale_down_threshold = target.target_value * policy.scale_down_multiplier;

    let Some((action, raw_count, basis)) = choose_action(summary, target, policy, current_replicas)
    else {
        return RecommendationDraft {
            action: RecommendationAction::None,
            current_count: current_replicas,
            recommended_count: current_replicas,
            reason: format!(
                "predicted values within thresholds (peak {:.3}, mean {:.3}, target {:.3}){}",
                summary.peak_predicted,
                summary.mean_predicted,
                target.target_value,
                drift_suffix(current_replicas, desired_replicas),
            ),
            confidence,
            status: RecommendationStatus::Pending,
        };
    };

    let bounded = raw_count.clamp(min_replicas, max_replicas);
    let delta = (bounded - current_replicas).abs();
    if delta < policy.min_replica_delta {
        return RecommendationDraft {
            action: RecommendationAction::None,
            current_count: current_replicas,
            recommended_count: current_replicas,
            reason: format!(
                "{action_str} indicated by {basis} but replica delta {delta} below threshold{}",
                drift_suffix(current_replicas, desired_replicas),
                action_str = action.as_str(),
            ),
            confidence,
            status: RecommendationStatus::Pending,
        };
    }

    let reason = match action {
        RecommendationAction::ScaleUp => format!(
            "peak predicted {:.3} exceeds target {:.3} * {:.2} ({:.3}); scaling {} -> {}{}",
            summary.peak_predicted,
            target.target_value,
            policy.scale_up_multiplier,
            scale_up_threshold,
            current_replicas,
            bounded,
            drift_suffix(current_replicas, desired_replicas),
        ),
        RecommendationAction::ScaleDown => format!(
            "mean predicted {:.3} below target {:.3} * {:.2} ({:.3}); scaling {} -> {}{}",
            summary.mean_predicted,
            target.target_value,
            policy.scale_down_multiplier,
            scale_down_threshold,
            current_replicas,
            bounded,
            drift_suffix(current_replicas, desired_replicas),
        ),
        RecommendationAction::None => unreachable!(),
    };

    RecommendationDraft {
        action,
        current_count: current_replicas,
        recommended_count: bounded,
        reason,
        confidence,
        status: RecommendationStatus::Pending,
    }
}

fn choose_action(
    summary: &ForecastSummary,
    target: &ScalingTarget,
    policy: RecommendationPolicy,
    current_replicas: i32,
) -> Option<(RecommendationAction, i32, &'static str)> {
    let scale_up_threshold = target.target_value * policy.scale_up_multiplier;
    let scale_down_threshold = target.target_value * policy.scale_down_multiplier;

    if summary.peak_predicted > scale_up_threshold {
        let scaled = scale_count(
            current_replicas,
            summary.peak_predicted,
            target.target_value,
        );
        Some((RecommendationAction::ScaleUp, scaled, "peak"))
    } else if summary.mean_predicted < scale_down_threshold {
        let scaled = scale_count(
            current_replicas,
            summary.mean_predicted,
            target.target_value,
        );
        Some((RecommendationAction::ScaleDown, scaled, "mean"))
    } else {
        None
    }
}

fn drift_suffix(current_replicas: i32, desired_replicas: i32) -> String {
    if current_replicas == desired_replicas {
        String::new()
    } else {
        format!("; capacity drift current {current_replicas} desired {desired_replicas}")
    }
}

fn recommendation_confidence(
    summary: &ForecastSummary,
    target: &ScalingTarget,
    policy: RecommendationPolicy,
    forecast_confidence: Option<f64>,
) -> f64 {
    let model_confidence = forecast_confidence
        .filter(|value| value.is_finite())
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    if !target.target_value.is_finite() || target.target_value <= 0.0 {
        return 0.0;
    }

    let up_margin =
        (summary.peak_predicted / (target.target_value * policy.scale_up_multiplier)) - 1.0;
    let down_margin =
        1.0 - (summary.mean_predicted / (target.target_value * policy.scale_down_multiplier));
    let signal = up_margin.max(down_margin).clamp(0.0, 1.0);
    (0.5 + signal / 2.0) * model_confidence
}

/// Compute a new replica count proportionally to the predicted/target ratio.
///
/// Uses `ceil` for scale-up and `floor` (via integer truncation) for scale-down
/// to err on the side of capacity availability. The minimum returned value is 1
/// so we never propose zero replicas; the bounding step in `evaluate` clamps
/// to the target's min/max.
fn scale_count(current: i32, predicted: f64, target_value: f64) -> i32 {
    let ratio = predicted / target_value;
    let raw = f64::from(current) * ratio;
    let scaled = if ratio >= 1.0 {
        raw.ceil()
    } else {
        raw.floor()
    };
    if scaled.is_nan() || scaled <= 0.0 {
        return 1;
    }
    // Clamp to i32 range to avoid panics on absurd inputs.
    if scaled >= f64::from(i32::MAX) {
        return i32::MAX;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let result = scaled as i32;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn target(min: i32, max: i32, value: f64) -> ScalingTarget {
        let now = Utc::now();
        ScalingTarget {
            id: 1,
            resource_group_id: 1,
            metric_name: "cpu_usage".to_string(),
            min_replicas: min,
            max_replicas: max,
            target_value: value,
            created_at: now,
            updated_at: now,
        }
    }

    fn capacity(current: u32, desired: u32, min: u32, max: u32) -> Capacity {
        Capacity {
            current,
            desired,
            min,
            max,
        }
    }

    #[test]
    fn capacity_input_uses_provider_current_and_bounds() {
        let summary = ForecastSummary {
            peak_predicted: 1.4,
            mean_predicted: 1.0,
            current_replicas: 1,
        };
        let cap = capacity(4, 4, 2, 6);
        let draft = evaluate_capacity(&RecommendationInput {
            forecast: &summary,
            capacity: &cap,
            target: &target(1, 10, 0.7),
            policy: RecommendationPolicy::default(),
            forecast_confidence: Some(0.9),
        });

        assert_eq!(draft.action, RecommendationAction::ScaleUp);
        assert_eq!(draft.current_count, 4);
        assert_eq!(draft.recommended_count, 6);
        assert!(draft.confidence > 0.0 && draft.confidence <= 0.9);
    }

    #[test]
    fn scale_down_threshold_defaults_to_eighty_percent() {
        let summary = ForecastSummary {
            peak_predicted: 0.5,
            mean_predicted: 0.55,
            current_replicas: 6,
        };
        let draft = evaluate(
            &summary,
            &target(1, 10, 0.7),
            RecommendationPolicy::default(),
        );

        assert_eq!(draft.action, RecommendationAction::ScaleDown);
    }

    #[test]
    fn capacity_drift_is_included_in_reason() {
        let summary = ForecastSummary {
            peak_predicted: 0.5,
            mean_predicted: 0.7,
            current_replicas: 3,
        };
        let cap = capacity(2, 5, 1, 10);
        let draft = evaluate_capacity(&RecommendationInput {
            forecast: &summary,
            capacity: &cap,
            target: &target(1, 10, 0.7),
            policy: RecommendationPolicy::default(),
            forecast_confidence: Some(1.0),
        });

        assert_eq!(draft.action, RecommendationAction::None);
        assert!(draft.reason.contains("capacity drift current 2 desired 5"));
    }

    #[test]
    fn invalid_forecast_confidence_is_clamped_to_default() {
        let summary = ForecastSummary {
            peak_predicted: 1.2,
            mean_predicted: 0.9,
            current_replicas: 3,
        };
        let cap = capacity(3, 3, 1, 10);
        let draft = evaluate_capacity(&RecommendationInput {
            forecast: &summary,
            capacity: &cap,
            target: &target(1, 10, 0.7),
            policy: RecommendationPolicy::default(),
            forecast_confidence: Some(f64::NAN),
        });

        assert!(draft.confidence > 0.5);
        assert!(draft.confidence <= 1.0);
    }

    #[test]
    fn scale_up_when_peak_exceeds_threshold() {
        let summary = ForecastSummary {
            peak_predicted: 0.95,
            mean_predicted: 0.8,
            current_replicas: 3,
        };
        let draft = evaluate(
            &summary,
            &target(1, 10, 0.7),
            RecommendationPolicy::default(),
        );
        assert_eq!(draft.action, RecommendationAction::ScaleUp);
        assert!(draft.recommended_count > 3);
        assert!(draft.recommended_count <= 10);
    }

    #[test]
    fn scale_down_when_mean_below_threshold() {
        let summary = ForecastSummary {
            peak_predicted: 0.4,
            mean_predicted: 0.3,
            current_replicas: 6,
        };
        let draft = evaluate(
            &summary,
            &target(1, 10, 0.7),
            RecommendationPolicy::default(),
        );
        assert_eq!(draft.action, RecommendationAction::ScaleDown);
        assert!(draft.recommended_count < 6);
        assert!(draft.recommended_count >= 1);
    }

    #[test]
    fn no_action_within_thresholds() {
        let summary = ForecastSummary {
            peak_predicted: 0.78,
            mean_predicted: 0.7,
            current_replicas: 4,
        };
        let draft = evaluate(
            &summary,
            &target(1, 10, 0.7),
            RecommendationPolicy::default(),
        );
        assert_eq!(draft.action, RecommendationAction::None);
        assert_eq!(draft.recommended_count, 4);
    }

    #[test]
    fn recommendation_clamped_to_max_replicas() {
        let summary = ForecastSummary {
            peak_predicted: 5.0,
            mean_predicted: 5.0,
            current_replicas: 3,
        };
        let draft = evaluate(
            &summary,
            &target(1, 5, 0.7),
            RecommendationPolicy::default(),
        );
        assert_eq!(draft.action, RecommendationAction::ScaleUp);
        assert_eq!(draft.recommended_count, 5);
    }

    #[test]
    fn recommendation_clamped_to_min_replicas() {
        let summary = ForecastSummary {
            peak_predicted: 0.05,
            mean_predicted: 0.01,
            current_replicas: 3,
        };
        let draft = evaluate(
            &summary,
            &target(2, 10, 0.7),
            RecommendationPolicy::default(),
        );
        assert_eq!(draft.action, RecommendationAction::ScaleDown);
        assert_eq!(draft.recommended_count, 2);
    }

    #[test]
    fn min_delta_downgrades_to_none() {
        let summary = ForecastSummary {
            peak_predicted: 0.85,
            mean_predicted: 0.7,
            current_replicas: 3,
        };
        let policy = RecommendationPolicy {
            min_replica_delta: 2,
            ..RecommendationPolicy::default()
        };
        let draft = evaluate(&summary, &target(1, 10, 0.7), policy);
        assert_eq!(draft.action, RecommendationAction::None);
        assert_eq!(draft.recommended_count, 3);
    }

    #[test]
    fn invalid_target_value_returns_none() {
        let summary = ForecastSummary {
            peak_predicted: 1.0,
            mean_predicted: 1.0,
            current_replicas: 1,
        };
        let draft = evaluate(
            &summary,
            &target(1, 10, 0.0),
            RecommendationPolicy::default(),
        );
        assert_eq!(draft.action, RecommendationAction::None);
    }
}
