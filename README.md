# AI Orchestrator

Personal/local Rust MVP for orchestrating multiple coding agents into one
decision loop.

The first version does not store AI account credentials. It calls already
installed and authenticated local CLIs:

- `claude` for draft and revision planning.
- `codex` for skeptical review and scope control.

This keeps the integration small, cross-platform, and safer than trying to
handle provider login sessions directly. For friends, the intended setup is that
each person uses their own local CLI login.

## Usage

## Terminal UI Stack

The interactive UI is a Rust TUI, not a shell-rendered screen:

- `ratatui` for layout, widgets, redraws, and resize-safe terminal rendering.
- `crossterm` for raw input, alternate screen, key events, and terminal control.
- `spec-duel` remains the orchestration engine used by the TUI.

## Project Layout

- `src/main.rs` wires modules and delegates startup.
- `src/model/` stores shared enums, command specs, effort constants, labels, language, provider, theme, and mode types.
- `src/app/` owns application state and workflows: config, chats, commands, editor history, effort, settings, worker events, footer state, onboarding, and model runs.
- `src/runtime.rs` handles terminal startup, event loop, and keyboard input.
- `src/ui/` renders the TUI screens and reusable widgets: command palette, prompt, footer, loader, transcript, onboarding, welcome, settings, and effort picker.
- `src/storage.rs` handles config, history, and saved chat files.
- `src/auth.rs` checks and launches local CLI authentication.
- `src/worker.rs` runs provider CLIs and the `spec-duel` engine.
- `src/input.rs` contains cursor and word-boundary helpers.

Current TUI shape:

- Claude Code-style welcome panel;
- selectable accent themes: purple, cyan, rose, amber, and mono;
- Russian interface by default;
- first-run setup wizard saved to `~/.duel/config`;
- persistent input history saved to `~/.duel/history`;
- saved chats stored under `~/.duel/chats`;
- startup opens the welcome screen instead of auto-restoring the last chat;
- saved chats can be reopened with `/chats` and `/resume <id>`;
- plain input sends a direct chat request to the selected direct-chat model;
- `/plan <task>` runs the multi-agent `spec-duel` planning loop;
- final brief content is printed back into the chat after each `/plan` run;
- bordered user messages in the transcript;
- bottom composer;
- animated loader while a model request is running, with stable text and token estimate;
- Codex `tokens used` output is parsed after direct chat responses when available;
- slash-command palette appears below the input when it starts with `/`, including Russian-layout command normalization;
- `/setup` reopens the setup wizard;
- `/plan <task>` runs spec-duel planning;
- `/settings` opens the settings screen;
- `/chat-model codex|claude` chooses who answers simple direct messages;
- `/theme purple|cyan|rose|amber|mono` changes the accent palette;
- `/roles <executor> <reviewer>` chooses the planning/code executor and reviewer;
- `/mode codex-only|claude-codex|codex-claude|claude-only` changes the orchestration pairing;
- `/new` starts a new saved chat;
- `/chats` lists saved chats;
- `/resume <id>` opens a saved chat;
- `/lang ru|en` switches interface language;
- `/effort` opens the effort picker;
- `Tab` completes the first visible slash command;
- `Up` / `Down` browse input history;
- `Alt+Left` / `Alt+Right` or `Ctrl+Left` / `Ctrl+Right` move by word;
- `Ctrl+A` / `Ctrl+E` move to line start/end;
- `Ctrl+Home` / `Ctrl+End` move to input start/end;
- `Ctrl+B` / `Ctrl+F` move one character left/right;
- `Ctrl+U` / `Ctrl+K` delete before/after cursor;
- `Ctrl+W` or `Alt+Backspace` deletes the previous word;
- `Alt+D` deletes the next word;
- `Ctrl+P` / `Ctrl+N` browse input history;
- `Ctrl+J` inserts a newline;
- `Enter` submits;
- `Esc` clears input;
- `Ctrl+C` exits.

Open the interactive Rust TUI:

```bash
duel
```

On first launch the TUI asks for the default agent pairing:

- `codex-only`;
- `claude-codex`;
- `codex-claude`;
- `claude-only`.

The setup screen checks `codex login status` and `claude auth status --text`.
It can launch `codex login` and `claude auth login`, then saves startup defaults
for language, review rounds, effort, output directory, mode, direct chat
provider, and theme.

The local launcher can be symlinked or copied into a directory on your `PATH`
as `duel`.

The launcher prefers the compiled Rust binary:

```bash
cd duel-cli
cargo build --release
duel
```

If Rust is not installed, direct one-shot task fallback still works through the
shell engine, but the interactive TUI will not open.

Run a task directly:

```bash
duel --codex-only "Build an affiliate program for a SaaS product"
```

Use the shell engine directly only when Rust is not installed yet:

```bash
./spec-duel "Build an affiliate program for a SaaS product"
```

If Claude Code subscription access is not enabled on the machine yet, use Codex
for both roles:

```bash
./spec-duel --codex-only "Build an affiliate program for a SaaS product"
```

Or pipe a larger task:

```bash
pbpaste | ./spec-duel
```

Useful shell engine flags:

```bash
./spec-duel --rounds 3 --out .ai-runs "Build an affiliate program"
./spec-duel --dry-run "Build an affiliate program"
./spec-duel --architect claude --reviewer codex "Build an affiliate program"
./spec-duel --codex-only --effort xhigh "Build an affiliate program"
./spec-duel --architect claude --reviewer claude --effort max "Build an affiliate program"
```

Effort is passed to the real provider CLI, not treated as a cosmetic UI value:

- Codex: `low|medium|high|xhigh` through `model_reasoning_effort`.
- Claude: `low|medium|high|max` through `--effort`.
- Claude+Codex mixed mode can use shared `low|medium|high`, or split settings:
  Claude `low|medium|high|max` and Codex `low|medium|high|xhigh`.

Rust TUI, once `cargo` is installed:

```bash
cargo run -- "Build an affiliate program for a SaaS product"
```

Or pipe a larger task:

```bash
pbpaste | cargo run --
```

Useful flags:

```bash
cargo run -- --rounds 3 --out .ai-runs "Build an affiliate program"
cargo run -- --dry-run "Build an affiliate program"
```

Each run creates a folder under `.ai-runs/` with the draft, review rounds, and
final decision brief.

## Requirements

- Rust toolchain with `cargo`.
- Claude Code CLI installed and authenticated.
- Codex CLI installed and authenticated.

The CLI executable names can be overridden:

```bash
AI_ORCHESTRATOR_CLAUDE=/path/to/claude \
AI_ORCHESTRATOR_CODEX=/path/to/codex \
cargo run -- "Plan a referral dashboard"
```

## MVP Boundary

This is intentionally a CLI-first orchestration kernel for personal use. A
future desktop app can wrap it with Tauri, add run history, approval screens,
provider profiles, prompt preset import/export, and a visual agent timeline.
