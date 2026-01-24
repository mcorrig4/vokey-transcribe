# Test Fixtures

This directory contains audio files for transcription integration tests.

## Required Files

Add the following WAV files to run the full test suite:

| File | Description | Duration | Purpose |
|------|-------------|----------|---------|
| `short_speech.wav` | Clear speech saying something recognizable | 2-3 sec | Basic transcription test |
| `silence.wav` | Silent/near-silent audio | 1-2 sec | Empty audio handling |
| `very_short.wav` | Very brief audio | <0.5 sec | Edge case for minimum duration |

## WAV Format Requirements

- **Format**: PCM WAV (uncompressed)
- **Sample Rate**: 16000 Hz recommended (Whisper's native rate), but 44100/48000 also work
- **Channels**: Mono preferred, stereo accepted
- **Bit Depth**: 16-bit

## Creating Test Files

### Using FFmpeg

```bash
# Convert any audio to correct format
ffmpeg -i input.mp3 -ar 16000 -ac 1 -acodec pcm_s16le short_speech.wav

# Create silence file (2 seconds)
ffmpeg -f lavfi -i anullsrc=r=16000:cl=mono -t 2 -acodec pcm_s16le silence.wav

# Trim to very short
ffmpeg -i input.wav -t 0.3 -acodec pcm_s16le very_short.wav
```

### Using Audacity

1. Record or import audio
2. Export as WAV (Microsoft) signed 16-bit PCM
3. Set sample rate to 16000 Hz

## Environment Setup

To run integration tests that call the real OpenAI API:

```bash
export OPENAI_API_KEY=sk-your-key-here
cd src-tauri
cargo test --test transcription_integration
```

## Test Categories

### Mock Tests (no API key needed)
- File read error handling
- Missing API key detection
- Error type formatting

### Integration Tests (API key + fixtures required)
- Real transcription with `short_speech.wav`
- Empty/silent audio handling
- Very short audio edge case
- API error response handling

## Security Note

Never commit API keys. The `.env` file is gitignored.
