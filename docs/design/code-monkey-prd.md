# Code Monkey — Product Requirements Document

## Overview

Code Monkey is a two-machine presentation system for technical demos. A **presenter client** runs on the presenter's laptop, showing narration cues in a TUI. A **demo agent** runs on the audience-visible demo machine, executing automation directives (typing code, running commands, switching apps, advancing slides). The two communicate over a direct TCP connection. Both work from the same `.cm` script file. The presenter hits Enter to trigger each next action on the demo machine, while their own screen always shows what to say next.

This two-machine architecture mirrors how human code-monkey teams worked: the presenter reads their notes on one screen while the code-monkey operates the projected screen.

## Goals

1. **Single script, two machines** — One `.cm` script file is shared between presenter and agent. The presenter sees narration cues; the agent executes action directives.
2. **Enter-to-advance** — The presenter hits Enter to trigger the next action block on the demo machine. No other interaction required during the presentation.
3. **Simulated typing** — Code and commands are "typed" character-by-character at a natural speed on the demo machine, not pasted instantly.
4. **Cross-application orchestration** — The agent controls Terminal, Keynote, PowerPoint, and arbitrary macOS apps on the demo machine via AppleScript.
5. **Presenter TUI** — The presenter's laptop shows a terminal UI with current narration cue, upcoming action, and progress. The presenter never loses their view.
6. **Rock-solid connection** — Direct TCP over an ethernet cable between the two machines. Sub-millisecond latency, no external dependencies, automatic reconnection.
7. **Dry-run mode** — Validate a script without connecting or executing, showing what would happen at each step.
8. **macOS agent** — The demo agent leverages AppleScript/osascript. The presenter client is cross-platform (it only needs a terminal).

## Non-Goals

- **Not a slide tool** — Does not render slides. Use Keynote, PowerPoint, or presenterm for slides.
- **Not a recording tool** — Does not produce GIFs or videos. Use asciinema or VHS for that.
- **Not a single-machine tool** — The architecture assumes two machines. Running both on one machine is technically possible (for testing) but not the designed workflow.
- **No audience interaction** — This is a linear script runner, not an interactive REPL.
- **No Internet dependency** — Works on a direct cable with static IPs. No DNS, no DHCP, no cloud.

## Architecture

```
┌──────────────────────┐          TCP           ┌──────────────────────┐
│   Presenter Laptop   │ ◄──────────────────── ►│    Demo Machine      │
│                      │    ethernet cable       │                      │
│  ┌────────────────┐  │    192.168.77.1:9876    │  ┌────────────────┐  │
│  │ code-monkey    │  │  ◄──── go / ack ────►   │  │ code-monkey    │  │
│  │ present        │  │                         │  │ agent          │  │
│  │                │  │                         │  │                │  │
│  │ - Shows TUI    │  │                         │  │ - Runs osascript│ │
│  │ - Reads script │  │                         │  │ - Types code   │  │
│  │ - Sends "go"   │  │                         │  │ - Switches apps│  │
│  │   on Enter     │  │                         │  │ - Advances     │  │
│  └────────────────┘  │                         │  │   slides       │  │
│                      │                         │  └────────────────┘  │
│  Presenter reads     │                         │  Projected to        │
│  narration here      │                         │  audience            │
└──────────────────────┘                         └──────────────────────┘
```

### Connection Model

- The **agent** listens on a configurable TCP port (default `9876`).
- The **presenter client** connects to the agent's IP:port.
- The protocol is a simple length-prefixed message format:
  - `[4 bytes: message length (u32 big-endian)] [N bytes: JSON payload]`
- Message types:
  - **Client → Agent**: `Execute { actions: [...] }` — run this action block
  - **Agent → Client**: `Ack { status: "ok" | "error", message: Option<String> }` — action block completed
  - **Client → Agent**: `Ping` — keepalive
  - **Agent → Client**: `Pong` — keepalive response
- On disconnect, the client shows "Connection lost — reconnecting..." in the TUI and retries every second. Script position is preserved client-side.
- On reconnect, the client does **not** re-execute the last block (the presenter decides whether to re-trigger by hitting Enter again).

### Why Both Machines Need the Script

The **presenter client** needs the script to:
- Display `[SAY]` narration cues in the TUI
- Show upcoming actions for context
- Track progress (section names, block count)

The **agent** needs the script to:
- Validate that incoming `Execute` messages reference valid actions
- Know front-matter settings (typing_speed, typing_variance)
- Optionally: allow the agent to be started first and "pre-loaded" with the script, so the presenter can connect later

**Alternative considered**: Only the client has the script and sends full action details over TCP. This is simpler but means the agent is a dumb relay with no ability to validate or pre-configure. We chose script-on-both-sides for robustness.

**Practical workflow**: The presenter copies the `.cm` file to the demo machine before the talk (via `scp`, USB, shared folder, etc.).

## Script Format

The script format is a plain-text, line-based DSL. Lines are directives or narration.

```
# Comments start with hash
# Front matter (optional)
---
title: Error Handling in Rust
typing_speed: 40  # milliseconds per character
---

## Section: Introduction

[SAY] Welcome to today's talk on error handling in Rust.
[SAY] Let me show you what happens with unwrap.

[FOCUS] Terminal
[TYPE] cat src/main.rs
[RUN]
[PAUSE]

[SAY] Notice the unwrap on line 12. Let's run it with bad input.

[TYPE] cargo run -- --invalid
[RUN]
[PAUSE 3]

[SAY] That panic is what we want to eliminate.

[FOCUS] Keynote
[SLIDE next]

[SAY] Here's the better approach using the question mark operator.

[FOCUS] Terminal
[TYPE] cat src/main_v2.rs
[RUN]
[PAUSE]

[CLEAR]
[TYPE] cargo run -- --invalid
[RUN]
[PAUSE]

## Section: Error Types

[SLIDE next]
[SAY] Now let's look at custom error types.
```

### Directives

| Directive | Description | Executed By |
|-----------|-------------|-------------|
| `[SAY] text` | Narration cue displayed to presenter. | Client (display only) |
| `[TYPE] text` | Simulate typing into the focused app, character by character. | Agent |
| `[RUN]` | Press Enter in the focused application. | Agent |
| `[PAUSE]` | Wait for presenter to hit Enter before continuing. | Client (blocks) |
| `[PAUSE N]` | Wait N seconds, then auto-continue. | Client (timer) |
| `[FOCUS] AppName` | Bring the named application to the foreground. | Agent |
| `[SLIDE next]` | Advance to the next slide. | Agent |
| `[SLIDE prev]` | Go to the previous slide. | Agent |
| `[SLIDE N]` | Go to slide number N. | Agent |
| `[KEY combo]` | Send a keystroke combo (e.g., `cmd+s`, `ctrl+c`). | Agent |
| `[CLEAR]` | Send Ctrl+L to the terminal. | Agent |
| `[WAIT N]` | Silently wait N seconds. | Agent |
| `[EXEC command]` | Run a shell command in the background. | Agent |
| `## Section: name` | Section header — displayed in TUI progress. | Client (display only) |
| `# comment` | Ignored. | Neither |
| `---` ... `---` | Front matter block (key: value pairs). | Both (config) |

### Front Matter

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `title` | string | filename | Presentation title shown in TUI |
| `typing_speed` | integer | 40 | Milliseconds per character for `[TYPE]` |
| `typing_variance` | integer | 15 | Random variance in typing speed (ms) for natural feel |
| `agent_port` | integer | 9876 | TCP port the agent listens on |

### Grouping Rules

Consecutive non-`[SAY]`/non-`[PAUSE]` directives are grouped into an **action block**. When the presenter hits Enter, the entire next action block is sent to the agent and executes sequentially. `[SAY]` lines accumulate and display together until the next action block. `[PAUSE]` is always a standalone block that requires Enter (or times out).

Example grouping:
```
[SAY] Let me show you the code.     # ← narration, displayed on client
[FOCUS] Terminal                      # ┐
[TYPE] cat src/main.rs                # │ action block 1 (sent to agent)
[RUN]                                 # ┘
[PAUSE]                               # ← client waits for Enter
[SAY] Now let's run it.              # ← narration, displayed on client
[TYPE] cargo run                      # ┐ action block 2 (sent to agent)
[RUN]                                 # ┘
```

## Acceptance Criteria (as test descriptions)

### Parser
1. `test_parse_empty_script` — When an empty file is parsed, an empty script with default front matter is returned.
2. `test_parse_comments_only` — When a script contains only comments, the parsed script has no directives.
3. `test_parse_front_matter` — When a script has `---`-delimited front matter, `title` and `typing_speed` are extracted correctly.
4. `test_parse_say_directive` — When `[SAY] Hello world` is parsed, a `Say("Hello world")` directive is produced.
5. `test_parse_type_directive` — When `[TYPE] cargo build` is parsed, a `Type("cargo build")` directive is produced.
6. `test_parse_run_directive` — When `[RUN]` is parsed, a `Run` directive is produced.
7. `test_parse_pause_no_arg` — When `[PAUSE]` is parsed, a `Pause(None)` directive is produced.
8. `test_parse_pause_with_seconds` — When `[PAUSE 3]` is parsed, a `Pause(Some(3))` directive is produced.
9. `test_parse_focus_directive` — When `[FOCUS] Keynote` is parsed, a `Focus("Keynote")` directive is produced.
10. `test_parse_slide_next` — When `[SLIDE next]` is parsed, a `Slide(Next)` directive is produced.
11. `test_parse_slide_number` — When `[SLIDE 5]` is parsed, a `Slide(GoTo(5))` directive is produced.
12. `test_parse_key_directive` — When `[KEY cmd+s]` is parsed, a `Key("cmd+s")` directive is produced.
13. `test_parse_clear_directive` — When `[CLEAR]` is parsed, a `Clear` directive is produced.
14. `test_parse_wait_directive` — When `[WAIT 2]` is parsed, a `Wait(2)` directive is produced.
15. `test_parse_exec_directive` — When `[EXEC cargo build --release]` is parsed, an `Exec("cargo build --release")` directive is produced.
16. `test_parse_section_header` — When `## Section: Intro` is parsed, a `Section("Intro")` directive is produced.
17. `test_parse_unknown_directive_errors` — When `[BOGUS]` is parsed, a descriptive error with line number is returned.

### Grouper
18. `test_group_action_block` — Consecutive `[FOCUS]`, `[TYPE]`, `[RUN]` directives are grouped into a single action block.
19. `test_group_say_accumulates` — Multiple `[SAY]` lines before an action block are concatenated into one narration cue.
20. `test_group_pause_standalone` — `[PAUSE]` is always its own block, even between other directives.

### AppleScript Generation
21. `test_applescript_focus` — `focus_app_script("Terminal")` generates the correct AppleScript string.
22. `test_applescript_slide_next` — `slide_next_script()` generates the correct Keynote AppleScript string.
23. `test_applescript_keystroke` — `keystroke_script("cmd+s")` generates AppleScript with correct modifiers.
24. `test_typewriter_output` — `typewriter_to_applescript("hello", 0, 0)` produces 5 script+delay pairs.

### Network Protocol
25. `test_serialize_execute_message` — An `Execute` message with actions serializes to valid JSON.
26. `test_deserialize_ack_message` — A JSON ack payload deserializes to `Ack { status: "ok" }`.
27. `test_length_prefix_roundtrip` — A message encoded with length prefix decodes back to the same bytes.
28. `test_agent_executes_action_block` — Agent receives an `Execute` message and calls the corresponding AppleScript functions (tested with a mock executor).
29. `test_agent_returns_ack` — Agent sends an `Ack` after completing an action block.
30. `test_client_reconnects_on_disconnect` — When the TCP connection drops, the client enters reconnection mode and retries.

### CLI
31. `test_cli_present_subcommand` — `code-monkey present --agent 192.168.77.2:9876 demo.cm` parses correctly.
32. `test_cli_agent_subcommand` — `code-monkey agent demo.cm` parses correctly.
33. `test_cli_check_subcommand` — `code-monkey check demo.cm` parses correctly.
34. `test_cli_dry_run_flag` — `code-monkey present --dry-run demo.cm` activates dry-run mode.

### End-to-End
35. `test_dry_run_no_side_effects` — In dry-run mode, action blocks are listed but no TCP connection is made and osascript is never called.
36. `test_full_script_roundtrip` — A complete multi-section script parses, groups, and produces the expected sequence of blocks.

## Technical Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust | User preference; matches presenterm ecosystem |
| CLI framework | `clap` (derive) | Standard for Rust CLIs |
| Terminal UI | `ratatui` + `crossterm` | De facto standard for Rust TUIs |
| macOS automation | `osascript` via `std::process::Command` | Simple, no FFI, no extra deps |
| Network protocol | TCP with length-prefixed JSON | Simple, reliable, debuggable. JSON for human-readable wire format during development |
| Serialization | `serde` + `serde_json` | Standard for Rust JSON. Used only for the wire protocol — the script format remains a custom DSL |
| Simulated typing | AppleScript keystrokes with sleep | Typing happens on the agent machine via osascript |
| Script parser | Hand-rolled line-based parser | Format is simple line-based; nom/pest are overkill |
| Front matter | Hand-rolled key:value parser | Only need simple pairs; no need for a YAML crate |
| Error handling | `anyhow` for application errors, `thiserror` for library errors | Standard Rust practice |
| Typing speed jitter | Thread-local RNG via `fastrand` | Lightweight, no-dependency alternative to `rand` |
| Reconnection | Client-side retry loop, 1 second interval | Simple, no exponential backoff needed for a local cable |

## Design and Operation

### User Perspective

**Setup (before the talk):**
```bash
# On the demo machine — start the agent
$ code-monkey agent demo.cm
Listening on 0.0.0.0:9876...

# On the presenter laptop — connect and present
$ code-monkey present --agent 192.168.77.2:9876 demo.cm
Connected to agent at 192.168.77.2:9876
```

**Other commands:**
```bash
$ code-monkey check demo.cm              # Parse and validate only
$ code-monkey present --dry-run demo.cm   # Show blocks without connecting
```

**Recommended network setup:**
1. Connect an ethernet cable between the two machines.
2. On both machines, set a static IP on the ethernet interface:
   - Demo machine: `192.168.77.2/24`
   - Presenter laptop: `192.168.77.1/24`
3. Start the agent, then the presenter client.

When presenting, the presenter sees a TUI on their laptop:

```
┌─ Code Monkey ──────────────────── demo.cm ─┐
│                                             │
│  Section: Introduction          [3 / 27]    │
│  ● Connected to 192.168.77.2:9876           │
│                                             │
│  SAY:                                       │
│  Welcome to today's talk on error handling   │
│  in Rust. Let me show you what happens with │
│  unwrap.                                    │
│                                             │
│  NEXT ACTION:                               │
│  [FOCUS] Terminal                           │
│  [TYPE] cat src/main.rs                     │
│  [RUN]                                      │
│                                             │
│  Press Enter to execute  │  b back  │ q quit│
└─────────────────────────────────────────────┘
```

The audience sees only the demo machine's screen (projected), where Terminal, Keynote, etc. are being automated.

### System Perspective

**Agent (demo machine):**
1. Parse `.cm` script and extract front matter (for typing_speed, etc.).
2. Bind to TCP port and wait for client connection.
3. On receiving an `Execute` message:
   - Iterate through the action list.
   - Dispatch each directive to the corresponding AppleScript function.
   - Send `Ack { status: "ok" }` when done, or `Ack { status: "error", message }` on failure.
4. On disconnect, return to listening state (allow presenter to reconnect).

**Presenter client:**
1. Parse `.cm` script, group into action blocks.
2. Connect to agent via TCP.
3. Initialize TUI, display first block.
4. Event loop:
   - Show current `[SAY]` text and upcoming action block.
   - Wait for Enter keypress (or `q` to quit, `b` to go back).
   - On Enter: send `Execute { actions }` to agent, show "Executing..." in TUI, wait for `Ack`.
   - On `Ack`: advance to next block, update TUI.
   - On timeout (no `Ack` in 30s): show warning, allow retry.
   - For `Pause(Some(n))`: use `crossterm::event::poll(Duration)` with timeout.
5. On quit: close TCP connection, restore terminal.

### Error Handling

| Failure Mode | Handling |
|---|---|
| Script file not found | Exit with clear error message and path |
| Parse error (unknown directive) | Exit with error, line number, and the offending line |
| Agent: port already in use | Exit with error suggesting a different port |
| Client: can't connect to agent | Show error with IP:port, retry every second, show countdown in TUI |
| Client: connection drops mid-presentation | TUI shows "Reconnecting..." with retry count. Script position preserved. |
| Agent: osascript fails (app not running) | Return `Ack { status: "error", message }`. Client shows warning, continues. |
| Agent: osascript fails (no accessibility permission) | Return error ack with fix instructions. Client shows prominently. |
| Agent: EXEC command fails | Return error ack. Client shows warning, continues. |
| Client: no Ack within 30 seconds | TUI shows "Agent not responding — Enter to retry, s to skip" |
| User hits q | Client sends disconnect, restores terminal, exits cleanly |
| Panic | crossterm panic hook restores terminal first |

### Edge Cases

- Empty script: parse succeeds, TUI shows "No content" and exits.
- Script with only `[SAY]` lines and no actions: all narration displayed, Enter advances through narration blocks. No messages sent to agent.
- Consecutive `[PAUSE]` directives: each is a separate block requiring Enter.
- Very long `[TYPE]` text: typed across multiple seconds on the agent; client shows "Executing..." until ack.
- `[FOCUS]` to an app that isn't open: osascript may open it (macOS behavior); agent returns a warning in the ack.
- Agent started without client: agent waits indefinitely for connection. No timeout.
- Client started without agent: TUI shows "Connecting to 192.168.77.2:9876..." with retry.
- Script mismatch (different versions on client/agent): not validated in v1. Document as a user responsibility.

## Test Strategy

- **Unit tests**: Parser, grouper, front matter extraction, keystroke combo parsing, message serialization/deserialization. Pure functions — straightforward.
- **Integration tests**: Protocol roundtrip (spawn agent in a thread, connect client, send Execute, verify Ack). AppleScript string generation. Dry-run mode.
- **Network tests**: Use `127.0.0.1` with a random port. Test connection, disconnection, and reconnection.
- **No mocking of osascript**: The AppleScript execution layer is thin. We test script generation, not execution. Manual testing covers the execution path.
- **TUI testing**: Manual only.

## Rollback and Safety

This is a new standalone tool. No rollback concerns. The agent only sends keystrokes and activates apps — it does not modify files, databases, or system state. Worst case: a stray keystroke in the wrong app on the demo machine, mitigated by the presenter watching the projected screen.

## Implementation Stages

1. **Stage 1: Project skeleton + parser** — Cargo project, types, script parser, parser tests. Deliverable: `cargo test` passes with full parser coverage.
2. **Stage 2: Grouper + front matter** — Group directives into action blocks, parse front matter. Deliverable: `cargo test` covers grouping logic.
3. **Stage 3: macOS automation layer** — AppleScript generation functions for focus, type, run, slide, key, clear. Deliverable: unit tests on generated AppleScript strings.
4. **Stage 4: Network protocol + agent** — TCP server, message format, agent executor. Deliverable: agent starts, accepts connections, executes actions from test client.
5. **Stage 5: Presenter client + CLI** — TCP client, clap CLI with `agent`/`present`/`check` subcommands, dry-run mode. Deliverable: `code-monkey present --dry-run demo.cm` works.
6. **Stage 6: Presenter TUI** — ratatui-based interface showing narration, next action, progress, connection status. Enter/q/b key handling. Deliverable: full interactive two-machine presentation from a script.
