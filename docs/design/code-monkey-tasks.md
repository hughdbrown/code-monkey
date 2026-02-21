# Code Monkey — Implementation Task List

## Stage 1: Project Skeleton + Parser

### Task 1.1: Initialize Cargo project

- **Code**:
  - Run `cargo init` in `~/workspace/hughdbrown/rust/code-monkey`
  - Add dependencies to `Cargo.toml`:
    - `anyhow = "1"` (application error handling)
    - `thiserror = "2"` (library error types)
    - `clap = { version = "4", features = ["derive"] }` (CLI)
    - `fastrand = "2"` (typing jitter RNG)
    - `serde = { version = "1", features = ["derive"] }` (serialization)
    - `serde_json = "1"` (JSON wire format)
    - `ratatui = "0.29"` (TUI — used in stage 6)
    - `crossterm = "0.28"` (terminal backend)
  - Create module structure:
    ```
    src/
      main.rs          # CLI entry point
      lib.rs           # Re-exports
      parser/
        mod.rs         # Parser module
        types.rs       # Directive, FrontMatter, Script types
        lexer.rs       # Line-by-line directive parsing
        front_matter.rs # Front matter extraction
      grouper.rs       # Group directives into action blocks
      protocol/
        mod.rs         # Network protocol types and codec
        messages.rs    # Message types (Execute, Ack, Ping, Pong)
        codec.rs       # Length-prefixed framing
      agent/
        mod.rs         # Agent (demo machine) server
        applescript.rs # macOS automation functions
        typewriter.rs  # Simulated typing via AppleScript
      client/
        mod.rs         # Presenter client
      tui/
        mod.rs         # TUI module (placeholder)
    ```
- **Verify**: `cargo check` compiles with no errors.

### Task 1.2: Define core types

- **Tests**: In `src/parser/types.rs`:
  - `test_directive_display` — Each directive variant has a human-readable Display impl.
  - `test_front_matter_defaults` — `FrontMatter::default()` returns `typing_speed: 40`, `typing_variance: 15`, `title: None`, `agent_port: 9876`.
- **Code**: In `src/parser/types.rs`:
  - `enum Directive` with variants:
    - `Say(String)`
    - `Type(String)`
    - `Run`
    - `Pause(Option<u64>)` — None = wait for Enter, Some(n) = auto-continue after n seconds
    - `Focus(String)`
    - `Slide(SlideAction)`
    - `Key(String)`
    - `Clear`
    - `Wait(u64)`
    - `Exec(String)`
    - `Section(String)`
  - Derive `Serialize, Deserialize` on `Directive` and `SlideAction` (needed for wire protocol).
  - `enum SlideAction` — `Next`, `Prev`, `GoTo(u32)`
  - `struct FrontMatter` — `title: Option<String>`, `typing_speed: u64`, `typing_variance: u64`, `agent_port: u16`
  - `struct ParsedLine` — `line_number: usize`, `directive: Directive`
  - `struct Script` — `front_matter: FrontMatter`, `lines: Vec<ParsedLine>`
  - Implement `Default` for `FrontMatter`.
  - Implement `Display` for `Directive` (used in TUI and dry-run output).
- **Verify**: `cargo test`

### Task 1.3: Implement line parser (lexer)

- **Tests**: In `src/parser/lexer.rs`:
  - `test_parse_empty_line` — Returns `None` (skip).
  - `test_parse_comment` — `# this is a comment` returns `None`.
  - `test_parse_say` — `[SAY] Hello world` → `Directive::Say("Hello world".into())`.
  - `test_parse_type` — `[TYPE] cargo build` → `Directive::Type("cargo build".into())`.
  - `test_parse_run` — `[RUN]` → `Directive::Run`.
  - `test_parse_pause_no_arg` — `[PAUSE]` → `Directive::Pause(None)`.
  - `test_parse_pause_with_seconds` — `[PAUSE 3]` → `Directive::Pause(Some(3))`.
  - `test_parse_focus` — `[FOCUS] Terminal` → `Directive::Focus("Terminal".into())`.
  - `test_parse_slide_next` — `[SLIDE next]` → `Directive::Slide(SlideAction::Next)`.
  - `test_parse_slide_prev` — `[SLIDE prev]` → `Directive::Slide(SlideAction::Prev)`.
  - `test_parse_slide_number` — `[SLIDE 5]` → `Directive::Slide(SlideAction::GoTo(5))`.
  - `test_parse_key` — `[KEY cmd+s]` → `Directive::Key("cmd+s".into())`.
  - `test_parse_clear` — `[CLEAR]` → `Directive::Clear`.
  - `test_parse_wait` — `[WAIT 2]` → `Directive::Wait(2)`.
  - `test_parse_exec` — `[EXEC cargo build --release]` → `Directive::Exec("cargo build --release".into())`.
  - `test_parse_section` — `## Section: Intro` → `Directive::Section("Intro".into())`.
  - `test_parse_unknown_directive` — `[BOGUS]` → error with line context.
  - `test_parse_directive_case_insensitive` — `[say] hello` → `Directive::Say("hello".into())`.
  - `test_parse_say_preserves_whitespace` — `[SAY]   spaced out  ` → trims leading/trailing whitespace from the text.
- **Code**: In `src/parser/lexer.rs`:
  - `pub fn parse_line(line: &str, line_number: usize) -> Result<Option<ParsedLine>, ParseError>`
  - `ParseError` type (via `thiserror`) with fields: `line_number`, `line_content`, `message`.
  - Logic: trim line, check for empty/comment, match `[DIRECTIVE]` pattern with case-insensitive prefix, extract argument, parse into `Directive` variant.
- **Verify**: `cargo test`

### Task 1.4: Implement full script parser

- **Tests**: In `src/parser/mod.rs`:
  - `test_parse_empty_script` — Empty string → `Script` with default front matter and no lines.
  - `test_parse_comments_only` — Only comments → no lines.
  - `test_parse_multi_line_script` — Multi-line script → correct sequence of `ParsedLine`s.
  - `test_parse_error_includes_line_number` — Script with error on line 5 → error message says "line 5".
  - `test_parse_full_script_roundtrip` — A realistic multi-section script parses to the expected `Script` struct.
- **Code**: In `src/parser/mod.rs`:
  - `pub fn parse_script(input: &str) -> Result<Script, ParseError>`
  - Iterate over lines, call `parse_line` for each, collect into `Vec<ParsedLine>`.
  - Skip `None` results (comments, blank lines).
  - Front matter extraction delegated to Task 2.1.
- **Verify**: `cargo test`

---

## Stage 2: Grouper + Front Matter

### Task 2.1: Implement front matter parser

- **Tests**: In `src/parser/front_matter.rs`:
  - `test_front_matter_basic` — Input with `---\ntitle: My Talk\ntyping_speed: 60\n---\n[SAY] hi` → `FrontMatter { title: Some("My Talk"), typing_speed: 60, typing_variance: 15, agent_port: 9876 }`, remaining lines start after closing `---`.
  - `test_front_matter_missing` — Input without `---` → default `FrontMatter`, all lines parsed.
  - `test_front_matter_empty` — `---\n---\n[SAY] hi` → default `FrontMatter`.
  - `test_front_matter_unknown_key_warns` — Unknown key `foo: bar` is ignored (no error).
  - `test_front_matter_invalid_number` — `typing_speed: abc` → error with line number.
  - `test_front_matter_all_fields` — All four fields set → all extracted correctly.
  - `test_front_matter_agent_port` — `agent_port: 4444` → `FrontMatter { agent_port: 4444, .. }`.
- **Code**: In `src/parser/front_matter.rs`:
  - `pub fn extract_front_matter(lines: &[&str]) -> Result<(FrontMatter, usize), ParseError>`
  - Returns the parsed front matter and the line index where content begins.
  - Update `parse_script` to call this first.
- **Verify**: `cargo test`

### Task 2.2: Implement action block grouper

- **Tests**: In `src/grouper.rs`:
  - `test_group_single_action` — `[TYPE] hello`, `[RUN]` → one action block with two directives.
  - `test_group_say_before_action` — `[SAY] text`, `[FOCUS] T`, `[TYPE] cmd`, `[RUN]` → one block with narration = "text" and 3 action directives.
  - `test_group_multiple_say_accumulate` — `[SAY] line1`, `[SAY] line2`, `[TYPE] x` → narration = "line1\nline2", action = `[TYPE] x`.
  - `test_group_pause_standalone` — `[TYPE] x`, `[PAUSE]`, `[TYPE] y` → three blocks: action(TYPE x), pause, action(TYPE y).
  - `test_group_pause_with_timeout` — `[PAUSE 3]` → pause block with timeout.
  - `test_group_section_header` — `## Section: Intro` updates the current section name on subsequent blocks.
  - `test_group_empty_script` — No directives → no blocks.
  - `test_group_say_only` — `[SAY] text` with no following action → a narration-only block (displayed, Enter advances).
  - `test_group_complex_script` — Multi-section script with mixed directives → correct block sequence.
- **Code**: In `src/grouper.rs`:
  - `struct ActionBlock`:
    - `narration: Option<String>` — accumulated `[SAY]` text
    - `actions: Vec<Directive>` — the directives to execute (sent to agent)
    - `section: Option<String>` — current section name
    - `block_type: BlockType` — `Action`, `Pause(Option<u64>)`, `NarrationOnly`
  - Derive `Serialize, Deserialize` on `ActionBlock` and `BlockType`.
  - `enum BlockType` — `Action`, `Pause(Option<u64>)`, `NarrationOnly`
  - `pub fn group_into_blocks(script: &Script) -> Vec<ActionBlock>`
  - Logic:
    - Walk through `ParsedLine`s.
    - Accumulate `[SAY]` text.
    - Track current section from `Section` directives.
    - When a non-SAY/non-Section directive is encountered, start an action group.
    - Continue grouping until a `[PAUSE]`, `[SAY]`, or `Section` is hit.
    - `[PAUSE]` always flushes the current group and becomes its own block.
    - At end-of-input, flush any remaining narration as a `NarrationOnly` block.
- **Verify**: `cargo test`

---

## Stage 3: macOS Automation Layer

### Task 3.1: Implement AppleScript generation

- **Tests**: In `src/agent/applescript.rs`:
  - `test_focus_app_script` — `focus_app_script("Terminal")` → `tell application "Terminal" to activate`.
  - `test_focus_app_escapes_quotes` — `focus_app_script("My \"App\"")` → quotes are escaped in the AppleScript string.
  - `test_slide_next_script` — `slide_next_script()` → correct Keynote AppleScript.
  - `test_slide_prev_script` — `slide_prev_script()` → correct Keynote AppleScript.
  - `test_slide_goto_script` — `slide_goto_script(5)` → correct Keynote AppleScript.
  - `test_keystroke_simple` — `keystroke_script("a")` → `tell application "System Events" to keystroke "a"`.
  - `test_keystroke_with_cmd` — `keystroke_script("cmd+s")` → `keystroke "s" using command down`.
  - `test_keystroke_with_multiple_modifiers` — `keystroke_script("cmd+shift+s")` → `using {command down, shift down}`.
  - `test_keystroke_return` — `keystroke_script("return")` → `key code 36`.
  - `test_keystroke_ctrl_c` — `keystroke_script("ctrl+c")` → `keystroke "c" using control down`.
  - `test_type_char_script` — `type_char_script('h')` → correct keystroke AppleScript.
  - `test_clear_script` — `clear_script()` → sends Ctrl+L via AppleScript.
- **Code**: In `src/agent/applescript.rs`:
  - `pub fn focus_app_script(app_name: &str) -> String`
  - `pub fn slide_next_script() -> String`
  - `pub fn slide_prev_script() -> String`
  - `pub fn slide_goto_script(n: u32) -> String`
  - `pub fn keystroke_script(combo: &str) -> String` — parses `mod+mod+key` format.
  - `pub fn type_char_script(ch: char) -> String` — single character keystroke.
  - `pub fn clear_script() -> String`
  - `pub fn run_applescript(script: &str) -> Result<String>` — calls `osascript -e` via `std::process::Command`.
  - Helper: `fn parse_key_combo(combo: &str) -> (Vec<&str>, &str)` — splits modifiers from key.
  - Modifier mapping: `cmd` → `command down`, `ctrl` → `control down`, `shift` → `shift down`, `alt`/`opt` → `option down`.
- **Verify**: `cargo test`

### Task 3.2: Implement typewriter effect

- **Tests**: In `src/agent/typewriter.rs`:
  - `test_typewriter_generates_per_char_scripts` — `typewriter_to_applescript("hello", 40, 0)` produces 5 `(script, delay)` pairs.
  - `test_typewriter_empty_string` — Empty string produces empty vec.
  - `test_typewriter_special_chars` — String with spaces, punctuation outputs correct AppleScript per char.
  - `test_typewriter_variance_range` — With `variance: 10`, all delays fall within `[speed, speed+variance]`.
- **Code**: In `src/agent/typewriter.rs`:
  - `pub fn typewriter_to_applescript(text: &str, speed_ms: u64, variance_ms: u64) -> Vec<(String, u64)>`
    - Returns a vec of `(applescript_string, delay_ms)` pairs — one per character.
    - Uses `fastrand` for jitter: `speed_ms + fastrand::u64(0..=variance_ms)`.
  - `pub fn execute_typewriter(text: &str, speed_ms: u64, variance_ms: u64) -> Result<()>`
    - Calls `run_applescript` for each character with the delay.
- **Verify**: `cargo test`

---

## Stage 4: Network Protocol + Agent

### Task 4.1: Define protocol message types

- **Tests**: In `src/protocol/messages.rs`:
  - `test_serialize_execute` — `Message::Execute { actions: vec![Directive::Run] }` serializes to expected JSON.
  - `test_deserialize_execute` — JSON string deserializes to `Message::Execute` with correct actions.
  - `test_serialize_ack_ok` — `Message::Ack { status: "ok", message: None }` serializes correctly.
  - `test_serialize_ack_error` — `Message::Ack { status: "error", message: Some("...") }` serializes correctly.
  - `test_deserialize_ack` — JSON ack round-trips correctly.
  - `test_serialize_ping_pong` — `Ping` and `Pong` messages serialize/deserialize.
- **Code**: In `src/protocol/messages.rs`:
  - ```rust
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "type")]
    enum Message {
        Execute { actions: Vec<Directive>, typing_speed: u64, typing_variance: u64 },
        Ack { status: AckStatus, message: Option<String> },
        Ping,
        Pong,
    }

    #[derive(Serialize, Deserialize, Debug)]
    enum AckStatus { Ok, Error }
    ```
  - The `Execute` message includes `typing_speed` and `typing_variance` so the agent doesn't need to parse front matter itself (simplification — the client is the authority on script config).
- **Verify**: `cargo test`

### Task 4.2: Implement length-prefixed codec

- **Tests**: In `src/protocol/codec.rs`:
  - `test_encode_message` — A message encodes to `[4-byte length][JSON bytes]`.
  - `test_decode_message` — Encoded bytes decode back to the original message.
  - `test_decode_partial_read` — When given incomplete bytes, decoder requests more data.
  - `test_roundtrip_large_message` — A message with a long `[TYPE]` text round-trips correctly.
- **Code**: In `src/protocol/codec.rs`:
  - `pub fn encode_message(msg: &Message) -> Result<Vec<u8>>` — serialize to JSON, prepend 4-byte big-endian length.
  - `pub fn decode_message(buf: &[u8]) -> Result<Option<(Message, usize)>>` — read length prefix, check if enough bytes available, deserialize JSON. Returns `None` if buffer is incomplete. Returns `(message, bytes_consumed)` on success.
- **Verify**: `cargo test`

### Task 4.3: Implement the agent server

- **Tests**: In `src/agent/mod.rs`:
  - `test_agent_handles_execute` — Spawn agent on localhost with a mock executor (records calls instead of running AppleScript). Send an `Execute` message. Verify the mock received the correct directives and an `Ack { status: Ok }` was returned.
  - `test_agent_handles_ping` — Send `Ping`, receive `Pong`.
  - `test_agent_returns_error_on_failure` — Mock executor returns an error. Verify `Ack { status: Error, message }` is returned.
  - `test_agent_accepts_reconnect` — Connect, disconnect, reconnect. Agent handles all three without crashing.
- **Code**: In `src/agent/mod.rs`:
  - `pub trait ActionExecutor` — trait with `fn execute(&self, actions: &[Directive], typing_speed: u64, typing_variance: u64) -> Result<()>`. This allows both real AppleScript execution and mock execution for tests.
  - `struct AppleScriptExecutor` — implements `ActionExecutor` using the functions from Task 3.1/3.2.
  - `struct Agent`:
    - `executor: Box<dyn ActionExecutor>`
    - `port: u16`
  - `Agent::new(executor: Box<dyn ActionExecutor>, port: u16) -> Self`
  - `Agent::run(&self) -> Result<()>`:
    - `TcpListener::bind(("0.0.0.0", self.port))`
    - Accept one connection at a time (single presenter).
    - Read loop: decode messages, dispatch:
      - `Execute` → call `self.executor.execute(actions)`, send `Ack`.
      - `Ping` → send `Pong`.
    - On disconnect, log and return to accept loop.
  - The agent does **not** parse the script — it receives actions over the wire. This simplifies the agent and means the client is the single source of truth for script interpretation.
- **Verify**: `cargo test` (network tests use `127.0.0.1` with port 0 for OS-assigned port).

**Note**: Task 4.1's design decision updated here — since the client sends full action details in the `Execute` message (including typing speed), the **agent does not need the script file**. This is simpler than the "script on both sides" approach described in the PRD. The agent is a pure executor. Update the PRD's "Why Both Machines Need the Script" section accordingly: only the client needs the script.

### Task 4.4: Create a sample script and manual test harness

- **Code**: Create `examples/demo.cm`:
  ```
  ---
  title: Code Monkey Demo
  typing_speed: 40
  ---

  ## Section: Introduction

  [SAY] Welcome to the Code Monkey demo.
  [SAY] Watch as I type commands automatically.

  [FOCUS] Terminal
  [TYPE] echo "Hello from Code Monkey!"
  [RUN]
  [PAUSE]

  [SAY] Let's try another command.

  [TYPE] ls -la
  [RUN]
  [PAUSE]

  ## Section: Conclusion

  [SAY] That's the basic idea. Thanks for watching!
  ```
- **Verify**: Manual test: start agent on localhost, use a simple test client (can be a `#[ignore]` test or a `examples/test_client.rs`) to send an `Execute` message and verify the agent responds with `Ack`.

---

## Stage 5: Presenter Client + CLI

### Task 5.1: Implement the presenter client

- **Tests**: In `src/client/mod.rs`:
  - `test_client_connects` — Start a mock server, client connects successfully.
  - `test_client_sends_execute_receives_ack` — Client sends `Execute`, mock server replies `Ack { Ok }`, client returns success.
  - `test_client_handles_error_ack` — Mock server replies `Ack { Error, "no accessibility" }`, client returns the error message.
  - `test_client_reconnects_on_disconnect` — Mock server accepts, then drops connection. Client reconnects when server accepts again.
  - `test_client_tracks_block_progress` — After executing 3 of 10 blocks, `progress()` returns `(3, 10)`.
- **Code**: In `src/client/mod.rs`:
  - `struct Presenter`:
    - `blocks: Vec<ActionBlock>`
    - `current: usize`
    - `front_matter: FrontMatter`
    - `connection: Option<TcpStream>`
    - `agent_addr: SocketAddr`
  - `Presenter::new(script: Script, agent_addr: SocketAddr) -> Self` — parses and groups internally.
  - `Presenter::connect(&mut self) -> Result<()>` — TCP connect with retry logic.
  - `Presenter::current_block(&self) -> Option<&ActionBlock>`
  - `Presenter::step(&mut self) -> Result<StepResult>`:
    - If block is `NarrationOnly` or `Pause` → advance locally, no network.
    - If block is `Action` → send `Execute` to agent, wait for `Ack`, advance.
  - `Presenter::go_back(&mut self)` — decrement current.
  - `Presenter::progress(&self) -> (usize, usize)`.
  - `enum StepResult` — `Executed`, `Paused(Option<u64>)`, `NarrationOnly`, `Finished`, `AgentError(String)`.
  - Reconnection logic: on send/receive failure, set `connection = None`, attempt `connect()`, return `ConnectionLost` status for TUI to display.
- **Verify**: `cargo test`

### Task 5.2: Implement CLI with clap

- **Tests**: In `tests/cli.rs`:
  - `test_cli_agent_subcommand` — Parsing `["code-monkey", "agent", "demo.cm"]` succeeds with default port.
  - `test_cli_agent_custom_port` — Parsing `["code-monkey", "agent", "--port", "4444", "demo.cm"]` sets port.
  - `test_cli_present_subcommand` — Parsing `["code-monkey", "present", "--agent", "192.168.77.2:9876", "demo.cm"]` parses correctly.
  - `test_cli_present_dry_run` — Parsing `["code-monkey", "present", "--dry-run", "demo.cm"]` sets dry_run flag.
  - `test_cli_check_subcommand` — Parsing `["code-monkey", "check", "demo.cm"]` succeeds.
  - `test_cli_present_missing_agent` — Parsing `["code-monkey", "present", "demo.cm"]` fails (--agent is required unless --dry-run).
- **Code**: In `src/main.rs`:
  - ```rust
    #[derive(Parser)]
    #[command(name = "code-monkey", version, about = "Automated presentation assistant")]
    struct Cli {
        #[command(subcommand)]
        command: Commands,
    }

    enum Commands {
        /// Start the demo agent (run on the demo machine)
        Agent {
            /// Script file path
            script: PathBuf,
            /// TCP port to listen on
            #[arg(long, default_value = "9876")]
            port: u16,
        },
        /// Run a presentation (run on the presenter's laptop)
        Present {
            /// Script file path
            script: PathBuf,
            /// Agent address (ip:port)
            #[arg(long)]
            agent: Option<String>,
            /// Show actions without connecting or executing
            #[arg(long)]
            dry_run: bool,
        },
        /// Parse and validate a script without running
        Check {
            /// Script file path
            script: PathBuf,
        },
    }
    ```
  - `agent` command: parse script (for validation only), create `AppleScriptExecutor`, create `Agent`, call `agent.run()`.
  - `check` command: parse script, report success or errors.
  - `present --dry-run`: parse → group → print blocks to stdout.
  - `present --agent`: handled in stage 6 with TUI.
- **Verify**: `cargo test` and manual: `cargo run -- check examples/demo.cm`

---

## Stage 6: Presenter TUI

### Task 6.1: Implement TUI layout and rendering

- **Tests**: Manual testing only (TUI rendering is impractical to unit test).
- **Code**: In `src/tui/mod.rs`:
  - `struct App`:
    - `presenter: Presenter`
    - `should_quit: bool`
    - `status_message: Option<String>` — transient status ("Executing...", "Reconnecting...", etc.)
    - `connection_state: ConnectionState` — `Connected`, `Disconnected`, `Reconnecting(u32)`
  - `enum ConnectionState` — `Connected`, `Disconnected`, `Reconnecting(u32)`
  - `App::new(presenter: Presenter) -> Self`
  - `fn ui(frame: &mut Frame, app: &App)` — renders the TUI layout:
    - **Title bar**: presentation title + progress `[N / M]`
    - **Connection indicator**: green dot + address when connected, red dot + "Reconnecting..." when not
    - **Section**: current section name
    - **Narration pane**: current `[SAY]` text, styled prominently (this is what the presenter reads)
    - **Action pane**: upcoming actions, styled as a list with directive prefixes
    - **Status bar**: transient messages ("Executing...", "Agent error: ...", etc.)
    - **Footer**: "Press Enter to execute | b = back | q = quit"
  - Layout: vertical chunks using `ratatui::layout::Layout`.
  - Styling: narration text in bold/white, action directives in cyan, section in yellow, connection green/red.
- **Verify**: `cargo run -- present --agent 127.0.0.1:9876 examples/demo.cm` shows the TUI.

### Task 6.2: Implement TUI event loop

- **Tests**: Manual testing only.
- **Code**: In `src/tui/mod.rs`:
  - `pub fn run_tui(app: &mut App) -> Result<()>`
  - Event loop:
    - `crossterm::terminal::enable_raw_mode()`
    - `crossterm::execute!(stdout, EnterAlternateScreen)`
    - Attempt initial connection, set `connection_state` accordingly.
    - Loop:
      - `terminal.draw(|f| ui(f, &app))`
      - `crossterm::event::read()` with poll timeout (250ms for responsive status updates):
        - `KeyCode::Enter`:
          - If disconnected → attempt reconnect.
          - If connected → call `app.presenter.step()`. Handle `StepResult`:
            - `Executed` → advance display, clear status.
            - `AgentError(msg)` → show error in status bar, don't advance.
            - `Paused(None)` → show narration, wait for next Enter.
            - `Paused(Some(n))` → show countdown timer, auto-advance.
            - `NarrationOnly` → advance display immediately.
            - `Finished` → show "Presentation complete!" and wait for q.
          - On connection error → set `connection_state = Reconnecting`, show status.
        - `KeyCode::Char('q')` → set `should_quit = true`
        - `KeyCode::Char('b')` → call `app.presenter.go_back()`
        - `KeyCode::Char('s')` → skip current block (if agent not responding)
      - If `should_quit` → break
    - `crossterm::terminal::disable_raw_mode()`
    - `crossterm::execute!(stdout, LeaveAlternateScreen)`
  - Panic hook: install a hook that restores terminal before panicking (ratatui pattern).
- **Verify**: Full manual test with two terminals on localhost:
  1. Terminal 1: `cargo run -- agent examples/demo.cm`
  2. Terminal 2: `cargo run -- present --agent 127.0.0.1:9876 examples/demo.cm`
  3. Walk through the script with Enter, verify actions execute in Terminal 1's environment.

### Task 6.3: Wire TUI into CLI

- **Code**: In `src/main.rs`:
  - Update `Commands::Present` handler:
    - If `dry_run` → print blocks to stdout (no TUI, no network).
    - If not → parse agent address, create `Presenter`, create `App`, call `run_tui(&mut app)`.
  - Validate that `--agent` is provided when not in `--dry-run` mode.
  - Ensure clean terminal restore on all exit paths (Ctrl+C, q, finish, error).
- **Verify**: Full end-to-end test as in Task 6.2.

---

## Risks and Notes

### Stage 1-2 Risks
- **None significant.** Pure data structures and parsing — no external dependencies or side effects.

### Stage 3 Risks
- **Accessibility permissions**: Simulated keystrokes via AppleScript require the terminal app on the demo machine to have Accessibility permission in System Settings > Privacy & Security > Accessibility. If not granted, all automation silently fails. The agent should run a benign AppleScript on startup and warn if it fails.
- **Keynote vs PowerPoint**: Slide commands assume Keynote. PowerPoint uses a different AppleScript dictionary. Document as a known v1 limitation. The `[KEY right]` directive can be used as a generic workaround for any presentation app.

### Stage 4 Risks
- **Port conflicts**: The default port 9876 may be in use. The `--port` flag handles this, but the agent should give a clear error message.
- **Single connection**: The agent accepts only one presenter at a time. This is intentional — multiple presenters would cause chaos. But the agent must gracefully handle a second connection attempt (reject with a message).

### Stage 5 Risks
- **Reconnection timing**: The client retries every second. During a live presentation, a 1-second gap is noticeable but acceptable. A longer outage (>5 seconds) will be visible to the audience as a pause in the demo.

### Stage 6 Risks
- **Typing speed tuning**: The right default (40ms) may feel too fast or too slow. This is subjective. Test with real presentations.
- **ratatui version churn**: ratatui moves fast. Pin to a specific minor version.
- **Testing on one machine**: During development, both agent and client run on localhost. The `[FOCUS]` directives will steal focus from the presenter TUI terminal. This is expected and not a problem — it only happens during single-machine testing, not in the real two-machine setup.

### Design Simplification from PRD

Task 4.3 simplifies the PRD's "script on both sides" design. The **agent does not need the script file** — it receives complete action blocks from the client over TCP. Only the presenter client parses and groups the script. This means:
- No need to keep scripts in sync between machines
- No script mismatch bugs
- The agent is a generic "action executor" that could theoretically be reused for other purposes
- The presenter client is the single source of truth

The agent's `--script` argument (in the CLI) is kept only for startup validation and displaying a confirmation message. The agent does not parse the script for execution.

## Coverage Validation

| Acceptance Criterion | Covered By |
|---|---|
| #1 test_parse_empty_script | Task 1.4 |
| #2 test_parse_comments_only | Task 1.4 |
| #3 test_parse_front_matter | Task 2.1 |
| #4-16 (all directive parsing) | Task 1.3 |
| #17 test_parse_unknown_directive_errors | Task 1.3 |
| #18 test_group_action_block | Task 2.2 |
| #19 test_group_say_accumulates | Task 2.2 |
| #20 test_group_pause_standalone | Task 2.2 |
| #21-23 (AppleScript generation) | Task 3.1 |
| #24 test_typewriter_output | Task 3.2 |
| #25-26 (message serialization) | Task 4.1 |
| #27 test_length_prefix_roundtrip | Task 4.2 |
| #28-29 (agent execute + ack) | Task 4.3 |
| #30 test_client_reconnects | Task 5.1 |
| #31-34 (CLI args) | Task 5.2 |
| #35 test_dry_run_no_side_effects | Task 5.2 |
| #36 test_full_script_roundtrip | Task 1.4 |
