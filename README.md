# Code Monkey

Automated presentation assistant for live technical demos. A two-machine system where the presenter controls a TUI on their laptop while an agent on the demo machine executes AppleScript automation — typing code, running commands, switching apps, and advancing slides.

## Why

Presenting a live coding demo while narrating is hard. You fumble keystrokes, lose your place in the script, and the audience watches you fix typos instead of learning. Code Monkey separates the two jobs: you talk, it types.

## Architecture

```
Presenter's Laptop              Demo Machine (audience-visible)
┌──────────────┐    TCP/ethernet    ┌──────────────┐
│  TUI Client  │ ──────────────── │    Agent      │
│  (present)   │   length-prefixed  │  (AppleScript │
│              │   JSON messages    │   executor)  │
└──────────────┘                    └──────────────┘
```

- **Presenter** reads narration cues and presses Enter to trigger each step
- **Agent** receives action blocks over TCP and executes them via AppleScript/osascript
- Direct ethernet cable recommended for maximum reliability

## Script Format

Scripts use the `.cm` format with an optional YAML-like front matter and bracket directives:

```
---
title: My Demo
typing_speed: 40
typing_variance: 15
---

## Section: Setup

[SAY] First, let's create the project.

[FOCUS] Terminal
[TYPE] cargo new my-project
[RUN]
[PAUSE]

[SAY] Now open the editor.

[FOCUS] VS Code
[KEY] cmd+shift+p
[TYPE] rust-analyzer: Restart Server
[KEY] return
[WAIT 2]

[SLIDE next]

[SAY] That's it!
```

### Directives

| Directive | Description |
|-----------|-------------|
| `[SAY] text` | Narration shown to presenter (not executed on demo machine) |
| `[TYPE] text` | Simulated typing with realistic speed and jitter |
| `[RUN]` | Press Enter/Return |
| `[FOCUS] app` | Bring application to foreground |
| `[KEY] combo` | Keystroke with modifiers (e.g., `cmd+shift+s`, `ctrl+c`) |
| `[CLEAR]` | Clear terminal (Cmd+K) |
| `[PAUSE]` | Wait for presenter to press Enter |
| `[PAUSE 3]` | Auto-continue after 3 seconds |
| `[WAIT 2]` | Sleep for 2 seconds (non-interactive) |
| `[SLIDE next]` | Advance to next slide |
| `[SLIDE prev]` | Go to previous slide |
| `[SLIDE 5]` | Jump to slide 5 |
| `[EXEC] command` | Run a shell command on the demo machine |
| `## Section: name` | Section header shown in TUI title bar |

### Front Matter

| Key | Default | Description |
|-----|---------|-------------|
| `title` | none | Presentation title |
| `typing_speed` | 40 | Milliseconds per keystroke |
| `typing_variance` | 15 | Random jitter added to typing speed |
| `agent_port` | 9876 | Default agent TCP port |

## Usage

### Validate a script

```bash
code-monkey check script.cm
```

### Preview without executing (dry run)

```bash
code-monkey present --dry-run script.cm
```

### Start the agent on the demo machine

```bash
code-monkey agent script.cm --port 9876
```

### Run the presentation from your laptop

```bash
code-monkey present --agent 192.168.1.100:9876 script.cm
```

### TUI Controls

| Key | Action |
|-----|--------|
| Enter | Execute next action block |
| b | Go back one block |
| s | Skip current block |
| q | Quit |

## Building

Requires Rust 2024 edition (1.85+):

```bash
cargo build --release
```

The agent requires macOS (uses AppleScript/osascript for automation). The presenter client runs on any platform.

## Testing

```bash
cargo test
```

88 tests covering the parser, grouper, protocol, agent, client, and CLI.

## License

[MIT](LICENSE)
