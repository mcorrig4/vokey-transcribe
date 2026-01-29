# E2E Testing Research for VoKey Transcribe

This document summarizes research into E2E testing approaches for our Tauri 2 + React transcription application, with a focus on UI automation and audio mocking.

---

## Executive Summary

For VoKey Transcribe, I recommend a **multi-layered testing strategy**:

| Layer | Tool | Purpose |
|-------|------|---------|
| **Unit Tests** | Vitest + `@tauri-apps/api/mocks` | Test React components with mocked IPC |
| **Component Integration** | Vitest + React Testing Library | Test UI flows without Tauri backend |
| **E2E (WebDriver)** | WebdriverIO + tauri-driver | Full app testing with real IPC |
| **E2E (Vision AI)** | TestDriver.ai | Selectorless UI testing via screenshots |
| **Audio Mocking** | Chromium fake audio + PipeWire | Inject WAV files as microphone input |

---

## Approach 1: Vitest + Tauri Mocks (Recommended for Unit/Integration)

### Overview
Use Vitest to test React components with mocked Tauri IPC calls. This is the fastest and most reliable approach for testing UI logic.

### Setup

```bash
pnpm add -D vitest @vitest/ui jsdom @testing-library/react @testing-library/jest-dom happy-dom
```

### Configuration

```typescript
// vitest.config.ts
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import { resolve } from 'path'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{js,ts,jsx,tsx}'],
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
  },
})
```

### Mocking Tauri IPC

```typescript
// src/test/setup.ts
import { beforeAll, afterEach } from 'vitest'
import { randomFillSync } from 'crypto'
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks'

// Polyfill crypto for jsdom
beforeAll(() => {
  Object.defineProperty(window, 'crypto', {
    value: {
      getRandomValues: (buffer: Uint8Array) => randomFillSync(buffer),
    },
  })
})

afterEach(() => {
  clearMocks()
})
```

### Example Test

```typescript
// src/components/HUD.test.tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { mockIPC } from '@tauri-apps/api/mocks'
import App from '../App'

describe('HUD Component', () => {
  it('displays recording state correctly', async () => {
    // Mock IPC calls
    // Note: UiState fields match src/types.ts - use 'status' not 'state', 'elapsedSecs' not 'elapsed_ms'
    mockIPC((cmd, args) => {
      switch (cmd) {
        case 'get_ui_state':
          return { status: 'recording', elapsedSecs: 5 }
        case 'get_metrics_summary':
          return { total_cycles: 10, success_rate: 0.95 }
        default:
          return null
      }
    })

    render(<App />)

    // Test that recording state is displayed
    expect(screen.getByText(/Recording/i)).toBeInTheDocument()
  })
})
```

### Pros & Cons

| Pros | Cons |
|------|------|
| Fast execution (no app startup) | Doesn't test real Tauri IPC |
| Easy to mock any backend response | Can't test Rust-side logic |
| Great for UI logic testing | Mocks can drift from reality |
| Works in CI without display | Limited to frontend testing |

### Sources
- [Tauri Mocking Documentation](https://v2.tauri.app/develop/tests/mocking/)
- [Vitest with Tauri Tutorial](https://yonatankra.com/how-to-setup-vitest-in-a-tauri-project/)

---

## Approach 2: WebdriverIO + tauri-driver (Recommended for Full E2E)

### Overview
Tauri's official E2E testing approach uses WebDriver protocol via `tauri-driver`. This tests the real application with the Rust backend running.

### Prerequisites (Linux)

```bash
# Install WebKitWebDriver (required for Linux)
sudo apt-get install webkit2gtk-driver

# Verify installation
which WebKitWebDriver
# Should output: /usr/bin/WebKitWebDriver

# Install tauri-driver
cargo install tauri-driver
```

### Setup

```bash
pnpm add -D @wdio/cli @wdio/local-runner @wdio/mocha-framework @wdio/spec-reporter
pnpm wdio config  # Interactive setup
```

### Configuration

```javascript
// wdio.conf.js
const path = require('path')

exports.config = {
  specs: ['./test/specs/**/*.js'],
  maxInstances: 1,
  capabilities: [{
    'tauri:options': {
      application: path.resolve('./src-tauri/target/release/vokey-transcribe'),
    },
  }],
  services: [
    ['tauri', {
      tauriDriver: 'tauri-driver',
    }],
  ],
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },
}
```

### Example Test

```javascript
// test/specs/recording.e2e.js
describe('VoKey Transcribe Recording Flow', () => {
  it('should show Ready state initially', async () => {
    const hud = await $('[data-testid="hud-status"]')
    await expect(hud).toHaveTextContaining('Ready')
  })

  it('should start recording when hotkey pressed', async () => {
    // Simulate hotkey via debug panel command
    await browser.execute(() => {
      window.__TAURI_INVOKE__('simulate_record_start')
    })

    const hud = await $('[data-testid="hud-status"]')
    await expect(hud).toHaveTextContaining('Recording')
  })

  it('should complete transcription flow', async () => {
    // Start recording
    await browser.execute(() => {
      window.__TAURI_INVOKE__('simulate_record_start')
    })

    // Wait for recording
    await browser.pause(2000)

    // Stop recording
    await browser.execute(() => {
      window.__TAURI_INVOKE__('simulate_record_stop')
    })

    // Verify transcription happens
    const hud = await $('[data-testid="hud-status"]')
    await hud.waitForExist({ timeout: 10000 })
    await expect(hud).toHaveTextContaining('Done')
  })
})
```

### Platform Support

| Platform | Support | WebDriver Server |
|----------|---------|------------------|
| Linux | ✅ Full | WebKitWebDriver |
| Windows | ✅ Full | Microsoft Edge Driver |
| macOS | ❌ None | No WKWebView driver available |

### Pros & Cons

| Pros | Cons |
|------|------|
| Tests real application | Slower than unit tests |
| Full Tauri IPC testing | Requires built app binary |
| Official Tauri support | No macOS support |
| Standard WebDriver APIs | Setup complexity |

### Sources
- [Tauri WebDriver Documentation](https://v2.tauri.app/develop/tests/webdriver/)
- [Tauri WebdriverIO Example](https://v2.tauri.app/develop/tests/webdriver/example/webdriverio/)
- [tauri-apps/webdriver-example](https://github.com/tauri-apps/webdriver-example)

---

## Approach 3: TestDriver.ai (Vision-Based AI Testing)

### Overview
TestDriver.ai uses AI vision to interact with applications, eliminating brittle selectors. It integrates with Playwright and can test any desktop application.

### How It Works
1. Captures screenshots of your application
2. Uses computer vision to identify UI elements
3. Generates actions based on natural language prompts
4. Self-heals when UI changes

### Setup

```bash
pnpm add -D @testdriver.ai/playwright
```

### Configuration

```typescript
// playwright.config.ts
import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  use: {
    baseURL: 'http://localhost:1420',
  },
  webServer: {
    command: 'pnpm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
  },
})
```

### Example Test

```typescript
// e2e/transcription.spec.ts
import { test, expect } from '@testdriver.ai/playwright'

test('complete transcription flow', async ({ desktop }) => {
  // Use natural language instead of selectors
  await desktop.ai('click the settings icon in the system tray')
  await desktop.ai('find the debug panel')
  await desktop.ai('click the simulate record button')

  // Wait for visual confirmation
  await desktop.ai('wait until you see "Recording" on screen')

  // Continue flow
  await desktop.ai('click stop recording button')
  await desktop.ai('verify the status shows "Done"')
})
```

### Pros & Cons

| Pros | Cons |
|------|------|
| No selectors to maintain | Requires API key (paid service) |
| Handles UI changes gracefully | Slower than selector-based tests |
| Natural language test definitions | AI can misinterpret complex UIs |
| Tests exactly what users see | Black box testing only |

### Demo Repository
- [testdriverai/demo-tauri-app](https://github.com/testdriverai/demo-tauri-app)

### Sources
- [TestDriver.ai Tauri Guide](https://docs.testdriver.ai/v6/apps/tauri-apps)
- [TestDriver.ai Overview](https://testdriver.ai/)

---

## Approach 4: Audio Mocking for Transcription Testing

### Overview
To test the transcription pipeline end-to-end, we need to inject pre-recorded audio files as microphone input. There are two main approaches:

### Option A: Chromium Fake Audio Capture (WebView Only)

Chromium-based browsers support injecting WAV files as fake microphone input. However, this **only works for web-based audio capture** (getUserMedia), not for CPAL-based native audio capture.

```typescript
// Only works if using Web Audio API in frontend
const browser = await chromium.launch({
  args: [
    '--use-fake-device-for-media-stream',
    '--use-fake-ui-for-media-stream',
    '--use-file-for-fake-audio-capture=/path/to/test.wav',
  ],
})
```

**Audio File Requirements:**
- Format: WAV only
- Sample rate: 48 kHz recommended
- Channels: 1 (mono)
- Bit depth: 16-bit PCM
- To disable looping: append `%noloop` to the path

**Limitation:** VoKey uses CPAL for native audio capture, so this approach won't work for our main flow.

### Option B: PipeWire/PulseAudio Virtual Microphone (Recommended)

For native audio capture with CPAL, we need to create a virtual microphone at the OS level and inject audio into it.

#### Setup (One-time)

```bash
# Create a virtual microphone source
pactl load-module module-null-sink sink_name=VirtualMic sink_properties=device.description=VirtualMic

# Create a virtual source from the sink's monitor
pactl load-module module-virtual-source source_name=VirtualMicSource master=VirtualMic.monitor source_properties=device.description=VirtualMicSource

# Verify it exists
pactl list sources short | grep VirtualMic
```

#### Test Audio Injection

```bash
# Play a WAV file into the virtual microphone
pw-play --target=VirtualMic /path/to/test-audio.wav

# Or with PulseAudio
paplay --device=VirtualMic /path/to/test-audio.wav
```

#### Automation Script

```bash
#!/bin/bash
# scripts/inject-test-audio.sh

VIRTUAL_MIC="VirtualMic"
AUDIO_FILE="${1:-test-data/hello-world.wav}"

# Ensure virtual mic exists
if ! pactl list sources short | grep -q "$VIRTUAL_MIC"; then
    pactl load-module module-null-sink sink_name=$VIRTUAL_MIC
    pactl load-module module-virtual-source source_name=${VIRTUAL_MIC}Source master=$VIRTUAL_MIC.monitor
fi

# Play audio file into virtual mic
pw-play --target=$VIRTUAL_MIC "$AUDIO_FILE" &
PID=$!

echo "Injecting audio (PID: $PID)..."
wait $PID
echo "Audio injection complete"
```

#### Integration with E2E Tests

```typescript
// e2e/transcription-with-audio.spec.ts
import { execSync } from 'child_process'

describe('Transcription E2E with Audio', () => {
  const VIRTUAL_MIC = 'VirtualMic'

  beforeAll(() => {
    // Setup virtual microphone
    try {
      execSync(`pactl load-module module-null-sink sink_name=${VIRTUAL_MIC}`)
      execSync(`pactl load-module module-virtual-source source_name=${VIRTUAL_MIC}Source master=${VIRTUAL_MIC}.monitor`)
    } catch (e) {
      // Modules may already be loaded
    }

    // Configure CPAL to use virtual mic (via env var or config)
    process.env.VOKEY_AUDIO_DEVICE = `${VIRTUAL_MIC}Source`
  })

  it('should transcribe "hello world" correctly', async () => {
    // Start recording
    await browser.execute(() => window.__TAURI_INVOKE__('simulate_record_start'))

    // Inject audio file
    execSync(`pw-play --target=${VIRTUAL_MIC} test-data/hello-world.wav`)

    // Wait for audio to finish + small buffer
    await browser.pause(2000)

    // Stop recording
    await browser.execute(() => window.__TAURI_INVOKE__('simulate_record_stop'))

    // Wait for transcription to complete
    // Note: get_transcription_status returns API config (api_key_configured, api_provider),
    // not transcript text. The transcript is available via the 'done' UI state or clipboard.
    await browser.pause(5000)

    // Option 1: Check UI state for 'done' status with text
    const uiState = await browser.execute(() =>
      window.__TAURI_INVOKE__('get_ui_state')
    )
    expect(uiState.status).toBe('done')
    expect(uiState.text.toLowerCase()).toContain('hello world')

    // Option 2: Read clipboard (transcript is auto-copied)
    // const clipboard = await browser.execute(() => navigator.clipboard.readText())
    // expect(clipboard.toLowerCase()).toContain('hello world')
  })
})
```

### Test Audio File Recommendations

Create a set of standard test audio files:

| File | Content | Duration | Purpose |
|------|---------|----------|---------|
| `hello-world.wav` | "Hello world" | 1-2s | Basic smoke test |
| `silence.wav` | Pure silence | 3s | Test NoSpeech detection |
| `short-utterance.wav` | Brief word | 0.3s | Test minimum duration |
| `long-sentence.wav` | Full paragraph | 15s | Test longer recordings |
| `numbers.wav` | "One two three four five" | 3s | Test numeric transcription |
| `coding-terms.wav` | "create function get user data" | 3s | Test Coding mode |
| `noisy-speech.wav` | Speech with background noise | 3s | Test robustness |

### Recording Test Audio Files

```bash
# Record a test file with arecord
arecord -f cd -d 3 -t wav hello-world.wav

# Or use a TTS engine for consistent test data
espeak-ng -w hello-world.wav "Hello world"
piper --model en_US-lessac-medium --output_file hello-world.wav "Hello world"
```

### Sources
- [PipeWire ArchWiki](https://wiki.archlinux.org/title/PipeWire)
- [Ubuntu Audio Loopback](https://wiki.ubuntu.com/record_system_sound)
- [Testing Speech Recognition with Playwright](https://dkarlovi.github.io/testing-speech-recognition/)
- [Playwright Fake Audio Setup](https://substack.com/home/post/p-126438141)
- [Chromium Media Testing Flags](https://webrtc.org/getting-started/testing)

---

## Recommended Implementation Plan

### Phase 1: Unit/Component Tests (Low Effort, High Value)

1. **Install Vitest + Testing Library**
   ```bash
   pnpm add -D vitest @vitest/ui jsdom @testing-library/react @testing-library/jest-dom
   ```

2. **Configure Vitest** with Tauri mock setup

3. **Write component tests** for:
   - HUD state display
   - Settings form validation
   - Usage metrics display
   - Error state handling

4. **Add to CI** with GitHub Actions

### Phase 2: Tauri IPC Integration Tests (Medium Effort)

1. **Create mock IPC handlers** that simulate real backend responses

2. **Test complex flows**:
   - Recording → Transcription → Clipboard
   - Error recovery
   - Settings persistence

3. **Add spy tracking** for IPC call verification

### Phase 3: Full E2E with WebdriverIO (Higher Effort)

1. **Install WebdriverIO + tauri-driver**

2. **Configure for Linux** (WebKitWebDriver)

3. **Write E2E tests** for critical paths:
   - App launch → Ready state
   - Tray menu interactions
   - Settings window navigation
   - Full recording flow (with simulated commands)

4. **Set up CI runner** with display (Xvfb or real)

### Phase 4: Audio Mocking (Highest Effort, Highest Value)

1. **Create test audio files**:
   - Use TTS (piper/espeak-ng) for consistent, reproducible audio
   - Cover various scenarios (silence, short, long, noisy)

2. **Set up PipeWire virtual microphone** in CI

3. **Modify CPAL config** to accept configurable audio device

4. **Write transcription E2E tests** with injected audio

5. **Add OpenAI API mocking** for deterministic results:
   ```typescript
   // Mock OpenAI responses for predictable testing
   mockIPC((cmd, args) => {
     if (cmd === 'transcribe_audio') {
       return { text: 'hello world', confidence: 0.95 }
     }
   })
   ```

---

## Security Considerations

### Content Security Policy (CSP)

VoKey's `tauri.conf.json` sets `"csp": null`, which disables Content Security Policy. This configuration choice is necessary for several reasons:

1. **Tauri IPC**: The `__TAURI_INVOKE__` bridge requires inline script execution for communication between frontend and Rust backend
2. **Development**: Hot module replacement (HMR) in Vite uses inline styles and scripts
3. **Testing**: WebDriver-based E2E tests inject scripts via `browser.execute()`

**Production hardening (future consideration):** For production builds, consider enabling a restrictive CSP:

```json
{
  "security": {
    "csp": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'"
  }
}
```

The `unsafe-inline` directive is required for:
- Tauri's IPC mechanism (`__TAURI_INVOKE__`)
- React's dynamic style injection (CSS-in-JS, Tailwind)
- Test frameworks that inject assertions via `browser.execute()`

**Note:** Investigate nonce-based CSP or strict-dynamic for improved security while maintaining functionality in future sprints.

---

## CI/CD Considerations

### GitHub Actions Setup

```yaml
# .github/workflows/e2e.yml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libgtk-3-dev libwebkit2gtk-4.1-dev \
            webkit2gtk-driver \
            pipewire pipewire-pulse \
            xvfb

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '22'

      - uses: pnpm/action-setup@v2
        with:
          version: 10

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install tauri-driver
        run: cargo install tauri-driver

      - name: Install dependencies
        run: pnpm install

      - name: Build app
        run: pnpm tauri build

      - name: Setup virtual audio
        run: |
          # Start PipeWire
          systemctl --user start pipewire pipewire-pulse

          # Create virtual microphone
          pactl load-module module-null-sink sink_name=VirtualMic
          pactl load-module module-virtual-source source_name=VirtualMicSource master=VirtualMic.monitor

      - name: Run E2E tests
        run: |
          xvfb-run --auto-servernum pnpm test:e2e
```

---

## Summary: Recommended Stack

| Layer | Tool | Status |
|-------|------|--------|
| Unit Tests | **Vitest** + React Testing Library | 🟢 Ready to implement |
| IPC Mocking | **@tauri-apps/api/mocks** | 🟢 Ready to implement |
| E2E Framework | **WebdriverIO** + tauri-driver | 🟡 Requires Linux |
| Vision Testing | **TestDriver.ai** (optional) | 🟠 Paid service |
| Audio Mocking | **PipeWire** virtual mic | 🟡 Requires setup |
| Transcription | Mock + Real API tests | 🟢 Ready to implement |

---

## References

### Official Documentation
- [Tauri v2 Testing Guide](https://v2.tauri.app/develop/tests/)
- [Tauri Mocking APIs](https://v2.tauri.app/develop/tests/mocking/)
- [Tauri WebDriver Setup](https://v2.tauri.app/develop/tests/webdriver/)

### Community Resources
- [Tauri E2E Discussion #10123](https://github.com/tauri-apps/tauri/discussions/10123)
- [Tauri E2E Discussion #3768](https://github.com/tauri-apps/tauri/discussions/3768)
- [KittyCAD Playwright E2E Issue](https://github.com/KittyCAD/modeling-app/issues/983)

### Audio Testing
- [Testing Speech Recognition (Playwright)](https://dkarlovi.github.io/testing-speech-recognition/)
- [Mock MediaRecorder Tutorial](https://www.linkedin.com/pulse/mock-mediarecorder-api-playwright-tutorial-aashish-paudel-g70sf)
- [Chromium Fake Media Flags](https://webrtc.org/getting-started/testing)

### Tools
- [TestDriver.ai](https://testdriver.ai/)
- [WebdriverIO](https://webdriver.io/)
- [Vitest](https://vitest.dev/)
