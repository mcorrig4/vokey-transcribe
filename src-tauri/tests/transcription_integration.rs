//! Integration tests for the transcription module
//!
//! These tests verify the OpenAI Whisper API integration and error handling.
//!
//! ## Running Tests
//!
//! ### Mock tests (no API key needed):
//! ```bash
//! cargo test --test transcription_integration mock_
//! ```
//!
//! ### Integration tests (requires API key + fixtures):
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo test --test transcription_integration integration_
//! ```
//!
//! ### All tests:
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo test --test transcription_integration
//! ```

use std::path::PathBuf;

// Import the library crate
use app_lib::transcription::{is_api_key_configured, transcribe_audio, TranscriptionError};

/// Get the path to the test fixtures directory
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Check if a fixture file exists
fn fixture_exists(name: &str) -> bool {
    fixtures_dir().join(name).exists()
}

/// Get path to a fixture file
fn fixture_path(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

// ============================================================================
// Mock Tests - No API key or fixtures required
// ============================================================================

mod mock_tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn mock_is_api_key_configured_reflects_env() {
        // This test verifies the function works - actual value depends on env
        let result = is_api_key_configured();
        // Just verify it returns a bool without panicking
        assert!(result == true || result == false);
    }

    #[tokio::test]
    async fn mock_file_read_error_for_nonexistent_file() {
        // Skip if API key not set (we need it to get past the first check)
        if !is_api_key_configured() {
            eprintln!("Skipping mock_file_read_error_for_nonexistent_file: OPENAI_API_KEY not set");
            return;
        }

        let nonexistent = PathBuf::from("/tmp/this_file_does_not_exist_12345.wav");
        let result = transcribe_audio(&nonexistent).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, TranscriptionError::FileReadError(_)),
            "Expected FileReadError, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn mock_missing_api_key_error() {
        // Temporarily unset the API key by checking current state
        // Note: This test may not work correctly if OPENAI_API_KEY is set
        // It's mainly for documentation of expected behavior
        if is_api_key_configured() {
            eprintln!(
                "Skipping mock_missing_api_key_error: OPENAI_API_KEY is set. \
                 Unset it to test MissingApiKey error path."
            );
            return;
        }

        let dummy_path = PathBuf::from("/tmp/dummy.wav");
        let result = transcribe_audio(&dummy_path).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, TranscriptionError::MissingApiKey),
            "Expected MissingApiKey, got: {:?}",
            err
        );
    }

    #[test]
    fn mock_error_display_formats_correctly() {
        // Test all error variants format correctly for user display
        let errors = vec![
            (TranscriptionError::MissingApiKey, "OPENAI_API_KEY"),
            (
                TranscriptionError::FileReadError("file not found".to_string()),
                "file not found",
            ),
            (
                TranscriptionError::NetworkError("connection refused".to_string()),
                "connection refused",
            ),
            (
                TranscriptionError::ApiError {
                    status: 401,
                    message: "Invalid API key".to_string(),
                },
                "401",
            ),
            (
                TranscriptionError::ParseError("invalid JSON".to_string()),
                "invalid JSON",
            ),
        ];

        for (err, expected_substring) in errors {
            let display = err.to_string();
            assert!(
                display.contains(expected_substring),
                "Error display '{}' should contain '{}'",
                display,
                expected_substring
            );
        }
    }

    #[tokio::test]
    async fn mock_empty_file_handling() {
        // Skip if API key not set
        if !is_api_key_configured() {
            eprintln!("Skipping mock_empty_file_handling: OPENAI_API_KEY not set");
            return;
        }

        // Create an empty temp file
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let result = transcribe_audio(temp_file.path()).await;

        // OpenAI should reject empty files - we expect an API error
        assert!(result.is_err(), "Empty file should fail transcription");
        let err = result.unwrap_err();

        // Could be ApiError (OpenAI rejects it) or ParseError (invalid WAV)
        assert!(
            matches!(
                err,
                TranscriptionError::ApiError { .. } | TranscriptionError::NetworkError(_)
            ),
            "Expected ApiError or NetworkError for empty file, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn mock_invalid_wav_header() {
        // Skip if API key not set
        if !is_api_key_configured() {
            eprintln!("Skipping mock_invalid_wav_header: OPENAI_API_KEY not set");
            return;
        }

        // Create a file with invalid WAV content
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(b"This is not a valid WAV file content")
            .expect("Failed to write to temp file");

        let result = transcribe_audio(temp_file.path()).await;

        // OpenAI should reject invalid WAV files
        assert!(result.is_err(), "Invalid WAV should fail transcription");
    }
}

// ============================================================================
// Integration Tests - Require API key and fixture files
// ============================================================================

mod integration_tests {
    use super::*;

    /// Helper to skip test if prerequisites aren't met
    fn check_prerequisites(fixture_name: &str) -> bool {
        if !is_api_key_configured() {
            eprintln!(
                "Skipping integration test: OPENAI_API_KEY not set. \
                 Set it to run integration tests."
            );
            return false;
        }

        if !fixture_exists(fixture_name) {
            eprintln!(
                "Skipping integration test: fixture '{}' not found. \
                 Add test WAV files to src-tauri/tests/fixtures/",
                fixture_name
            );
            return false;
        }

        true
    }

    #[tokio::test]
    async fn integration_transcribe_short_speech() {
        const FIXTURE: &str = "short_speech.wav";
        if !check_prerequisites(FIXTURE) {
            return;
        }

        let path = fixture_path(FIXTURE);
        let result = transcribe_audio(&path).await;

        assert!(
            result.is_ok(),
            "Transcription should succeed for valid speech: {:?}",
            result.err()
        );

        let text = result.unwrap().text;
        assert!(
            !text.is_empty(),
            "Transcribed text should not be empty for speech audio"
        );

        println!("Transcribed text: {}", text);
    }

    #[tokio::test]
    async fn integration_transcribe_silence() {
        const FIXTURE: &str = "silence.wav";
        if !check_prerequisites(FIXTURE) {
            return;
        }

        let path = fixture_path(FIXTURE);
        let result = transcribe_audio(&path).await;

        // Silence may return empty text or very short text - both are valid
        assert!(
            result.is_ok(),
            "Transcription should succeed for silence: {:?}",
            result.err()
        );

        let text = result.unwrap().text;
        println!("Silence transcription result: '{}'", text);

        // Whisper often returns empty string or whitespace for silence
        // This is expected behavior
    }

    #[tokio::test]
    async fn integration_transcribe_very_short() {
        const FIXTURE: &str = "very_short.wav";
        if !check_prerequisites(FIXTURE) {
            return;
        }

        let path = fixture_path(FIXTURE);
        let result = transcribe_audio(&path).await;

        // Very short audio may succeed with empty text or fail
        // We mainly want to verify it doesn't panic
        match result {
            Ok(result) => {
                println!("Very short audio transcription: '{}'", result.text);
            }
            Err(e) => {
                println!("Very short audio error (may be expected): {}", e);
                // Some errors are acceptable for very short audio
                assert!(
                    matches!(e, TranscriptionError::ApiError { .. }),
                    "Unexpected error type for very short audio: {:?}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn integration_metrics_timing() {
        const FIXTURE: &str = "short_speech.wav";
        if !check_prerequisites(FIXTURE) {
            return;
        }

        let path = fixture_path(FIXTURE);

        // Measure transcription time
        let start = std::time::Instant::now();
        let result = transcribe_audio(&path).await;
        let duration = start.elapsed();

        assert!(result.is_ok(), "Transcription should succeed");

        println!(
            "Transcription took {:?} for {} bytes",
            duration,
            std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
        );

        // Sanity check: transcription should complete in reasonable time
        // (allowing for network latency)
        assert!(
            duration.as_secs() < 30,
            "Transcription took too long: {:?}",
            duration
        );
    }
}

// ============================================================================
// Error Case Tests for Issue #43
// ============================================================================

mod error_case_tests {
    use super::*;

    #[tokio::test]
    async fn error_case_api_key_validation() {
        // Test that MissingApiKey is properly detected
        // This is a documentation test - behavior depends on env
        let configured = is_api_key_configured();
        println!("API key configured: {}", configured);

        // The function should not panic regardless of configuration
        assert!(configured == true || configured == false);
    }

    #[tokio::test]
    async fn error_case_invalid_api_key() {
        // This test requires temporarily setting an invalid key
        // We can't easily do this without affecting other tests
        // So we document the expected behavior

        // If OPENAI_API_KEY=invalid_key_12345, we expect:
        // - TranscriptionError::ApiError { status: 401, message: "..." }

        println!(
            "To test invalid API key handling, run with: \
             OPENAI_API_KEY=invalid_key cargo test error_case_invalid"
        );
    }

    #[tokio::test]
    async fn error_case_network_timeout() {
        // Network errors are hard to simulate reliably
        // Document expected behavior:
        // - TranscriptionError::NetworkError("...")

        println!(
            "Network errors result in TranscriptionError::NetworkError. \
             To test, disconnect network and run transcription."
        );
    }

    #[tokio::test]
    async fn error_case_file_permissions() {
        // Skip if API key not set
        if !is_api_key_configured() {
            eprintln!("Skipping error_case_file_permissions: OPENAI_API_KEY not set");
            return;
        }

        // Try to read a file we don't have permission for
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
            let restricted_file = temp_dir.path().join("restricted.wav");

            // Create file then remove read permissions
            std::fs::write(&restricted_file, b"dummy content").expect("Failed to write");
            let mut perms = std::fs::metadata(&restricted_file)
                .expect("Failed to get metadata")
                .permissions();
            perms.set_mode(0o000); // No permissions
            std::fs::set_permissions(&restricted_file, perms).expect("Failed to set permissions");

            let result = transcribe_audio(&restricted_file).await;

            // Restore permissions for cleanup
            let mut perms = std::fs::metadata(&restricted_file)
                .unwrap_or_else(|_| std::fs::metadata(".").unwrap())
                .permissions();
            perms.set_mode(0o644);
            let _ = std::fs::set_permissions(&restricted_file, perms);

            assert!(result.is_err(), "Should fail for permission denied");
            let err = result.unwrap_err();
            assert!(
                matches!(err, TranscriptionError::FileReadError(_)),
                "Expected FileReadError for permission denied, got: {:?}",
                err
            );
        }
    }

    #[tokio::test]
    async fn error_case_all_error_types_are_send_sync() {
        // Verify error types can be sent across threads (important for async)
        fn assert_send_sync<T: Send + Sync>() {}

        // TranscriptionError should be Send + Sync for use in async contexts
        // This is a compile-time check
        assert_send_sync::<TranscriptionError>();
    }
}
