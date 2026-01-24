# Sprint 7B — Post-processing Modes

**Goal:** Add one "wow" upgrade to VoKey Transcribe.

**Decision:** Implement **Option B: Post-processing Modes** first, followed by Option A: Streaming in a future sprint.

**Target:** Complete post-processing pipeline with 4 modes: Normal, Coding, Markdown, Prompt

---

## Options Analysis

### Option A: Streaming Partial Transcript (Realtime API)

**What it does:**
- Show partial text in HUD while recording
- Use OpenAI Realtime API (WebSocket-based) for live transcription
- Finalize with high-quality text on stop

**Pros:**
- Immediate visual feedback ("wow" factor)
- Modern UX experience
- OpenAI Realtime API now supports transcription-only mode

**Cons:**
- Requires WebSocket/WebRTC connection management
- Significant architecture changes (audio streaming to API)
- Higher API costs (Realtime API pricing)
- More complex error handling and reconnection logic
- Audio must be streamed in real-time (not post-capture)

**Effort:** High (3-4 phases, significant refactoring)

---

### Option B: Post-processing Modes (Recommended for Sprint 7)

**What it does:**
- After transcription, apply text transformation based on selected mode
- Modes: Normal (raw), Coding (variable case), Markdown (formatting), Prompt (custom)
- Uses existing batch transcription flow

**Pros:**
- Builds on existing architecture (minimal changes)
- High value for developers (coding mode is killer feature)
- Lower complexity and faster delivery
- Prepares architecture for future AI enhancements
- Can be combined with streaming later

**Cons:**
- No real-time feedback during recording
- Additional API call for post-processing (if using LLM)

**Effort:** Medium (2-3 phases)

---

## Recommendation

**Start with Option B (Post-processing Modes)** for Sprint 7B:

1. Lower risk — builds on proven batch transcription
2. Faster delivery — can complete in 1-2 weeks
3. High developer value — coding mode for variable names, markdown for docs
4. Future-ready — architecture supports adding streaming in Sprint 8

---

## Sprint 7B Implementation Plan

### Phase 1: Mode Selection Infrastructure

#### 1.1 Define Processing Modes

**New File:** `src-tauri/src/processing/mod.rs`

```rust
use serde::{Deserialize, Serialize};

/// Processing mode applied after transcription
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingMode {
    /// Raw transcription output, no processing
    #[default]
    Normal,
    /// Coding mode: convert to snake_case/camelCase, remove filler words
    Coding,
    /// Markdown mode: format as markdown (lists, headers, emphasis)
    Markdown,
    /// Prompt mode: apply custom user prompt
    Prompt,
}

impl ProcessingMode {
    pub fn label(&self) -> &'static str {
        match self {
            ProcessingMode::Normal => "Normal",
            ProcessingMode::Coding => "Coding",
            ProcessingMode::Markdown => "Markdown",
            ProcessingMode::Prompt => "Prompt",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ProcessingMode::Normal => "Raw transcription, no changes",
            ProcessingMode::Coding => "Code-friendly: snake_case, remove fillers",
            ProcessingMode::Markdown => "Format as markdown lists and structure",
            ProcessingMode::Prompt => "Apply custom transformation prompt",
        }
    }
}
```

#### 1.2 Add Mode State

**Modify:** `src-tauri/src/lib.rs`

```rust
use std::sync::atomic::{AtomicU8, Ordering};

// Global mode state (atomic for thread-safety)
static CURRENT_MODE: AtomicU8 = AtomicU8::new(0); // 0 = Normal

fn get_processing_mode() -> ProcessingMode {
    match CURRENT_MODE.load(Ordering::SeqCst) {
        1 => ProcessingMode::Coding,
        2 => ProcessingMode::Markdown,
        3 => ProcessingMode::Prompt,
        _ => ProcessingMode::Normal,
    }
}

#[tauri::command]
fn set_processing_mode(mode: ProcessingMode) {
    let value = match mode {
        ProcessingMode::Normal => 0,
        ProcessingMode::Coding => 1,
        ProcessingMode::Markdown => 2,
        ProcessingMode::Prompt => 3,
    };
    CURRENT_MODE.store(value, Ordering::SeqCst);
    log::info!("Processing mode changed to: {:?}", mode);
}

#[tauri::command]
fn get_current_processing_mode() -> ProcessingMode {
    get_processing_mode()
}
```

#### 1.3 Tray Menu Mode Selector

**Modify:** `src-tauri/src/lib.rs` — Tray menu

```rust
// Add mode submenu to tray
let mode_menu = SubmenuBuilder::new(app, "Mode")
    .item(&MenuItem::new(app, "Normal", true, None::<&str>)?)
    .item(&MenuItem::new(app, "Coding", true, None::<&str>)?)
    .item(&MenuItem::new(app, "Markdown", true, None::<&str>)?)
    .item(&MenuItem::new(app, "Prompt", true, None::<&str>)?)
    .build()?;

// Handle menu events
app.on_menu_event(move |app, event| {
    match event.id.0.as_str() {
        "Normal" => set_processing_mode(ProcessingMode::Normal),
        "Coding" => set_processing_mode(ProcessingMode::Coding),
        "Markdown" => set_processing_mode(ProcessingMode::Markdown),
        "Prompt" => set_processing_mode(ProcessingMode::Prompt),
        _ => {}
    }
});
```

---

### Phase 2: Processing Engines

#### 2.1 Normal Mode (Passthrough)

No processing needed — return transcription as-is.

#### 2.2 Coding Mode (Local Processing)

**New File:** `src-tauri/src/processing/coding.rs`

```rust
/// Convert transcription to code-friendly format
/// - Remove filler words ("um", "uh", "like", "you know")
/// - Convert to snake_case or camelCase based on context
/// - Handle common programming terms
pub fn process_coding(input: &str) -> String {
    let mut result = input.to_string();

    // Remove filler words
    let fillers = [
        "um", "uh", "like", "you know", "basically", "actually",
        "so", "well", "right", "okay", "ok",
    ];
    for filler in fillers {
        result = regex_replace_word(&result, filler, "");
    }

    // Normalize whitespace
    result = result.split_whitespace().collect::<Vec<_>>().join(" ");

    // Convert to snake_case
    result = to_snake_case(&result);

    result.trim().to_string()
}

fn to_snake_case(s: &str) -> String {
    s.to_lowercase()
        .replace(' ', "_")
        .replace('-', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn regex_replace_word(text: &str, word: &str, replacement: &str) -> String {
    // Simple word boundary replacement
    let pattern = format!(r"\b{}\b", regex::escape(word));
    regex::Regex::new(&pattern)
        .map(|re| re.replace_all(text, replacement).to_string())
        .unwrap_or_else(|_| text.to_string())
}
```

**Examples:**
| Input | Output |
|-------|--------|
| "um create user account" | "create_user_account" |
| "get the current time" | "get_current_time" |
| "like validate email address" | "validate_email_address" |

#### 2.3 Markdown Mode (Local Processing)

**New File:** `src-tauri/src/processing/markdown.rs`

```rust
/// Format transcription as markdown
/// - Detect list items ("first", "second", "next")
/// - Convert quoted text to code blocks
/// - Add emphasis for key terms
pub fn process_markdown(input: &str) -> String {
    let lines: Vec<&str> = input.split('.').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

    let mut result = Vec::new();
    let mut in_list = false;

    for line in lines {
        let lower = line.to_lowercase();

        // Detect list items
        if lower.starts_with("first") || lower.starts_with("1") {
            in_list = true;
            result.push(format!("1. {}", strip_ordinal(line)));
        } else if in_list && (lower.starts_with("second") || lower.starts_with("next") ||
                             lower.starts_with("then") || lower.starts_with("also")) {
            result.push(format!("- {}", strip_ordinal(line)));
        } else {
            if in_list {
                in_list = false;
                result.push(String::new()); // Add blank line after list
            }
            result.push(format!("{}.", line));
        }
    }

    result.join("\n")
}

fn strip_ordinal(s: &str) -> String {
    let ordinals = ["first", "second", "third", "next", "then", "also", "finally"];
    let mut result = s.to_string();
    for ord in ordinals {
        if result.to_lowercase().starts_with(ord) {
            result = result[ord.len()..].trim_start_matches([',', ' ']).to_string();
            break;
        }
    }
    result
}
```

#### 2.4 Prompt Mode (LLM Processing)

**New File:** `src-tauri/src/processing/prompt.rs`

```rust
use crate::transcription::TranscriptionError;

/// Process text using a custom LLM prompt
/// Uses OpenAI Chat Completions API for flexible transformations
pub async fn process_with_prompt(
    input: &str,
    custom_prompt: Option<&str>,
) -> Result<String, TranscriptionError> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| TranscriptionError::MissingApiKey)?;

    let system_prompt = custom_prompt.unwrap_or(
        "You are a text formatter. Clean up the following transcription for clarity. \
         Fix grammar, remove filler words, and improve readability. \
         Return only the formatted text, no explanations."
    );

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": input}
            ],
            "max_tokens": 1000,
            "temperature": 0.3
        }))
        .send()
        .await
        .map_err(|e| TranscriptionError::NetworkError(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error_text = response.text().await.unwrap_or_default();
        return Err(TranscriptionError::ApiError {
            status,
            message: error_text,
        });
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| TranscriptionError::ParseError(e.to_string()))?;

    let text = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or(input)
        .to_string();

    Ok(text)
}
```

---

### Phase 3: Pipeline Integration

#### 3.1 Post-Processing Pipeline

**New File:** `src-tauri/src/processing/pipeline.rs`

```rust
use super::{coding, markdown, prompt, ProcessingMode};
use crate::transcription::TranscriptionError;

/// Process transcribed text based on current mode
pub async fn process_text(
    raw_text: &str,
    mode: ProcessingMode,
    custom_prompt: Option<&str>,
) -> Result<String, TranscriptionError> {
    match mode {
        ProcessingMode::Normal => Ok(raw_text.to_string()),
        ProcessingMode::Coding => Ok(coding::process_coding(raw_text)),
        ProcessingMode::Markdown => Ok(markdown::process_markdown(raw_text)),
        ProcessingMode::Prompt => prompt::process_with_prompt(raw_text, custom_prompt).await,
    }
}
```

#### 3.2 Update Effects Runner

**Modify:** `src-tauri/src/effects.rs`

```rust
Effect::StartTranscription { id, wav_path } => {
    let metrics = self.metrics.clone();

    tokio::spawn(async move {
        log::info!("Starting transcription for {:?}", wav_path);

        // Track transcription started
        {
            let mut m = metrics.lock().await;
            m.transcription_started();
        }

        let start_time = Instant::now();

        // Step 1: Transcribe audio
        let raw_text = match transcription::transcribe_audio(&wav_path).await {
            Ok(text) => text,
            Err(e) => {
                log::error!("Transcription failed: {}", e);
                let mut m = metrics.lock().await;
                m.cycle_failed(e.to_string());
                let _ = tx.send(Event::TranscribeFail { id, err: e.to_string() }).await;
                return;
            }
        };

        // Step 2: Apply post-processing
        let mode = get_processing_mode();
        log::info!("Applying post-processing mode: {:?}", mode);

        let final_text = match processing::process_text(&raw_text, mode, None).await {
            Ok(text) => text,
            Err(e) => {
                log::warn!("Post-processing failed, using raw text: {}", e);
                raw_text // Fallback to raw on error
            }
        };

        let duration = start_time.elapsed();
        log::info!(
            "Transcription + processing complete: {} chars in {:?}",
            final_text.len(),
            duration
        );

        // Track completion
        {
            let mut m = metrics.lock().await;
            m.transcription_completed(final_text.len());
        }

        let _ = tx.send(Event::TranscribeOk { id, text: final_text }).await;
    });
}
```

---

### Phase 4: UI Integration

#### 4.1 Mode Indicator in HUD

**Modify:** `src/App.tsx`

```tsx
const [mode, setMode] = useState<string>('normal');

useEffect(() => {
  // Listen for mode changes
  const unlisten = listen('mode-changed', (event) => {
    setMode(event.payload as string);
  });
  return () => { unlisten.then(fn => fn()); };
}, []);

// In render:
{state.status === 'idle' && (
  <div className="mode-indicator">
    {mode.toUpperCase()}
  </div>
)}
```

#### 4.2 Debug Panel Mode Display

**Modify:** `src/Debug.tsx`

```tsx
const [currentMode, setCurrentMode] = useState<string>('normal');

async function fetchMode() {
  const mode = await invoke('get_current_processing_mode');
  setCurrentMode(mode as string);
}

async function changeMode(mode: string) {
  await invoke('set_processing_mode', { mode });
  setCurrentMode(mode);
}

// In render:
<section className="mode-selector">
  <h3>Processing Mode</h3>
  <div className="mode-buttons">
    {['normal', 'coding', 'markdown', 'prompt'].map(mode => (
      <button
        key={mode}
        className={currentMode === mode ? 'active' : ''}
        onClick={() => changeMode(mode)}
      >
        {mode.charAt(0).toUpperCase() + mode.slice(1)}
      </button>
    ))}
  </div>
</section>
```

#### 4.3 Styles

**Modify:** `src/styles/debug.css`

```css
.mode-selector {
  margin: 16px 0;
}

.mode-buttons {
  display: flex;
  gap: 8px;
}

.mode-buttons button {
  padding: 8px 16px;
  border: 1px solid #444;
  background: #2a2a2a;
  color: #fff;
  cursor: pointer;
  border-radius: 4px;
}

.mode-buttons button.active {
  background: #4a9eff;
  border-color: #4a9eff;
}

.mode-buttons button:hover:not(.active) {
  background: #3a3a3a;
}
```

**Modify:** `src/styles/hud.css`

```css
.mode-indicator {
  font-size: 10px;
  opacity: 0.6;
  text-transform: uppercase;
  letter-spacing: 1px;
}
```

---

## Phase 5: Prompt Configuration (Stretch Goal)

#### 5.1 Custom Prompt Storage

**New File:** `src-tauri/src/config.rs`

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub custom_prompt: Option<String>,
    pub default_mode: ProcessingMode,
}

impl AppConfig {
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("vokey-transcribe")
            .join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
}
```

#### 5.2 Tauri Commands for Config

```rust
#[tauri::command]
fn get_custom_prompt() -> Option<String> {
    AppConfig::load().custom_prompt
}

#[tauri::command]
fn set_custom_prompt(prompt: String) -> Result<(), String> {
    let mut config = AppConfig::load();
    config.custom_prompt = Some(prompt);
    config.save().map_err(|e| e.to_string())
}
```

---

## Files Summary

### New Files
| File | Purpose |
|------|---------|
| `src-tauri/src/processing/mod.rs` | Module exports, ProcessingMode enum |
| `src-tauri/src/processing/coding.rs` | Coding mode text processor |
| `src-tauri/src/processing/markdown.rs` | Markdown mode text processor |
| `src-tauri/src/processing/prompt.rs` | LLM-based prompt processor |
| `src-tauri/src/processing/pipeline.rs` | Processing pipeline orchestration |
| `src-tauri/src/config.rs` | App configuration persistence |

### Modified Files
| File | Changes |
|------|---------|
| `src-tauri/src/lib.rs` | Mode state, tray menu, new commands |
| `src-tauri/src/effects.rs` | Post-processing after transcription |
| `src-tauri/Cargo.toml` | Add `regex` dependency |
| `src/App.tsx` | Mode indicator in HUD |
| `src/Debug.tsx` | Mode selector UI |
| `src/styles/hud.css` | Mode indicator styles |
| `src/styles/debug.css` | Mode selector styles |

---

## Dependencies

Add to `src-tauri/Cargo.toml`:

```toml
regex = "1"
```

---

## Acceptance Criteria

1. **Mode Selection Works**
   - [ ] Can change mode via tray menu
   - [ ] Can change mode via Debug panel
   - [ ] Mode persists during session

2. **Normal Mode**
   - [ ] Returns raw transcription unchanged

3. **Coding Mode**
   - [ ] Removes filler words ("um", "uh", "like")
   - [ ] Converts to snake_case
   - [ ] Output is valid identifier format

4. **Markdown Mode**
   - [ ] Detects list items and formats as bullets
   - [ ] Adds structure to flowing text

5. **Prompt Mode**
   - [ ] Calls OpenAI Chat Completions API
   - [ ] Falls back to raw text on error
   - [ ] Uses default prompt if none configured

6. **HUD Integration**
   - [ ] Shows current mode indicator when idle
   - [ ] Mode visible but unobtrusive

---

## Testing Checklist

### Manual Tests

| Test | Steps | Expected |
|------|-------|----------|
| Mode switch | Change mode in Debug panel | Mode updates, indicator changes |
| Coding basic | Say "create user account" in Coding mode | Clipboard: `create_user_account` |
| Coding fillers | Say "um like get the current time" | Clipboard: `get_current_time` |
| Markdown list | Say "first do this, second do that" | Clipboard has `1.` and `-` formatting |
| Prompt mode | Say anything in Prompt mode | Text is cleaned/formatted by LLM |
| Fallback | Disconnect network, use Prompt mode | Falls back to raw transcription |

### Demo Script (30s)

1. Set mode to Coding via tray menu
2. Record: "um create user account"
3. Stop → Clipboard contains `create_user_account`
4. Switch to Normal mode
5. Record same phrase → Clipboard contains raw "um create user account"
6. Show mode indicator in HUD

---

## Parallel Work: Sprint 7A — Streaming Transcription

Sprint 7A (parallel team) is implementing Option A (Streaming):

1. Use OpenAI Realtime API transcription-only mode
2. WebSocket connection during recording
3. Show partial transcripts in HUD
4. Finalize with full batch transcription on stop
5. Combine with post-processing modes

The post-processing pipeline from Sprint 7B will work with Sprint 7A's streaming output.

---

## Implementation Order

### Week 1: Core Modes

| Day | Task | Files |
|-----|------|-------|
| 1 | Create processing module structure | `processing/*.rs` |
| 2 | Implement Coding mode | `coding.rs` |
| 3 | Implement Markdown mode | `markdown.rs` |
| 4 | Implement Prompt mode | `prompt.rs` |
| 5 | Create pipeline, integrate with effects | `pipeline.rs`, `effects.rs` |

### Week 2: UI + Polish

| Day | Task | Files |
|-----|------|-------|
| 1 | Add mode state and commands | `lib.rs` |
| 2 | Tray menu mode selector | `lib.rs` |
| 3 | Debug panel mode UI | `Debug.tsx` |
| 4 | HUD mode indicator | `App.tsx`, `hud.css` |
| 5 | Testing and bug fixes | All |

---

## Success Criteria

1. **Modes are useful** — Coding mode produces valid identifiers
2. **Pipeline is robust** — Errors fall back gracefully
3. **UX is clear** — User always knows current mode
4. **Architecture is extensible** — Easy to add new modes
