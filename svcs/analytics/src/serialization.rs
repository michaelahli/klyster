//! Conversion between Klyster domain metrics and `analytics.MetricDataPoint` protobuf messages.
//!
//! The Seer protocol uses `int64` Unix-second timestamps and a `map<string,string>` for
//! labels; this module translates between that wire format and the [`Metric`] /
//! [`MetricLabel`] structs from the `domain` crate. NaN and ±Inf values are dropped during
//! serialization because they are not meaningful inputs to forecasting models — callers
//! that need different handling should sanitize beforehand.

use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use domain::models::{Metric, MetricLabel};
use thiserror::Error;

use crate::proto::MetricDataPoint;

/// Errors produced when converting protobuf data points back into domain metrics.
#[derive(Error, Debug)]
pub enum SerializationError {
    /// Timestamp could not be represented as a UTC `DateTime`.
    #[error("invalid timestamp {0}: out of representable UTC range")]
    InvalidTimestamp(i64),

    /// Value is NaN or ±Infinity, which forecasting models cannot consume.
    #[error("non-finite metric value at index {index}")]
    NonFiniteValue {
        /// Index of the offending point in the input slice.
        index: usize,
    },
}

/// Convert domain metrics into protobuf `MetricDataPoint`s.
///
/// `labels_by_metric_id` is consulted per-metric to attach dimensional labels.
/// Points with non-finite values are skipped; the returned list preserves
/// input order otherwise.
#[allow(clippy::implicit_hasher)] // callers use std::collections::HashMap; generics hurt inference at None call sites.
#[must_use] 
pub fn metrics_to_data_points(
    metrics: &[Metric],
    labels_by_metric_id: Option<&HashMap<i64, Vec<MetricLabel>>>,
) -> Vec<MetricDataPoint> {
    let mut out = Vec::with_capacity(metrics.len());
    for metric in metrics {
        if !metric.value.is_finite() {
            continue;
        }
        let labels = labels_by_metric_id
            .and_then(|map| map.get(&metric.id))
            .map(|labels| {
                labels
                    .iter()
                    .map(|l| (l.key.clone(), l.value.clone()))
                    .collect()
            })
            .unwrap_or_default();
        out.push(MetricDataPoint {
            timestamp: metric.timestamp.timestamp(),
            value: metric.value,
            labels,
        });
    }
    out
}

/// Convert a single domain metric into a protobuf `MetricDataPoint`.
///
/// Returns `None` for non-finite values so callers can decide how to filter.
#[must_use]
pub fn metric_to_data_point(
    metric: &Metric,
    labels: Option<&[MetricLabel]>,
) -> Option<MetricDataPoint> {
    if !metric.value.is_finite() {
        return None;
    }
    let labels_map = labels
        .map(|labels| {
            labels
                .iter()
                .map(|l| (l.key.clone(), l.value.clone()))
                .collect()
        })
        .unwrap_or_default();
    Some(MetricDataPoint {
        timestamp: metric.timestamp.timestamp(),
        value: metric.value,
        labels: labels_map,
    })
}

/// Convert protobuf `MetricDataPoint`s into domain `Metric`s.
///
/// Database-managed columns (`id`, `created_at`) are populated with sentinels
/// (`0` and the conversion timestamp) since the wire format does not carry
/// them. Use this when ingesting forecast inputs or test fixtures, not when
/// reading from the database.
pub fn data_points_to_metrics(
    points: &[MetricDataPoint],
    source_id: i64,
    name: &str,
) -> Result<Vec<Metric>, SerializationError> {
    let now = Utc::now();
    let mut out = Vec::with_capacity(points.len());
    for (index, point) in points.iter().enumerate() {
        if !point.value.is_finite() {
            return Err(SerializationError::NonFiniteValue { index });
        }
        let timestamp = unix_seconds_to_datetime(point.timestamp)?;
        out.push(Metric {
            id: 0,
            source_id,
            name: name.to_string(),
            value: point.value,
            timestamp,
            created_at: now,
        });
    }
    Ok(out)
}

/// Convert a Unix-second timestamp to a UTC `DateTime`.
fn unix_seconds_to_datetime(seconds: i64) -> Result<DateTime<Utc>, SerializationError> {
    Utc.timestamp_opt(seconds, 0)
        .single()
        .ok_or(SerializationError::InvalidTimestamp(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_metric(id: i64, ts_secs: i64, value: f64) -> Metric {
        Metric {
            id,
            source_id: 1,
            name: "cpu_usage".to_string(),
            value,
            timestamp: Utc.timestamp_opt(ts_secs, 0).unwrap(),
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
        }
    }

    fn make_label(metric_id: i64, key: &str, value: &str) -> MetricLabel {
        MetricLabel {
            id: 0,
            metric_id,
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn metrics_to_data_points_preserves_order_and_values() {
        let metrics = vec![
            make_metric(1, 100, 1.0),
            make_metric(2, 200, 2.5),
            make_metric(3, 300, 3.25),
        ];
        let points = metrics_to_data_points(&metrics, None);
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].timestamp, 100);
        assert!((points[1].value - 2.5).abs() < f64::EPSILON);
        assert!(points[2].labels.is_empty());
    }

    #[test]
    fn metrics_to_data_points_attaches_labels() {
        let metrics = vec![make_metric(7, 100, 1.0)];
        let mut labels_map = HashMap::new();
        labels_map.insert(
            7,
            vec![
                make_label(7, "pod", "api-1"),
                make_label(7, "ns", "default"),
            ],
        );

        let points = metrics_to_data_points(&metrics, Some(&labels_map));
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].labels.get("pod"), Some(&"api-1".to_string()));
        assert_eq!(points[0].labels.get("ns"), Some(&"default".to_string()));
    }

    #[test]
    fn metrics_to_data_points_skips_non_finite() {
        let metrics = vec![
            make_metric(1, 1, 1.0),
            make_metric(2, 2, f64::NAN),
            make_metric(3, 3, f64::INFINITY),
            make_metric(4, 4, 4.0),
        ];
        let points = metrics_to_data_points(&metrics, None);
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].timestamp, 1);
        assert_eq!(points[1].timestamp, 4);
    }

    #[test]
    fn data_points_to_metrics_round_trip() {
        let original = vec![
            make_metric(1, 100, 1.0),
            make_metric(2, 200, 2.0),
            make_metric(3, 300, 3.0),
        ];
        let points = metrics_to_data_points(&original, None);
        let restored = data_points_to_metrics(&points, 1, "cpu_usage").unwrap();
        assert_eq!(restored.len(), original.len());
        for (a, b) in restored.iter().zip(original.iter()) {
            assert_eq!(a.timestamp.timestamp(), b.timestamp.timestamp());
            assert!((a.value - b.value).abs() < f64::EPSILON);
            assert_eq!(a.name, b.name);
            assert_eq!(a.source_id, b.source_id);
        }
    }

    #[test]
    fn data_points_to_metrics_rejects_nan() {
        let points = vec![
            MetricDataPoint {
                timestamp: 100,
                value: 1.0,
                labels: HashMap::new(),
            },
            MetricDataPoint {
                timestamp: 200,
                value: f64::NAN,
                labels: HashMap::new(),
            },
        ];
        let err = data_points_to_metrics(&points, 1, "cpu").unwrap_err();
        assert!(matches!(
            err,
            SerializationError::NonFiniteValue { index: 1 }
        ));
    }

    #[test]
    fn data_points_to_metrics_rejects_invalid_timestamp() {
        let points = vec![MetricDataPoint {
            timestamp: i64::MAX,
            value: 1.0,
            labels: HashMap::new(),
        }];
        let err = data_points_to_metrics(&points, 1, "cpu").unwrap_err();
        assert!(matches!(err, SerializationError::InvalidTimestamp(_)));
    }

    #[test]
    fn metric_to_data_point_filters_non_finite() {
        let metric = make_metric(1, 100, f64::NAN);
        assert!(metric_to_data_point(&metric, None).is_none());
    }

    #[test]
    fn serialization_throughput_under_budget() {
        // CP-M4-011 acceptance: 10k points serialize in < 50ms.
        let metrics: Vec<Metric> = (0..10_000)
            .map(|i| make_metric(i64::from(i), i64::from(i), f64::from(i)))
            .collect();
        let started = std::time::Instant::now();
        let points = metrics_to_data_points(&metrics, None);
        let elapsed = started.elapsed();
        assert_eq!(points.len(), 10_000);
        assert!(
            elapsed.as_millis() < 50,
            "serialization took {} ms, budget is 50 ms",
            elapsed.as_millis()
        );
    }
}
