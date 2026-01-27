//! OpenAI Realtime API WebSocket client
//!
//! Manages the WebSocket connection lifecycle for streaming transcription.
//!
//! # Connection Flow
//!
//! 1. `connect()` - Establish WebSocket, receive `session.created`, send config
//! 2. `send_audio()` - Stream audio chunks (non-blocking)
//! 3. `receive()` - Get incoming messages (transcripts, errors)
//! 4. `disconnect()` - Clean shutdown
//!
//! # Retry Strategy
//!
//! Initial connection retries 3 times with exponential backoff (1s, 2s, 4s).
//! Mid-session disconnects do NOT reconnect - fall back to batch transcription.

use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{
        client::IntoClientRequest,
        http::{HeaderValue, Request},
        Message,
    },
    MaybeTlsStream, WebSocketStream,
};

use super::protocol::{ClientMessage, ServerMessage, REALTIME_API_URL};
use super::StreamingError;

/// Connection timeout for initial WebSocket handshake
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Timeout for waiting for session.created message
const SESSION_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum retry attempts for initial connection
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (doubles each retry)
const RETRY_BASE_DELAY: Duration = Duration::from_secs(1);

/// Handle to an active Realtime API session
///
/// The session owns the WebSocket connection and provides methods for
/// sending audio and receiving transcripts.
pub struct RealtimeSession {
    /// WebSocket write half for sending messages
    write: futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    /// Channel receiver for incoming messages (processed by background task)
    /// Wrapped in Option so it can be taken for concurrent processing
    incoming_rx: Option<mpsc::Receiver<ServerMessage>>,
    /// Session ID from OpenAI
    session_id: String,
    /// Handle to the receiver task (for cleanup on disconnect/drop)
    receiver_task: tokio::task::JoinHandle<()>,
}

impl RealtimeSession {
    /// Connect to the OpenAI Realtime API
    ///
    /// This method:
    /// 1. Establishes a WebSocket connection (with retries)
    /// 2. Waits for `session.created` message
    /// 3. Sends session configuration
    /// 4. Waits for `session.updated` confirmation
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key for authentication
    ///
    /// # Returns
    /// * `Ok(RealtimeSession)` - Connected and configured session
    /// * `Err(StreamingError)` - Connection or authentication failed
    pub async fn connect(api_key: &str) -> Result<Self, StreamingError> {
        // Retry connection with exponential backoff
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = RETRY_BASE_DELAY * 2u32.pow(attempt - 1);
                log::info!(
                    "Retrying WebSocket connection in {:?} (attempt {}/{})",
                    delay,
                    attempt + 1,
                    MAX_RETRIES
                );
                tokio::time::sleep(delay).await;
            }

            match Self::try_connect(api_key).await {
                Ok(session) => return Ok(session),
                Err(e) => {
                    log::warn!("Connection attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            StreamingError::ConnectionFailed("Max retries exceeded".to_string())
        }))
    }

    /// Single connection attempt (no retries)
    async fn try_connect(api_key: &str) -> Result<Self, StreamingError> {
        // Build WebSocket request with auth header
        let mut request = REALTIME_API_URL
            .into_client_request()
            .map_err(|e| StreamingError::ConnectionFailed(e.to_string()))?;

        request.headers_mut().insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|e| StreamingError::AuthenticationFailed(e.to_string()))?,
        );

        request
            .headers_mut()
            .insert("OpenAI-Beta", HeaderValue::from_static("realtime=v1"));

        log::info!("Connecting to OpenAI Realtime API...");

        // Connect with timeout
        let (ws_stream, _response) = timeout(
            CONNECTION_TIMEOUT,
            connect_async_with_config(
                request, None, false, // disable_nagle (we want low latency)
            ),
        )
        .await
        .map_err(|_| StreamingError::ConnectionFailed("Connection timeout".to_string()))?
        .map_err(|e| StreamingError::ConnectionFailed(e.to_string()))?;

        log::info!("WebSocket connected, waiting for session.created...");

        // Split into read/write halves
        let (write, mut read) = ws_stream.split();

        // Wait for session.created message
        let session_id = timeout(SESSION_TIMEOUT, async {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => match serde_json::from_str::<ServerMessage>(&text) {
                        Ok(ServerMessage::SessionCreated { session }) => {
                            log::info!("Session created: {}", session.id);
                            return Ok(session.id);
                        }
                        Ok(ServerMessage::Error { error }) => {
                            return Err(StreamingError::AuthenticationFailed(error.message));
                        }
                        Ok(_) => {
                            log::debug!("Ignoring message while waiting for session.created");
                        }
                        Err(e) => {
                            log::warn!("Failed to parse message: {}", e);
                        }
                    },
                    Ok(Message::Close(_)) => {
                        return Err(StreamingError::Disconnected(
                            "Connection closed before session created".to_string(),
                        ));
                    }
                    Err(e) => {
                        return Err(StreamingError::ProtocolError(e.to_string()));
                    }
                    _ => {} // Ignore ping/pong/binary
                }
            }
            Err(StreamingError::Disconnected("Stream ended".to_string()))
        })
        .await
        .map_err(|_| StreamingError::ConnectionFailed("Session creation timeout".to_string()))??;

        // Create channel for incoming messages
        let (incoming_tx, incoming_rx) = mpsc::channel(100);

        // Spawn background task to receive messages
        let receiver_task = tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => match serde_json::from_str::<ServerMessage>(&text) {
                        Ok(msg) => {
                            if incoming_tx.send(msg).await.is_err() {
                                log::debug!("Receiver channel closed");
                                break;
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to parse message: {}", e);
                        }
                    },
                    Ok(Message::Close(_)) => {
                        log::info!("WebSocket closed by server");
                        break;
                    }
                    Err(e) => {
                        log::warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {} // Ignore ping/pong/binary
                }
            }
            log::debug!("Receiver task exiting");
        });

        let mut session = Self {
            write,
            incoming_rx: Some(incoming_rx),
            session_id,
            receiver_task,
        };

        // Send session configuration
        session.configure_session().await?;

        Ok(session)
    }

    /// Send session configuration for transcription-only mode
    async fn configure_session(&mut self) -> Result<(), StreamingError> {
        log::info!("Configuring session for transcription...");

        let config_msg = ClientMessage::session_update();
        self.send_message(&config_msg).await?;

        // Get a mutable reference to the receiver (should always be present during config)
        let incoming_rx = self.incoming_rx.as_mut().ok_or_else(|| {
            StreamingError::ProtocolError("Incoming receiver already taken".to_string())
        })?;

        // Wait for session.updated confirmation
        let deadline = tokio::time::Instant::now() + SESSION_TIMEOUT;

        while tokio::time::Instant::now() < deadline {
            match timeout(deadline - tokio::time::Instant::now(), incoming_rx.recv()).await {
                Ok(Some(ServerMessage::SessionUpdated { session })) => {
                    log::info!("Session configured: {:?}", session.modalities);
                    return Ok(());
                }
                Ok(Some(ServerMessage::Error { error })) => {
                    return Err(StreamingError::ProtocolError(error.message));
                }
                Ok(Some(_)) => {
                    // Other message, keep waiting
                }
                Ok(None) => {
                    return Err(StreamingError::Disconnected(
                        "Channel closed during configuration".to_string(),
                    ));
                }
                Err(_) => {
                    return Err(StreamingError::ConnectionFailed(
                        "Session configuration timeout".to_string(),
                    ));
                }
            }
        }

        Err(StreamingError::ConnectionFailed(
            "Session configuration timeout".to_string(),
        ))
    }

    /// Send a client message over the WebSocket
    async fn send_message(&mut self, msg: &ClientMessage) -> Result<(), StreamingError> {
        let json =
            serde_json::to_string(msg).map_err(|e| StreamingError::ProtocolError(e.to_string()))?;

        self.write
            .send(Message::Text(json))
            .await
            .map_err(|e| StreamingError::SendFailed(e.to_string()))?;

        Ok(())
    }

    /// Send audio samples to the Realtime API
    ///
    /// Samples should be PCM16 mono at 24kHz.
    /// This method is async but designed to be fast - it just queues the send.
    pub async fn send_audio(&mut self, samples: &[i16]) -> Result<(), StreamingError> {
        let msg = ClientMessage::audio_append(samples);
        self.send_message(&msg).await
    }

    /// Commit the audio buffer, signaling end of input
    ///
    /// Call this when the user stops recording to trigger final transcription.
    pub async fn commit_audio(&mut self) -> Result<(), StreamingError> {
        let msg = ClientMessage::audio_commit();
        self.send_message(&msg).await
    }

    /// Clear the audio buffer without committing
    pub async fn clear_audio(&mut self) -> Result<(), StreamingError> {
        let msg = ClientMessage::audio_clear();
        self.send_message(&msg).await
    }

    /// Try to receive the next message (non-blocking)
    ///
    /// Returns `None` if no message is available or if receiver was taken.
    pub fn try_recv(&mut self) -> Option<ServerMessage> {
        self.incoming_rx.as_mut()?.try_recv().ok()
    }

    /// Receive the next message (blocking)
    ///
    /// Returns `None` if the connection is closed or if receiver was taken.
    pub async fn recv(&mut self) -> Option<ServerMessage> {
        match self.incoming_rx.as_mut() {
            Some(rx) => rx.recv().await,
            None => None,
        }
    }

    /// Take ownership of the incoming message receiver
    ///
    /// This allows concurrent processing of incoming messages (transcripts)
    /// while the session is being used for sending audio.
    ///
    /// After calling this, `recv()` and `try_recv()` will return `None`.
    ///
    /// # Returns
    /// The incoming message receiver, or `None` if already taken.
    pub fn take_incoming_receiver(&mut self) -> Option<mpsc::Receiver<ServerMessage>> {
        self.incoming_rx.take()
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Gracefully disconnect from the API
    ///
    /// This closes the WebSocket connection cleanly and aborts the receiver task.
    pub async fn disconnect(mut self) {
        log::info!("Disconnecting from Realtime API...");

        // Abort the receiver task to ensure clean shutdown
        self.receiver_task.abort();

        // Send close frame
        if let Err(e) = self.write.close().await {
            log::warn!("Error closing WebSocket: {}", e);
        }
    }
}

impl Drop for RealtimeSession {
    fn drop(&mut self) {
        // Ensure receiver task is aborted if session is dropped without disconnect()
        self.receiver_task.abort();
    }
}

/// Get the OpenAI API key from environment
pub fn get_api_key() -> Option<String> {
    std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_api_key_missing() {
        // This test depends on environment, but we can at least verify it doesn't panic
        let _ = get_api_key();
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_realtime_connection() {
        let api_key = get_api_key().expect("OPENAI_API_KEY required");

        let session = RealtimeSession::connect(&api_key).await;
        assert!(session.is_ok(), "Connection failed: {:?}", session.err());

        let session = session.unwrap();
        assert!(!session.session_id().is_empty());

        session.disconnect().await;
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_send_audio() {
        let api_key = get_api_key().expect("OPENAI_API_KEY required");

        let mut session = RealtimeSession::connect(&api_key)
            .await
            .expect("Connection failed");

        // Send some silence
        let silence = vec![0i16; 2400]; // 100ms at 24kHz
        let result = session.send_audio(&silence).await;
        assert!(result.is_ok(), "Send failed: {:?}", result.err());

        session.disconnect().await;
    }
}
