//! WebSocket endpoint for real-time metric streaming.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::state::AppState;

/// WebSocket subscription message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Subscribe to metrics.
    #[serde(rename = "subscribe")]
    Subscribe {
        /// List of metric names to subscribe to.
        metrics: Vec<String>,
    },
    /// Unsubscribe from metrics.
    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        /// List of metric names to unsubscribe from.
        metrics: Vec<String>,
    },
    /// Ping message.
    #[serde(rename = "ping")]
    Ping,
    /// Pong response.
    #[serde(rename = "pong")]
    Pong,
}

/// WebSocket handler for metric streaming.
///
/// WS /api/v1/ws/metrics
pub async fn ws_metrics_handler(ws: WebSocketUpgrade, State(_state): State<AppState>) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    info!("WebSocket connection established");

    let mut subscribed_metrics: Vec<String> = Vec::new();
    let mut heartbeat = interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            // Handle incoming messages
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        debug!("Received WebSocket message: {}", text);

                        match serde_json::from_str::<WsMessage>(&text) {
                            Ok(WsMessage::Subscribe { metrics }) => {
                                info!("Subscribing to metrics: {:?}", metrics);
                                subscribed_metrics.extend(metrics.clone());

                                let response = serde_json::json!({
                                    "type": "subscribed",
                                    "metrics": metrics,
                                });

                                if socket.send(Message::Text(response.to_string())).await.is_err() {
                                    break;
                                }
                            }
                            Ok(WsMessage::Unsubscribe { metrics }) => {
                                info!("Unsubscribing from metrics: {:?}", metrics);
                                subscribed_metrics.retain(|m| !metrics.contains(m));

                                let response = serde_json::json!({
                                    "type": "unsubscribed",
                                    "metrics": metrics,
                                });

                                if socket.send(Message::Text(response.to_string())).await.is_err() {
                                    break;
                                }
                            }
                            Ok(WsMessage::Ping) => {
                                debug!("Received ping");
                                let pong = serde_json::json!({"type": "pong"});
                                if socket.send(Message::Text(pong.to_string())).await.is_err() {
                                    break;
                                }
                            }
                            Ok(WsMessage::Pong) => {
                                debug!("Received pong");
                            }
                            Err(e) => {
                                warn!("Failed to parse WebSocket message: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket connection closed by client");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    _ => {}
                }
            }

            // Send heartbeat ping
            _ = heartbeat.tick() => {
                debug!("Sending heartbeat ping");
                let ping = serde_json::json!({"type": "ping"});
                if socket.send(Message::Text(ping.to_string())).await.is_err() {
                    break;
                }
            }
        }
    }

    info!("WebSocket connection terminated");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_subscribe_serialization() {
        let msg = WsMessage::Subscribe {
            metrics: vec!["cpu_usage".to_string(), "memory_usage".to_string()],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("subscribe"));
        assert!(json.contains("cpu_usage"));
    }

    #[test]
    fn test_ws_message_subscribe_deserialization() {
        let json = r#"{"type":"subscribe","metrics":["cpu_usage","memory_usage"]}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();
        match msg {
            WsMessage::Subscribe { metrics } => {
                assert_eq!(metrics.len(), 2);
                assert_eq!(metrics[0], "cpu_usage");
            }
            _ => panic!("Expected Subscribe message"),
        }
    }

    #[test]
    fn test_ws_message_unsubscribe() {
        let json = r#"{"type":"unsubscribe","metrics":["cpu_usage"]}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();
        match msg {
            WsMessage::Unsubscribe { metrics } => {
                assert_eq!(metrics.len(), 1);
                assert_eq!(metrics[0], "cpu_usage");
            }
            _ => panic!("Expected Unsubscribe message"),
        }
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_ws_message_ping_pong() {
        let ping_json = r#"{"type":"ping"}"#;
        let ping: WsMessage = serde_json::from_str(ping_json).unwrap();
        assert!(matches!(ping, WsMessage::Ping));

        let pong_json = r#"{"type":"pong"}"#;
        let pong: WsMessage = serde_json::from_str(pong_json).unwrap();
        assert!(matches!(pong, WsMessage::Pong));
    }
}
