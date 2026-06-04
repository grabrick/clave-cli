use std::{
    collections::VecDeque,
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    style::force_color_output,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};

type AnyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Copy)]
struct CommandSpec {
    usage: &'static str,
    insert: &'static str,
    description_en: &'static str,
    description_ru: &'static str,
}

const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        usage: "/brainstorming",
        insert: "/brainstorming",
        description_en: "(superpowers) Explore options before creative work",
        description_ru: "(superpowers) Исследовать варианты перед творческой работой",
    },
    CommandSpec {
        usage: "/writing-plans",
        insert: "/writing-plans",
        description_en: "(superpowers) Turn a spec into a multi-step plan",
        description_ru: "(superpowers) Превратить спеку в пошаговый план",
    },
    CommandSpec {
        usage: "/finishing-a-development-branch",
        insert: "/finishing-a-development-branch",
        description_en: "(superpowers) Decide how to complete and polish work",
        description_ru: "(superpowers) Довести ветку разработки до завершения",
    },
    CommandSpec {
        usage: "/subagent-driven-development",
        insert: "/subagent-driven-development",
        description_en: "(superpowers) Split implementation across agents",
        description_ru: "(superpowers) Разделить реализацию между агентами",
    },
    CommandSpec {
        usage: "/using-git-worktrees",
        insert: "/using-git-worktrees",
        description_en: "(superpowers) Use isolated workspaces for parallel work",
        description_ru: "(superpowers) Использовать worktree для параллельной работы",
    },
    CommandSpec {
        usage: "/add-dir",
        insert: "/add-dir ",
        description_en: "Add a new working directory",
        description_ru: "Добавить рабочую директорию",
    },
    CommandSpec {
        usage: "/advisor",
        insert: "/advisor",
        description_en: "Consult a stronger model for guidance",
        description_ru: "Попросить сильную модель о совете",
    },
    CommandSpec {
        usage: "/agents",
        insert: "/agents",
        description_en: "Manage agent configurations",
        description_ru: "Управлять конфигурациями агентов",
    },
    CommandSpec {
        usage: "/autofix-pr",
        insert: "/autofix-pr",
        description_en: "Monitor and autofix issues with the current PR",
        description_ru: "Отслеживать и исправлять проблемы в текущем PR",
    },
    CommandSpec {
        usage: "/background",
        insert: "/background",
        description_en: "Send this session to the background",
        description_ru: "Отправить сессию в фон",
    },
    CommandSpec {
        usage: "/branch",
        insert: "/branch",
        description_en: "Create a branch of the current conversation",
        description_ru: "Создать ветку текущего разговора",
    },
    CommandSpec {
        usage: "/btw",
        insert: "/btw ",
        description_en: "Ask a quick side question",
        description_ru: "Задать быстрый побочный вопрос",
    },
    CommandSpec {
        usage: "/plan <task>",
        insert: "/plan ",
        description_en: "Run the multi-agent spec-duel planning loop",
        description_ru: "Запустить multi-agent планирование spec-duel",
    },
    CommandSpec {
        usage: "/clear",
        insert: "/clear",
        description_en: "Start fresh with empty context",
        description_ru: "Очистить контекст",
    },
    CommandSpec {
        usage: "/new",
        insert: "/new",
        description_en: "Start a new saved chat",
        description_ru: "Начать новый сохранённый чат",
    },
    CommandSpec {
        usage: "/chats",
        insert: "/chats",
        description_en: "Show saved chats",
        description_ru: "Показать сохранённые чаты",
    },
    CommandSpec {
        usage: "/resume <id>",
        insert: "/resume ",
        description_en: "Resume a saved chat",
        description_ru: "Открыть сохранённый чат",
    },
    CommandSpec {
        usage: "/color",
        insert: "/color ",
        description_en: "Set the prompt bar color",
        description_ru: "Изменить цвет строки ввода",
    },
    CommandSpec {
        usage: "/effort",
        insert: "/effort",
        description_en: "Adjust model effort level",
        description_ru: "Настроить уровень усилия модели",
    },
    CommandSpec {
        usage: "/logout",
        insert: "/logout",
        description_en: "Return to CLI authentication",
        description_ru: "Вернуться к авторизации CLI",
    },
    CommandSpec {
        usage: "/help",
        insert: "/help",
        description_en: "Show available commands",
        description_ru: "Показать доступные команды",
    },
    CommandSpec {
        usage: "/lang ru|en",
        insert: "/lang ",
        description_en: "Switch interface language",
        description_ru: "Переключить язык интерфейса",
    },
    CommandSpec {
        usage: "/mode codex-only",
        insert: "/mode codex-only",
        description_en: "Use Codex for both architect and reviewer",
        description_ru: "Использовать Codex как архитектора и ревьюера",
    },
    CommandSpec {
        usage: "/mode claude-only",
        insert: "/mode claude-only",
        description_en: "Use Claude for both architect and reviewer",
        description_ru: "Использовать Claude как архитектора и ревьюера",
    },
    CommandSpec {
        usage: "/mode claude-codex",
        insert: "/mode claude-codex",
        description_en: "Use Claude as architect and Codex as reviewer",
        description_ru: "Использовать Claude как архитектора и Codex как ревьюера",
    },
    CommandSpec {
        usage: "/rounds <n>",
        insert: "/rounds ",
        description_en: "Set review loop limit",
        description_ru: "Задать лимит раундов ревью",
    },
    CommandSpec {
        usage: "/out <dir>",
        insert: "/out ",
        description_en: "Set artifact directory",
        description_ru: "Задать папку артефактов",
    },
    CommandSpec {
        usage: "/status",
        insert: "/status",
        description_en: "Print session state",
        description_ru: "Показать состояние сессии",
    },
    CommandSpec {
        usage: "/setup",
        insert: "/setup",
        description_en: "Open first-run setup again",
        description_ru: "Открыть стартовую настройку заново",
    },
    CommandSpec {
        usage: "/quit",
        insert: "/quit",
        description_en: "Exit Duel",
        description_ru: "Выйти из Duel",
    },
];

const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
const CODEX_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh"];
const CLAUDE_EFFORTS: &[&str] = &["low", "medium", "high", "max"];
const COMMON_EFFORTS: &[&str] = &["low", "medium", "high"];
const ACCENT: Color = Color::Indexed(141);
const ACCENT_SOFT: Color = Color::Indexed(183);
const ACCENT_DIM: Color = Color::Indexed(97);
const ACCENT_BG: Color = Color::Indexed(60);
const MUTED: Color = Color::Gray;
const MAX_TRANSCRIPT_LINES: usize = 700;
const MAX_HISTORY_LINES: usize = 200;
const CHAT_FILE_EXTENSION: &str = "duel";
const LOADER_PHRASES: &[&str] = &[
    "Spelunking",
    "Thinking",
    "Reading context",
    "Drafting",
    "Reviewing",
    "Polishing",
];
impl CommandSpec {
    fn description(self, lang: Language) -> &'static str {
        match lang {
            Language::En => self.description_en,
            Language::Ru => self.description_ru,
        }
    }
}

fn effort_label(index: usize) -> &'static str {
    EFFORTS.get(index).copied().unwrap_or("high")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    CodexOnly,
    ClaudeOnly,
    ClaudeCodex,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::CodexOnly => "codex-only",
            Mode::ClaudeOnly => "claude-only",
            Mode::ClaudeCodex => "claude-codex",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "codex-only" => Some(Mode::CodexOnly),
            "claude-only" => Some(Mode::ClaudeOnly),
            "claude-codex" => Some(Mode::ClaudeCodex),
            _ => None,
        }
    }

    fn needs_codex(self) -> bool {
        matches!(self, Mode::CodexOnly | Mode::ClaudeCodex)
    }

    fn needs_claude(self) -> bool {
        matches!(self, Mode::ClaudeOnly | Mode::ClaudeCodex)
    }
}

fn provider_supports_effort(provider: &str, effort: &str) -> bool {
    match provider {
        "codex" => matches!(effort, "low" | "medium" | "high" | "xhigh"),
        "claude" => matches!(effort, "low" | "medium" | "high" | "max"),
        _ => false,
    }
}

fn provider_allowed_efforts(provider: &str) -> &'static [&'static str] {
    match provider {
        "codex" => CODEX_EFFORTS,
        "claude" => CLAUDE_EFFORTS,
        _ => COMMON_EFFORTS,
    }
}

fn effort_index_for(effort: &str) -> usize {
    EFFORTS
        .iter()
        .position(|value| *value == effort)
        .unwrap_or(2)
}

fn normalize_effort_index_for(allowed: &[&str], index: usize) -> usize {
    let effort = effort_label(index);
    if allowed.iter().any(|allowed| *allowed == effort) {
        index
    } else {
        effort_index_for("high")
    }
}

fn normalize_common_effort_index(index: usize) -> usize {
    normalize_effort_index_for(COMMON_EFFORTS, index)
}

fn normalize_provider_effort_index(provider: &str, index: usize) -> usize {
    normalize_effort_index_for(provider_allowed_efforts(provider), index)
}

fn move_effort_index_in(allowed: &[&str], index: usize, direction: isize) -> usize {
    let normalized = normalize_effort_index_for(allowed, index);
    let current = effort_label(normalized);
    let current_pos = allowed
        .iter()
        .position(|effort| *effort == current)
        .unwrap_or_else(|| {
            allowed
                .iter()
                .position(|effort| *effort == "high")
                .unwrap_or(0)
        });
    let next_pos = if direction < 0 {
        current_pos.saturating_sub(1)
    } else {
        (current_pos + 1).min(allowed.len().saturating_sub(1))
    };
    effort_index_for(allowed[next_pos])
}

fn move_common_effort_index(index: usize, direction: isize) -> usize {
    move_effort_index_in(COMMON_EFFORTS, index, direction)
}

fn move_provider_effort_index(provider: &str, index: usize, direction: isize) -> usize {
    move_effort_index_in(provider_allowed_efforts(provider), index, direction)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Language {
    Ru,
    En,
}

impl Language {
    fn as_str(self) -> &'static str {
        match self {
            Language::Ru => "ru",
            Language::En => "en",
        }
    }

    fn choose(self, ru: &'static str, en: &'static str) -> &'static str {
        match self {
            Language::Ru => ru,
            Language::En => en,
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "ru" => Some(Language::Ru),
            "en" => Some(Language::En),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct AppConfig {
    onboarding_done: bool,
    mode: Mode,
    lang: Language,
    rounds: usize,
    out_dir: String,
    effort_index: usize,
    codex_effort_index: usize,
    claude_effort_index: usize,
    linked_effort_split: bool,
    last_chat_id: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            onboarding_done: false,
            mode: Mode::CodexOnly,
            lang: Language::Ru,
            rounds: 2,
            out_dir: ".ai-runs".to_string(),
            effort_index: 3,
            codex_effort_index: 3,
            claude_effort_index: 4,
            linked_effort_split: true,
            last_chat_id: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OnboardingStep {
    Provider,
    Auth,
    Settings,
}

struct Onboarding {
    step: OnboardingStep,
    provider_index: usize,
    setting_index: usize,
    codex_installed: bool,
    claude_installed: bool,
    codex_authenticated: bool,
    claude_authenticated: bool,
    codex_status: String,
    claude_status: String,
    message: String,
}

impl Onboarding {
    fn new(mode: Mode) -> Self {
        let codex = codex_auth_probe();
        let claude = claude_auth_probe();

        Self {
            step: OnboardingStep::Provider,
            provider_index: provider_index(mode),
            setting_index: 0,
            codex_installed: codex.installed,
            claude_installed: claude.installed,
            codex_authenticated: codex.authenticated,
            claude_authenticated: claude.authenticated,
            codex_status: codex.status,
            claude_status: claude.status,
            message: "Выбери, какие модели будут работать в Duel.".to_string(),
        }
    }

    fn refresh_auth(&mut self) {
        let codex = codex_auth_probe();
        let claude = claude_auth_probe();
        self.codex_installed = codex.installed;
        self.claude_installed = claude.installed;
        self.codex_authenticated = codex.authenticated;
        self.claude_authenticated = claude.authenticated;
        self.codex_status = codex.status;
        self.claude_status = claude.status;
    }
}

struct AuthProbe {
    installed: bool,
    authenticated: bool,
    status: String,
}

#[derive(Debug)]
enum WorkerEvent {
    Line(String),
    ChatLine(String),
    Done(i32),
    ChatDone(&'static str, i32),
    Cancelled,
    Failed(String),
}

enum ChatRunResult {
    Completed(i32, String, String),
    Cancelled,
}

struct RevealLine {
    text: String,
    visible_chars: usize,
    transcript_index: usize,
    last_tick: Instant,
}

#[derive(Clone, Copy)]
struct EffortSnapshot {
    effort_index: usize,
    codex_effort_index: usize,
    claude_effort_index: usize,
    linked_effort_split: bool,
    effort_focus: usize,
}

struct ExternalCommand {
    program: &'static str,
    args: &'static [&'static str],
    label_ru: &'static str,
    label_en: &'static str,
}

struct App {
    mode: Mode,
    lang: Language,
    rounds: usize,
    out_dir: String,
    config_path: PathBuf,
    history_path: PathBuf,
    chats_dir: PathBuf,
    chat_id: String,
    chat_path: PathBuf,
    onboarding: Option<Onboarding>,
    pending_external: Option<ExternalCommand>,
    input: String,
    cursor: usize,
    transcript: Vec<String>,
    reveal_queue: VecDeque<String>,
    reveal_active: Option<RevealLine>,
    status: String,
    last_run: Option<String>,
    running: bool,
    run_started_at: Option<Instant>,
    run_label: String,
    run_token_estimate: Option<usize>,
    cancel_tx: Option<Sender<()>>,
    last_ctrl_c_at: Option<Instant>,
    footer_notice: Option<(String, Instant)>,
    footer_right_text: String,
    footer_right_previous_text: Option<String>,
    footer_right_changed_at: Option<Instant>,
    should_quit: bool,
    history: Vec<String>,
    history_index: Option<usize>,
    selected_suggestion: usize,
    command_palette_opened_at: Option<Instant>,
    command_palette_query: String,
    effort_picker: bool,
    effort_original: Option<EffortSnapshot>,
    effort_focus: usize,
    effort_index: usize,
    codex_effort_index: usize,
    claude_effort_index: usize,
    linked_effort_split: bool,
    tx: Sender<WorkerEvent>,
    rx: Receiver<WorkerEvent>,
}

impl App {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let config_path = config_path();
        let history_path = history_path();
        let chats_dir = chats_dir();
        let mut config = load_config(&config_path);
        if env::var("DUEL_SKIP_ONBOARDING").ok().as_deref() == Some("1") {
            config.onboarding_done = true;
        }
        config.effort_index = normalize_common_effort_index(config.effort_index);
        config.codex_effort_index =
            normalize_provider_effort_index("codex", config.codex_effort_index);
        config.claude_effort_index =
            normalize_provider_effort_index("claude", config.claude_effort_index);
        let onboarding = if config.onboarding_done {
            None
        } else {
            Some(Onboarding::new(config.mode))
        };

        let (chat_id, chat_path, transcript) =
            restore_or_create_chat(&chats_dir, None, config.lang);
        let history = load_history(&history_path).unwrap_or_default();
        let last_run = None;

        Self {
            mode: config.mode,
            lang: config.lang,
            rounds: config.rounds,
            out_dir: config.out_dir,
            config_path,
            history_path,
            chats_dir,
            chat_id,
            chat_path,
            onboarding,
            pending_external: None,
            input: String::new(),
            cursor: 0,
            transcript,
            reveal_queue: VecDeque::new(),
            reveal_active: None,
            status: config.lang.choose("готов", "ready").to_string(),
            last_run,
            running: false,
            run_started_at: None,
            run_label: String::new(),
            run_token_estimate: None,
            cancel_tx: None,
            last_ctrl_c_at: None,
            footer_notice: None,
            footer_right_text: String::new(),
            footer_right_previous_text: None,
            footer_right_changed_at: None,
            should_quit: false,
            history,
            history_index: None,
            selected_suggestion: 0,
            command_palette_opened_at: None,
            command_palette_query: String::new(),
            effort_picker: false,
            effort_original: None,
            effort_focus: 0,
            effort_index: config.effort_index,
            codex_effort_index: config.codex_effort_index,
            claude_effort_index: config.claude_effort_index,
            linked_effort_split: config.linked_effort_split,
            tx,
            rx,
        }
    }

    fn current_config(&self, onboarding_done: bool) -> AppConfig {
        AppConfig {
            onboarding_done,
            mode: self.mode,
            lang: self.lang,
            rounds: self.rounds,
            out_dir: self.out_dir.clone(),
            effort_index: self.effort_index,
            codex_effort_index: self.codex_effort_index,
            claude_effort_index: self.claude_effort_index,
            linked_effort_split: self.linked_effort_split,
            last_chat_id: Some(self.chat_id.clone()),
        }
    }

    fn save_current_config(&mut self, onboarding_done: bool) {
        if let Err(err) = save_config(&self.config_path, &self.current_config(onboarding_done)) {
            self.push_system(format!(
                "{} {}",
                self.lang
                    .choose("Не удалось сохранить конфиг:", "Failed to save config:"),
                err
            ));
        }
    }

    fn remember_history_entry(&mut self, line: &str) {
        self.history.retain(|entry| entry != line);
        self.history.push(line.to_string());
        if self.history.len() > MAX_HISTORY_LINES {
            let remove_count = self.history.len() - MAX_HISTORY_LINES;
            self.history.drain(0..remove_count);
        }

        if let Err(err) = save_history(&self.history_path, &self.history) {
            self.status = self
                .lang
                .choose("ошибка истории", "history error")
                .to_string();
            self.transcript.push(format!(
                "{} {}",
                self.lang
                    .choose("Не удалось сохранить историю:", "Failed to save history:"),
                err
            ));
        }
    }

    fn start_new_chat(&mut self) {
        self.chat_id = new_chat_id();
        self.chat_path = chat_path_for_id(&self.chats_dir, &self.chat_id);
        self.transcript.clear();
        self.last_run = None;
        self.status = self.lang.choose("новый чат", "new chat").to_string();

        if let Err(err) = save_chat_transcript(&self.chat_path, &self.chat_id, &self.transcript) {
            self.transcript.push(format!(
                "{} {}",
                self.lang.choose(
                    "Не удалось создать файл чата:",
                    "Failed to create chat file:"
                ),
                err
            ));
        }

        self.save_current_config(true);
        self.push_system(format!(
            "{} {}",
            self.lang.choose("Новый чат:", "New chat:"),
            self.chat_id
        ));
    }

    fn resume_chat(&mut self, chat_id: &str) {
        let chat_id = sanitize_chat_id(chat_id);
        if chat_id.is_empty() {
            self.push_system(self.lang.choose(
                "Использование: /resume <id-чата>",
                "Usage: /resume <chat-id>",
            ));
            return;
        }

        let path = chat_path_for_id(&self.chats_dir, &chat_id);
        match load_chat_transcript(&path) {
            Ok(lines) if !lines.is_empty() => {
                self.chat_id = chat_id;
                self.chat_path = path;
                self.transcript = lines;
                self.last_run = find_last_run(&self.transcript);
                self.status = self.lang.choose("чат открыт", "chat resumed").to_string();
                self.save_current_config(true);
                self.push_system(format!(
                    "{} {}",
                    self.lang.choose("Чат открыт:", "Chat resumed:"),
                    self.chat_id
                ));
            }
            Ok(_) => self.push_system(
                self.lang
                    .choose("Чат пустой или повреждён.", "Chat is empty or corrupted."),
            ),
            Err(err) => self.push_system(format!(
                "{} {}",
                self.lang
                    .choose("Не удалось открыть чат:", "Failed to open chat:"),
                err
            )),
        }
    }

    fn show_saved_chats(&mut self) {
        let chats = list_saved_chats(&self.chats_dir, 12);
        if chats.is_empty() {
            self.push_system(
                self.lang
                    .choose("Сохранённых чатов пока нет.", "No saved chats yet."),
            );
            return;
        }

        self.push_system(self.lang.choose("Сохранённые чаты:", "Saved chats:"));
        for chat in chats {
            let marker = if chat.id == self.chat_id { "●" } else { " " };
            self.push_system(format!(
                "{} {} · {} · {}",
                marker, chat.id, chat.lines, chat.title
            ));
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.effort_focus = 0;
        self.effort_index = normalize_common_effort_index(self.effort_index);
        self.codex_effort_index = normalize_provider_effort_index("codex", self.codex_effort_index);
        self.claude_effort_index =
            normalize_provider_effort_index("claude", self.claude_effort_index);
    }

    fn effort_summary(&self) -> String {
        match self.mode {
            Mode::CodexOnly => format!("codex {}", effort_label(self.codex_effort_index)),
            Mode::ClaudeOnly => format!("claude {}", effort_label(self.claude_effort_index)),
            Mode::ClaudeCodex if self.linked_effort_split => format!(
                "claude {} · codex {}",
                effort_label(self.claude_effort_index),
                effort_label(self.codex_effort_index)
            ),
            Mode::ClaudeCodex => format!("shared {}", effort_label(self.effort_index)),
        }
    }

    fn compact_effort_summary(&self) -> String {
        match self.mode {
            Mode::CodexOnly => effort_label(self.codex_effort_index).to_string(),
            Mode::ClaudeOnly => effort_label(self.claude_effort_index).to_string(),
            Mode::ClaudeCodex if self.linked_effort_split => format!(
                "cl:{} cd:{}",
                effort_label(self.claude_effort_index),
                effort_label(self.codex_effort_index)
            ),
            Mode::ClaudeCodex => effort_label(self.effort_index).to_string(),
        }
    }

    fn provider_effort(&self, provider: &str) -> &'static str {
        if self.mode == Mode::ClaudeCodex && !self.linked_effort_split {
            return effort_label(self.effort_index);
        }

        match provider {
            "claude" => effort_label(self.claude_effort_index),
            "codex" => effort_label(self.codex_effort_index),
            _ => effort_label(self.effort_index),
        }
    }

    fn active_effort_for_tokens(&self) -> &'static str {
        match self.mode {
            Mode::CodexOnly => effort_label(self.codex_effort_index),
            Mode::ClaudeOnly => effort_label(self.claude_effort_index),
            Mode::ClaudeCodex if self.linked_effort_split => {
                let claude = effort_label(self.claude_effort_index);
                let codex = effort_label(self.codex_effort_index);
                if effort_weight(claude) >= effort_weight(codex) {
                    claude
                } else {
                    codex
                }
            }
            Mode::ClaudeCodex => effort_label(self.effort_index),
        }
    }

    fn effort_snapshot(&self) -> EffortSnapshot {
        EffortSnapshot {
            effort_index: self.effort_index,
            codex_effort_index: self.codex_effort_index,
            claude_effort_index: self.claude_effort_index,
            linked_effort_split: self.linked_effort_split,
            effort_focus: self.effort_focus,
        }
    }

    fn restore_effort_snapshot(&mut self, snapshot: EffortSnapshot) {
        self.effort_index = snapshot.effort_index;
        self.codex_effort_index = snapshot.codex_effort_index;
        self.claude_effort_index = snapshot.claude_effort_index;
        self.linked_effort_split = snapshot.linked_effort_split;
        self.effort_focus = snapshot.effort_focus;
    }

    fn effort_picker_rows(&self) -> usize {
        match self.mode {
            Mode::ClaudeCodex if self.linked_effort_split => 3,
            Mode::ClaudeCodex => 2,
            _ => 1,
        }
    }

    fn adjust_effort_focus(&mut self, direction: isize) {
        match self.mode {
            Mode::CodexOnly => {
                self.codex_effort_index =
                    move_provider_effort_index("codex", self.codex_effort_index, direction);
            }
            Mode::ClaudeOnly => {
                self.claude_effort_index =
                    move_provider_effort_index("claude", self.claude_effort_index, direction);
            }
            Mode::ClaudeCodex => match self.effort_focus {
                0 => {
                    self.linked_effort_split = !self.linked_effort_split;
                    self.effort_index = normalize_common_effort_index(self.effort_index);
                    if self.effort_focus >= self.effort_picker_rows() {
                        self.effort_focus = self.effort_picker_rows().saturating_sub(1);
                    }
                }
                1 if self.linked_effort_split => {
                    self.claude_effort_index =
                        move_provider_effort_index("claude", self.claude_effort_index, direction);
                }
                2 if self.linked_effort_split => {
                    self.codex_effort_index =
                        move_provider_effort_index("codex", self.codex_effort_index, direction);
                }
                1 => {
                    self.effort_index = move_common_effort_index(self.effort_index, direction);
                }
                _ => {}
            },
        }
    }

    fn adjust_startup_effort(&mut self, direction: isize) {
        match self.mode {
            Mode::CodexOnly => {
                self.codex_effort_index =
                    move_provider_effort_index("codex", self.codex_effort_index, direction);
            }
            Mode::ClaudeOnly => {
                self.claude_effort_index =
                    move_provider_effort_index("claude", self.claude_effort_index, direction);
            }
            Mode::ClaudeCodex if self.linked_effort_split => {
                self.linked_effort_split = false;
                self.effort_index = normalize_common_effort_index(self.effort_index);
            }
            Mode::ClaudeCodex => {
                self.effort_index = move_common_effort_index(self.effort_index, direction);
            }
        }
    }

    fn drain_worker_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                WorkerEvent::Line(line) => {
                    if let Some(path) = line.strip_prefix("Final brief: ") {
                        let path = path.to_string();
                        self.last_run = Some(path.clone());
                        self.push_system(line);
                        self.push_final_brief(&path);
                    } else {
                        self.push_system(line);
                    }
                }
                WorkerEvent::ChatLine(line) => self.enqueue_reveal(line),
                WorkerEvent::Done(code) => {
                    self.running = false;
                    self.run_started_at = None;
                    self.run_label.clear();
                    self.run_token_estimate = None;
                    self.cancel_tx = None;
                    self.status = if code == 0 {
                        self.lang.choose("готово", "completed").to_string()
                    } else {
                        format!("{}:{code}", self.lang.choose("ошибка", "failed"))
                    };
                    self.push_system(format!(
                        "{} {code}.",
                        self.lang
                            .choose("Duel завершился с кодом", "Duel finished with exit code")
                    ));
                }
                WorkerEvent::ChatDone(provider, code) => {
                    self.running = false;
                    self.run_started_at = None;
                    self.run_label.clear();
                    self.run_token_estimate = None;
                    self.cancel_tx = None;
                    self.status = if code == 0 {
                        self.lang.choose("готово", "completed").to_string()
                    } else {
                        format!("{}:{code}", self.lang.choose("ошибка", "failed"))
                    };
                    if code != 0 {
                        self.push_system(format!(
                            "{} {} {}.",
                            provider_display(provider, self.lang),
                            self.lang
                                .choose("завершился с кодом", "finished with exit code"),
                            code
                        ));
                    }
                }
                WorkerEvent::Cancelled => {
                    self.running = false;
                    self.run_started_at = None;
                    self.run_label.clear();
                    self.run_token_estimate = None;
                    self.cancel_tx = None;
                    self.status = self.lang.choose("остановлено", "stopped").to_string();
                    self.push_system(
                        self.lang
                            .choose("⏹ Выполнение остановлено.", "⏹ Run stopped."),
                    );
                }
                WorkerEvent::Failed(message) => {
                    self.running = false;
                    self.run_started_at = None;
                    self.run_label.clear();
                    self.run_token_estimate = None;
                    self.cancel_tx = None;
                    self.status = self.lang.choose("ошибка", "failed").to_string();
                    self.push_system(message);
                }
            }
        }
    }

    fn push_final_brief(&mut self, path: &str) {
        match final_brief_lines_for_chat(path, self.lang) {
            Ok(lines) => {
                self.push_system(self.lang.choose("⏺ Итоговый ответ", "⏺ Final answer"));
                for line in lines {
                    self.push_system(line);
                }
            }
            Err(err) => self.push_system(format!(
                "{} {}",
                self.lang.choose(
                    "Не удалось прочитать итоговый ответ:",
                    "Failed to read final answer:"
                ),
                err
            )),
        }
    }

    fn push_system(&mut self, line: impl Into<String>) {
        let line = line.into();
        if let Err(err) = append_chat_line(&self.chat_path, &line) {
            self.status = self.lang.choose("ошибка чата", "chat error").to_string();
            self.transcript.push(format!(
                "{} {}",
                self.lang
                    .choose("Не удалось сохранить чат:", "Failed to save chat:"),
                err
            ));
        }

        self.transcript.push(line);
        if self.transcript.len() > MAX_TRANSCRIPT_LINES {
            let remove_count = self.transcript.len() - MAX_TRANSCRIPT_LINES;
            self.transcript.drain(0..remove_count);
        }
    }

    fn enqueue_reveal(&mut self, line: impl Into<String>) {
        self.reveal_queue.push_back(line.into());
    }

    fn advance_reveal(&mut self) {
        if self.reveal_active.is_none() {
            self.start_next_reveal_line();
        }

        let Some(active) = self.reveal_active.as_mut() else {
            return;
        };

        let elapsed = active.last_tick.elapsed();
        if elapsed < Duration::from_millis(18) {
            return;
        }

        let total_chars = active.text.chars().count();
        let steps = (elapsed.as_millis() / 18).max(1) as usize;
        active.visible_chars = active
            .visible_chars
            .saturating_add(steps * 3)
            .min(total_chars);
        active.last_tick = Instant::now();

        let visible = prefix_chars(&active.text, active.visible_chars);
        if active.transcript_index < self.transcript.len() {
            self.transcript[active.transcript_index] = visible;
        }

        if active.visible_chars >= total_chars {
            let text = active.text.clone();
            self.reveal_active = None;
            if let Err(err) = append_chat_line(&self.chat_path, &text) {
                self.status = self.lang.choose("ошибка чата", "chat error").to_string();
                self.transcript.push(format!(
                    "{} {}",
                    self.lang
                        .choose("Не удалось сохранить чат:", "Failed to save chat:"),
                    err
                ));
            }
            self.start_next_reveal_line();
        }
    }

    fn start_next_reveal_line(&mut self) {
        let Some(text) = self.reveal_queue.pop_front() else {
            return;
        };

        let transcript_index = self.transcript.len();
        self.transcript.push(String::new());
        if self.transcript.len() > MAX_TRANSCRIPT_LINES {
            let remove_count = self.transcript.len() - MAX_TRANSCRIPT_LINES;
            self.transcript.drain(0..remove_count);
        }
        let transcript_index = transcript_index.min(self.transcript.len().saturating_sub(1));
        self.reveal_active = Some(RevealLine {
            text,
            visible_chars: 0,
            transcript_index,
            last_tick: Instant::now(),
        });
    }

    fn open_auth_screen(&mut self, message: String, force_next_start: bool) {
        let mut onboarding = Onboarding::new(self.mode);
        onboarding.step = OnboardingStep::Auth;
        onboarding.message = message;
        self.onboarding = Some(onboarding);
        self.status = self.lang.choose("авторизация", "auth").to_string();
        if force_next_start {
            self.save_current_config(false);
        }
    }

    fn ensure_auth_ready_for_current_mode(&mut self) -> bool {
        let onboarding = Onboarding::new(self.mode);
        if auth_requirements_ready(self.mode, &onboarding) {
            return true;
        }

        let missing = missing_auth_text(self.mode, &onboarding, self.lang);
        let message = format!(
            "{} {}. {}",
            self.lang
                .choose("Для режима нужен логин:", "Login required for mode:"),
            missing,
            self.lang.choose(
                "Нажми C для Codex login или L для Claude auth login.",
                "Press C for Codex login or L for Claude auth login."
            )
        );
        self.open_auth_screen(message.clone(), true);
        self.show_footer_notice(message);
        false
    }

    fn suggestions(&self) -> Vec<CommandSpec> {
        if !self.input.starts_with('/') {
            return Vec::new();
        }

        let needle = self.input.trim();
        COMMANDS
            .iter()
            .copied()
            .filter(|command| {
                command.usage.starts_with(needle) || command.insert.starts_with(needle)
            })
            .collect()
    }

    fn complete_command(&mut self) {
        let suggestions = self.suggestions();
        if suggestions.is_empty() {
            return;
        }

        let index = self.selected_suggestion.min(suggestions.len() - 1);
        if let Some(suggestion) = suggestions.get(index).copied() {
            self.input = suggestion.insert.to_string();
            self.cursor = self.input.len();
        }
    }

    fn submit_input(&mut self) {
        let line = self.input.trim().to_string();
        self.input.clear();
        self.cursor = 0;
        self.history_index = None;

        if line.is_empty() {
            return;
        }

        self.remember_history_entry(&line);

        if line.eq_ignore_ascii_case("logout") {
            self.push_command_invocation(&line);
            self.push_command_result(self.lang.choose("Auth screen", "Auth screen"));
            self.open_auth_screen(
                self.lang
                    .choose(
                        "Проверь авторизацию CLI. Можно запустить Codex или Claude login.",
                        "Check CLI authentication. You can run Codex or Claude login.",
                    )
                    .to_string(),
                true,
            );
        } else if line.starts_with('/') {
            self.handle_command(&line);
        } else {
            self.start_chat(line);
        }
    }

    fn handle_command(&mut self, line: &str) {
        let mut parts = line.split_whitespace();
        let command = parts.next().unwrap_or_default();
        let rest = parts.collect::<Vec<_>>().join(" ");

        match command {
            "/help" => {
                self.push_system(self.lang.choose("⏺ Команды", "⏺ Commands"));
                for command in COMMANDS {
                    self.push_system(format!(
                        "  ⎿ {:<22} {}",
                        command.usage,
                        command.description(self.lang)
                    ));
                }
                self.status = self.lang.choose("помощь", "help").to_string();
            }
            "/lang" | "/language" => match rest.as_str() {
                "ru" | "рус" | "russian" => {
                    self.lang = Language::Ru;
                    self.status = "язык:ru".to_string();
                    self.save_current_config(true);
                    self.push_system("Язык интерфейса изменён на русский.");
                }
                "en" | "eng" | "english" => {
                    self.lang = Language::En;
                    self.status = "lang:en".to_string();
                    self.save_current_config(true);
                    self.push_system("Interface language changed to English.");
                }
                _ => self.push_system(
                    self.lang
                        .choose("Использование: /lang ru|en", "Usage: /lang ru|en"),
                ),
            },
            "/mode" => match rest.as_str() {
                "codex-only" => self.apply_mode(Mode::CodexOnly),
                "claude-only" => self.apply_mode(Mode::ClaudeOnly),
                "claude-codex" => self.apply_mode(Mode::ClaudeCodex),
                _ => self.push_system(self.lang.choose(
                    "Использование: /mode codex-only|claude-only|claude-codex",
                    "Usage: /mode codex-only|claude-only|claude-codex",
                )),
            },
            "/plan" | "/duel" => {
                if rest.trim().is_empty() {
                    self.push_system(
                        self.lang
                            .choose("Использование: /plan <задача>", "Usage: /plan <task>"),
                    );
                } else {
                    self.start_task(rest.trim().to_string());
                }
            }
            "/rounds" => match rest.parse::<usize>() {
                Ok(value) if value > 0 => {
                    self.rounds = value;
                    self.status = format!("rounds:{value}");
                    self.save_current_config(true);
                    self.push_system(format!(
                        "{} {value}.",
                        self.lang.choose("Количество раундов:", "Rounds set to")
                    ));
                }
                _ => self.push_system(self.lang.choose(
                    "Использование: /rounds <положительное-число>",
                    "Usage: /rounds <positive-number>",
                )),
            },
            "/out" => {
                if rest.trim().is_empty() {
                    self.push_system(
                        self.lang
                            .choose("Использование: /out <папка>", "Usage: /out <directory>"),
                    );
                } else {
                    self.out_dir = rest;
                    self.status = self
                        .lang
                        .choose("папка обновлена", "out updated")
                        .to_string();
                    self.save_current_config(true);
                    self.push_system(format!(
                        "{} {}.",
                        self.lang.choose("Папка артефактов:", "Output directory:"),
                        self.out_dir
                    ));
                }
            }
            "/status" => {
                self.push_system(format!(
                    "{}={} {}={} {}={} {}={} {}={}",
                    self.lang.choose("режим", "mode"),
                    self.mode.as_str(),
                    self.lang.choose("язык", "lang"),
                    self.lang.as_str(),
                    self.lang.choose("раунды", "rounds"),
                    self.rounds,
                    "effort",
                    self.effort_summary(),
                    "out",
                    self.out_dir
                ));
            }
            "/effort" => {
                self.push_command_invocation(line);
                self.effort_original = Some(self.effort_snapshot());
                self.effort_focus = 0;
                self.effort_picker = true;
                self.status = "effort".to_string();
            }
            "/logout" | "/auth" => {
                self.push_command_invocation(line);
                self.push_command_result(self.lang.choose("Auth screen", "Auth screen"));
                self.open_auth_screen(
                    self.lang
                        .choose(
                            "Проверь авторизацию CLI. Можно запустить Codex или Claude login.",
                            "Check CLI authentication. You can run Codex or Claude login.",
                        )
                        .to_string(),
                    true,
                );
            }
            "/setup" => {
                self.onboarding = Some(Onboarding::new(self.mode));
                self.status = self.lang.choose("настройка", "setup").to_string();
            }
            "/new" => self.start_new_chat(),
            "/chats" => self.show_saved_chats(),
            "/resume" => {
                if rest.trim().is_empty() {
                    self.push_system(self.lang.choose(
                        "Использование: /resume <id-чата>",
                        "Usage: /resume <chat-id>",
                    ));
                } else {
                    self.resume_chat(rest.trim());
                }
            }
            "/clear" => {
                self.transcript.clear();
                self.push_system(self.lang.choose("Лента очищена.", "Transcript cleared."));
            }
            "/quit" | "/exit" => self.should_quit = true,
            _ => self.push_system(format!(
                "{} {command}",
                self.lang.choose("Неизвестная команда:", "Unknown command:")
            )),
        }
    }

    fn apply_mode(&mut self, mode: Mode) {
        self.set_mode(mode);
        self.status = format!("mode:{}", self.mode.as_str());
        self.save_current_config(true);
        self.push_system(format!(
            "{} {}.",
            self.lang.choose("Режим изменён на", "Mode changed to"),
            self.mode.as_str()
        ));
        self.ensure_auth_ready_for_current_mode();
    }

    fn start_chat(&mut self, message: String) {
        if self.running {
            self.push_system(
                self.lang
                    .choose("Duel уже выполняется.", "A duel is already running."),
            );
            return;
        }

        if !self.ensure_auth_ready_for_current_mode() {
            return;
        }

        let provider = chat_provider(self.mode);
        let provider_name = provider_display(provider, self.lang);
        let effort = self.provider_effort(provider).to_string();
        let context = recent_chat_context(&self.transcript, 40);
        let lang = self.lang;
        let prompt = chat_prompt(&message, &context, lang);
        let token_estimate = estimate_tokens(&prompt);
        let work_dir = engine_path()
            .map(|engine| engine_work_dir(&engine))
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let (cancel_tx, cancel_rx) = mpsc::channel();

        self.running = true;
        self.run_started_at = Some(Instant::now());
        self.run_label = provider_name.to_string();
        self.run_token_estimate = Some(token_estimate);
        self.cancel_tx = Some(cancel_tx);
        self.last_ctrl_c_at = None;
        self.status = format!("{}...", provider_name.to_lowercase());
        self.push_system(format!("◆ {message}"));
        let tx = self.tx.clone();
        thread::spawn(move || {
            let command_result =
                run_chat_provider(provider, &effort, &prompt, &work_dir, cancel_rx);

            match command_result {
                Ok(ChatRunResult::Completed(code, stdout, stderr)) => {
                    let stdout = stdout.trim();
                    let stderr = stderr.trim();

                    if !stdout.is_empty() {
                        emit_chat_lines(&tx, stdout);
                    } else if code == 0 {
                        let _ = tx.send(WorkerEvent::Line(format!(
                            "{}",
                            lang.choose(
                                "Модель не вернула текстовый ответ.",
                                "The model returned no text response."
                            )
                        )));
                    } else {
                        let _ = tx.send(WorkerEvent::Line(format!(
                            "{} {}:",
                            provider_display(provider, lang),
                            lang.choose("вернул ошибку", "returned an error")
                        )));
                        emit_error_lines(&tx, stderr);
                    }

                    let _ = tx.send(WorkerEvent::ChatDone(provider, code));
                }
                Ok(ChatRunResult::Cancelled) => {
                    let _ = tx.send(WorkerEvent::Cancelled);
                }
                Err(err) => {
                    let _ = tx.send(WorkerEvent::Failed(format!(
                        "{}: {}",
                        provider_display(provider, lang),
                        err
                    )));
                }
            }
        });
    }

    fn start_task(&mut self, task: String) {
        if self.running {
            self.push_system(
                self.lang
                    .choose("Duel уже выполняется.", "A duel is already running."),
            );
            return;
        }

        if !self.ensure_auth_ready_for_current_mode() {
            return;
        }

        let engine = match engine_path() {
            Some(path) => path,
            None => {
                self.status = "engine missing".to_string();
                self.push_system(self.lang.choose(
                    "spec-duel не найден. Задай DUEL_ENGINE или запусти из корня проекта.",
                    "spec-duel engine not found. Set DUEL_ENGINE or run from project root.",
                ));
                return;
            }
        };
        let (cancel_tx, cancel_rx) = mpsc::channel();

        self.running = true;
        self.run_started_at = Some(Instant::now());
        self.run_label = "spec-duel".to_string();
        self.run_token_estimate = Some(estimate_tokens(&task));
        self.cancel_tx = Some(cancel_tx);
        self.last_ctrl_c_at = None;
        self.status = self.lang.choose("запущено", "running").to_string();
        self.push_system(format!("◆ {task}"));
        self.push_system(format!(
            "{} {} {} {} · effort {}.",
            self.lang.choose("⏺ Запускаю режим", "⏺ Running"),
            self.mode.as_str(),
            self.lang.choose("на раундов:", "with round(s):"),
            self.rounds,
            self.effort_summary()
        ));

        let tx = self.tx.clone();
        let mode = self.mode;
        let rounds = self.rounds.to_string();
        let out_dir = self.out_dir.clone();
        let common_effort = effort_label(self.effort_index).to_string();
        let architect_effort = match mode {
            Mode::CodexOnly => self.provider_effort("codex").to_string(),
            Mode::ClaudeOnly | Mode::ClaudeCodex => self.provider_effort("claude").to_string(),
        };
        let reviewer_effort = match mode {
            Mode::ClaudeOnly => self.provider_effort("claude").to_string(),
            Mode::CodexOnly | Mode::ClaudeCodex => self.provider_effort("codex").to_string(),
        };
        let work_dir = engine_work_dir(&engine);
        let work_dir_arg = work_dir.to_string_lossy().to_string();

        thread::spawn(move || {
            let mut args = Vec::new();

            match mode {
                Mode::CodexOnly => args.push("--codex-only".to_string()),
                Mode::ClaudeOnly => {
                    args.extend([
                        "--architect".to_string(),
                        "claude".to_string(),
                        "--reviewer".to_string(),
                        "claude".to_string(),
                    ]);
                }
                Mode::ClaudeCodex => {
                    args.extend([
                        "--architect".to_string(),
                        "claude".to_string(),
                        "--reviewer".to_string(),
                        "codex".to_string(),
                    ]);
                }
            }

            args.extend([
                "--cwd".to_string(),
                work_dir_arg,
                "--rounds".to_string(),
                rounds,
                "--out".to_string(),
                out_dir,
                "--effort".to_string(),
                common_effort,
                "--architect-effort".to_string(),
                architect_effort,
                "--reviewer-effort".to_string(),
                reviewer_effort,
                task,
            ]);

            let mut child = match Command::new(&engine)
                .current_dir(&work_dir)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(err) => {
                    let _ = tx.send(WorkerEvent::Failed(format!(
                        "Failed to spawn {}: {err}",
                        engine.display()
                    )));
                    return;
                }
            };

            if let Some(stdout) = child.stdout.take() {
                spawn_reader(stdout, tx.clone());
            }

            if let Some(stderr) = child.stderr.take() {
                spawn_reader(stderr, tx.clone());
            }

            loop {
                if cancel_rx.try_recv().is_ok() {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = tx.send(WorkerEvent::Cancelled);
                    return;
                }

                match child.try_wait() {
                    Ok(Some(status)) => {
                        let _ = tx.send(WorkerEvent::Done(status.code().unwrap_or(1)));
                        return;
                    }
                    Ok(None) => thread::sleep(Duration::from_millis(80)),
                    Err(err) => {
                        let _ = tx.send(WorkerEvent::Failed(format!("Wait failed: {err}")));
                        return;
                    }
                }
            }
        });
    }

    fn insert_char(&mut self, ch: char) {
        self.input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let prev = previous_boundary(&self.input, self.cursor);
        self.input.drain(prev..self.cursor);
        self.cursor = prev;
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    fn delete(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }

        let next = next_boundary(&self.input, self.cursor);
        self.input.drain(self.cursor..next);
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    fn move_left(&mut self) {
        self.cursor = previous_boundary(&self.input, self.cursor);
    }

    fn move_right(&mut self) {
        self.cursor = next_boundary(&self.input, self.cursor);
    }

    fn move_word_left(&mut self) {
        self.cursor = previous_word_boundary(&self.input, self.cursor);
    }

    fn move_word_right(&mut self) {
        self.cursor = next_word_boundary(&self.input, self.cursor);
    }

    fn move_line_start(&mut self) {
        self.cursor = line_start_boundary(&self.input, self.cursor);
    }

    fn move_line_end(&mut self) {
        self.cursor = line_end_boundary(&self.input, self.cursor);
    }

    fn delete_word_back(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = previous_word_boundary(&self.input, self.cursor);
        self.input.drain(start..self.cursor);
        self.cursor = start;
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    fn delete_word_forward(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        let end = next_word_boundary(&self.input, self.cursor);
        self.input.drain(self.cursor..end);
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    fn kill_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.input.drain(..self.cursor);
        self.cursor = 0;
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    fn kill_after_cursor(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        self.input.drain(self.cursor..);
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    fn history_prev(&mut self) {
        if self.history_index.is_none() {
            let suggestions = self.suggestions();
            if !suggestions.is_empty() {
                self.selected_suggestion = self.selected_suggestion.saturating_sub(1);
                return;
            }
        }

        if self.history.is_empty() {
            return;
        }

        let next_index = match self.history_index {
            Some(index) => index.saturating_sub(1),
            None => self.history.len() - 1,
        };
        self.history_index = Some(next_index);
        self.input = self.history[next_index].clone();
        self.cursor = self.input.len();
    }

    fn history_next(&mut self) {
        if self.history_index.is_none() {
            let suggestions = self.suggestions();
            if !suggestions.is_empty() {
                self.selected_suggestion =
                    (self.selected_suggestion + 1).min(suggestions.len() - 1);
                return;
            }
        }

        let Some(index) = self.history_index else {
            return;
        };

        if index + 1 >= self.history.len() {
            self.history_index = None;
            self.input.clear();
        } else {
            let next_index = index + 1;
            self.history_index = Some(next_index);
            self.input = self.history[next_index].clone();
        }
        self.cursor = self.input.len();
    }

    fn push_command_invocation(&mut self, command: &str) {
        self.push_system(format!("❯ {command}"));
    }

    fn push_command_result(&mut self, result: impl Into<String>) {
        self.push_system(format!("  ⎿  {}", result.into()));
    }

    fn show_footer_notice(&mut self, message: impl Into<String>) {
        self.footer_notice = Some((message.into(), Instant::now()));
    }

    fn expire_footer_notice(&mut self) {
        let expired = self
            .footer_notice
            .as_ref()
            .map(|(_, shown_at)| shown_at.elapsed() > Duration::from_secs(2))
            .unwrap_or(false);

        if expired {
            self.footer_notice = None;
            if self.status == self.lang.choose("подтверди выход", "confirm exit") {
                self.status = self.lang.choose("готов", "ready").to_string();
            }
        }
    }

    fn refresh_command_palette_state(&mut self) {
        let active =
            self.input.starts_with('/') && self.onboarding.is_none() && !self.effort_picker;
        if active {
            if self.command_palette_opened_at.is_none() || self.command_palette_query != self.input
            {
                self.command_palette_opened_at = Some(Instant::now());
                self.command_palette_query = self.input.clone();
            }
        } else if self.command_palette_opened_at.is_some() {
            self.command_palette_opened_at = None;
            self.command_palette_query.clear();
        }
    }

    fn refresh_footer_right_state(&mut self) {
        let next = footer_right_target(self);
        if self.footer_right_text.is_empty() {
            self.footer_right_text = next;
            return;
        }

        if self.footer_right_text != next {
            self.footer_right_previous_text = Some(self.footer_right_text.clone());
            self.footer_right_text = next;
            self.footer_right_changed_at = Some(Instant::now());
            return;
        }

        let transition_done = self
            .footer_right_changed_at
            .map(|changed_at| changed_at.elapsed() > Duration::from_millis(820))
            .unwrap_or(false);
        if transition_done {
            self.footer_right_previous_text = None;
            self.footer_right_changed_at = None;
        }
    }

    fn handle_ctrl_c(&mut self) {
        let now = Instant::now();
        let is_double = self
            .last_ctrl_c_at
            .map(|previous| now.duration_since(previous) <= Duration::from_secs(2))
            .unwrap_or(false);
        self.last_ctrl_c_at = Some(now);

        if is_double {
            if let Some(cancel_tx) = self.cancel_tx.take() {
                let _ = cancel_tx.send(());
            }
            self.should_quit = true;
            return;
        }

        if self.running {
            if let Some(cancel_tx) = self.cancel_tx.take() {
                let _ = cancel_tx.send(());
            }
            self.status = self.lang.choose("остановка", "stopping").to_string();
            self.show_footer_notice(self.lang.choose(
                "Останавливаю выполнение. Ctrl+C ещё раз в течение 2 секунд — выйти.",
                "Stopping the run. Press Ctrl+C again within 2 seconds to exit.",
            ));
        } else {
            self.status = self
                .lang
                .choose("подтверди выход", "confirm exit")
                .to_string();
            self.show_footer_notice(self.lang.choose(
                "Нажми Ctrl+C ещё раз в течение 2 секунд, чтобы выйти.",
                "Press Ctrl+C again within 2 seconds to exit.",
            ));
        }
    }
}

fn main() -> AnyResult<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_usage();
        return Ok(());
    }

    if !args.is_empty() {
        return run_engine_direct(args);
    }

    run_tui()
}

fn print_usage() {
    println!(
        "duel\n\nUsage:\n  duel                 Open TUI\n  duel <task...>       Run task directly through spec-duel\n  duel --help          Show help\n"
    );
}

fn run_engine_direct(args: Vec<String>) -> AnyResult<()> {
    let engine = engine_path().ok_or("spec-duel engine not found")?;
    let work_dir = engine_work_dir(&engine);
    let status = Command::new(&engine)
        .current_dir(work_dir)
        .args(args)
        .status()?;
    std::process::exit(status.code().unwrap_or(1));
}

fn run_tui() -> AnyResult<()> {
    force_color_output(true);
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> AnyResult<()> {
    let mut app = App::new();

    loop {
        app.drain_worker_events();
        app.advance_reveal();
        app.expire_footer_notice();
        app.refresh_command_palette_state();
        app.refresh_footer_right_state();
        terminal.draw(|frame| draw(frame, &app))?;

        if app.should_quit {
            return Ok(());
        }

        if event::poll(Duration::from_millis(80))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(&mut app, key),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if let Some(command) = app.pending_external.take() {
            let label = app
                .lang
                .choose(command.label_ru, command.label_en)
                .to_string();
            let result = run_external_command(terminal, &command);
            match result {
                Ok(code) => {
                    let mode = app.mode;
                    let lang = app.lang;
                    if let Some(onboarding) = app.onboarding.as_mut() {
                        onboarding.refresh_auth();
                        let ready = auth_requirements_ready(mode, onboarding);
                        onboarding.message = if ready {
                            onboarding.step = OnboardingStep::Settings;
                            lang.choose(
                                "Авторизация готова. Проверь стартовые настройки и нажми Enter.",
                                "Authentication is ready. Review startup settings and press Enter.",
                            )
                            .to_string()
                        } else if code == 0 {
                            lang.choose(
                                "Логин завершился. Статус обновлен, но нужные аккаунты еще не все готовы.",
                                "Login finished. Status updated, but not every required account is ready yet.",
                            ).to_string()
                        } else {
                            lang.choose(
                                "Команда логина завершилась с ошибкой. Проверь текст выше и повтори.",
                                "Login command failed. Check the text above and try again.",
                            ).to_string()
                        };
                    }
                    app.push_system(format!("{label}: exit {code}"));
                }
                Err(err) => app.push_system(format!("{label}: {err}")),
            }
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    if app.onboarding.is_some() {
        handle_onboarding_key(app, key);
        return;
    }

    if app.effort_picker {
        handle_effort_key(app, key);
        return;
    }

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    if ctrl {
        match key.code {
            KeyCode::Char('c') => app.handle_ctrl_c(),
            KeyCode::Char('j') => app.insert_newline(),
            KeyCode::Char('m') => app.submit_input(),
            KeyCode::Char('a') => app.move_line_start(),
            KeyCode::Char('e') => app.move_line_end(),
            KeyCode::Char('b') => app.move_left(),
            KeyCode::Char('f') => app.move_right(),
            KeyCode::Char('p') => app.history_prev(),
            KeyCode::Char('n') => app.history_next(),
            KeyCode::Char('u') => app.kill_before_cursor(),
            KeyCode::Char('k') => app.kill_after_cursor(),
            KeyCode::Char('w') => app.delete_word_back(),
            KeyCode::Char('d') => app.delete(),
            KeyCode::Left => app.move_word_left(),
            KeyCode::Right => app.move_word_right(),
            KeyCode::Backspace => app.delete_word_back(),
            KeyCode::Delete => app.delete_word_forward(),
            KeyCode::Home => app.cursor = 0,
            KeyCode::End => app.cursor = app.input.len(),
            _ => {}
        }
        return;
    }

    if alt {
        match key.code {
            KeyCode::Left | KeyCode::Char('b') => app.move_word_left(),
            KeyCode::Right | KeyCode::Char('f') => app.move_word_right(),
            KeyCode::Backspace => app.delete_word_back(),
            KeyCode::Delete | KeyCode::Char('d') => app.delete_word_forward(),
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Enter => app.submit_input(),
        KeyCode::Tab => app.complete_command(),
        KeyCode::Backspace => app.backspace(),
        KeyCode::Delete => app.delete(),
        KeyCode::Left => app.move_left(),
        KeyCode::Right => app.move_right(),
        KeyCode::Up => app.history_prev(),
        KeyCode::Down => app.history_next(),
        KeyCode::Home => app.move_line_start(),
        KeyCode::End => app.move_line_end(),
        KeyCode::Esc => {
            app.input.clear();
            app.cursor = 0;
            app.history_index = None;
            app.selected_suggestion = 0;
        }
        KeyCode::Char(ch) if !ch.is_control() => app.insert_char(ch),
        _ => {}
    }
}

fn handle_onboarding_key(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        app.handle_ctrl_c();
        return;
    }

    let Some(step) = app.onboarding.as_ref().map(|onboarding| onboarding.step) else {
        return;
    };

    match step {
        OnboardingStep::Provider => handle_onboarding_provider_key(app, key),
        OnboardingStep::Auth => handle_onboarding_auth_key(app, key),
        OnboardingStep::Settings => handle_onboarding_settings_key(app, key),
    }
}

fn handle_onboarding_provider_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            let index = {
                let onboarding = app.onboarding.as_mut().expect("onboarding exists");
                onboarding.provider_index = onboarding.provider_index.saturating_sub(1);
                onboarding.provider_index
            };
            app.set_mode(provider_mode(index));
        }
        KeyCode::Down => {
            let index = {
                let onboarding = app.onboarding.as_mut().expect("onboarding exists");
                onboarding.provider_index =
                    (onboarding.provider_index + 1).min(provider_count() - 1);
                onboarding.provider_index
            };
            app.set_mode(provider_mode(index));
        }
        KeyCode::Enter => {
            let provider_index = app
                .onboarding
                .as_ref()
                .map(|onboarding| onboarding.provider_index);
            if let Some(provider_index) = provider_index {
                app.set_mode(provider_mode(provider_index));
            }
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.step = OnboardingStep::Auth;
                onboarding.message = app
                    .lang
                    .choose(
                        "Проверь авторизацию CLI. Можно запустить логин прямо отсюда.",
                        "Check CLI authentication. You can run login from here.",
                    )
                    .to_string();
            }
        }
        _ => {}
    }
}

fn handle_onboarding_auth_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.pending_external = Some(ExternalCommand {
                program: "codex",
                args: &["login"],
                label_ru: "Codex login",
                label_en: "Codex login",
            });
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            app.pending_external = Some(ExternalCommand {
                program: "claude",
                args: &["auth", "login"],
                label_ru: "Claude auth login",
                label_en: "Claude auth login",
            });
        }
        KeyCode::Enter => {
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.step = OnboardingStep::Settings;
                onboarding.message = app
                    .lang
                    .choose(
                        "Выставь стартовые настройки. Enter сохранит конфиг.",
                        "Choose startup defaults. Enter saves the config.",
                    )
                    .to_string();
            }
        }
        KeyCode::Backspace | KeyCode::Esc => {
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.step = OnboardingStep::Provider;
            }
        }
        _ => {}
    }
}

fn handle_onboarding_settings_key(app: &mut App, key: KeyEvent) {
    let setting_index = app
        .onboarding
        .as_ref()
        .map(|onboarding| onboarding.setting_index)
        .unwrap_or(0);

    match key.code {
        KeyCode::Up => {
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.setting_index = onboarding.setting_index.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.setting_index = (onboarding.setting_index + 1).min(2);
            }
        }
        KeyCode::Left => adjust_onboarding_setting(app, setting_index, -1),
        KeyCode::Right => adjust_onboarding_setting(app, setting_index, 1),
        KeyCode::Char('l') | KeyCode::Char('L') => {
            app.lang = if app.lang == Language::Ru {
                Language::En
            } else {
                Language::Ru
            };
        }
        KeyCode::Enter => {
            app.onboarding = None;
            app.status = app.lang.choose("готов", "ready").to_string();
            app.save_current_config(true);
        }
        KeyCode::Backspace | KeyCode::Esc => {
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.step = OnboardingStep::Auth;
            }
        }
        _ => {}
    }
}

fn adjust_onboarding_setting(app: &mut App, setting_index: usize, direction: isize) {
    match setting_index {
        0 => {
            if direction < 0 {
                app.rounds = app.rounds.saturating_sub(1).max(1);
            } else {
                app.rounds = (app.rounds + 1).min(9);
            }
        }
        1 => {
            app.adjust_startup_effort(direction);
        }
        2 => {
            app.lang = if app.lang == Language::Ru {
                Language::En
            } else {
                Language::Ru
            };
        }
        _ => {}
    }
}

fn handle_effort_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => app.effort_focus = app.effort_focus.saturating_sub(1),
        KeyCode::Down => {
            app.effort_focus = (app.effort_focus + 1).min(app.effort_picker_rows() - 1);
        }
        KeyCode::Left => app.adjust_effort_focus(-1),
        KeyCode::Right => app.adjust_effort_focus(1),
        KeyCode::Enter => {
            app.effort_picker = false;
            app.effort_original = None;
            app.status = app.lang.choose("готов", "ready").to_string();
            app.save_current_config(true);
            app.push_command_result(format!("Set to {}", app.effort_summary()));
        }
        KeyCode::Esc => {
            if let Some(snapshot) = app.effort_original.take() {
                app.restore_effort_snapshot(snapshot);
            }
            app.effort_picker = false;
            app.status = app.lang.choose("готов", "ready").to_string();
            app.push_command_result("Cancelled");
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.handle_ctrl_c();
        }
        _ => {}
    }
}

fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    if app.onboarding.is_some() {
        draw_onboarding(frame, area, app);
        return;
    }

    if app.effort_picker {
        draw_effort_screen(frame, area, app);
        return;
    }

    let command_mode = app.input.starts_with('/');
    let composer_height = composer_height(app, area.width).min(area.height.saturating_sub(2));
    let palette_height = if command_mode {
        command_palette_height(app, area.height, composer_height)
    } else {
        0
    };
    let footer_height = if command_mode { 0 } else { 1 };
    let output_gap = if app.transcript.is_empty() && !app.running && app.last_run.is_none() {
        0
    } else {
        1
    };
    let palette_gap = if command_mode { 1 } else { 0 };
    let main_height = main_area_height(
        app,
        area,
        composer_height,
        palette_height,
        footer_height,
        output_gap,
        palette_gap,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(main_height),
            Constraint::Length(output_gap),
            Constraint::Length(composer_height),
            Constraint::Length(palette_gap),
            Constraint::Length(palette_height),
            Constraint::Length(footer_height),
            Constraint::Min(0),
        ])
        .split(area);

    draw_main_area(frame, chunks[0], app);
    draw_prompt_bar(frame, chunks[2], app);
    if command_mode {
        draw_command_screen(frame, chunks[4], app);
    } else {
        draw_footer(frame, chunks[5], app);
    }
}

fn draw_main_area(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if app.transcript.is_empty() && !app.running && app.last_run.is_none() {
        draw_welcome(frame, area, app);
    } else {
        draw_transcript(frame, area, app);
    }
}

fn draw_transcript(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let visible = area.height.saturating_sub(1) as usize;
    let mut lines = vec![Line::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(ACCENT_DIM),
    )];
    for line in &app.transcript {
        lines.extend(transcript_entry_lines(line, app.lang, area.width));
    }

    if app.running {
        lines.push(Line::from(""));
        lines.push(loader_line(app));
    }

    let start = lines.len().saturating_sub(visible);
    let lines = lines[start..].to_vec();

    let transcript = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(transcript, area);
}

fn transcript_entry_lines(line: &str, lang: Language, width: u16) -> Vec<Line<'static>> {
    if let Some(message) = line.strip_prefix("◆ ") {
        return user_message_box(message, lang, width);
    }

    wrap_terminal_line(line, width)
        .into_iter()
        .map(|wrapped| style_transcript_line(&wrapped, lang))
        .collect()
}

fn user_message_box(message: &str, lang: Language, width: u16) -> Vec<Line<'static>> {
    let width = width as usize;
    if width < 12 {
        return vec![Line::styled(
            format!("{} {}", lang.choose("Ты", "You"), message),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )];
    }

    let label = format!(" {} ", lang.choose("Ты", "You"));
    let content_width = width.saturating_sub(4).max(8);
    let horizontal_width = content_width + 2;
    let mut lines = Vec::new();
    let top_tail = "─".repeat(horizontal_width.saturating_sub(label.chars().count()));
    lines.push(Line::styled(
        format!("╭{label}{top_tail}╮"),
        Style::default().fg(ACCENT),
    ));

    for wrapped in wrap_chars(message, content_width) {
        let padding = content_width.saturating_sub(wrapped.chars().count());
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(ACCENT)),
            Span::styled(
                wrapped,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ".repeat(padding)),
            Span::styled(" │", Style::default().fg(ACCENT)),
        ]));
    }

    lines.push(Line::styled(
        format!("╰{}╯", "─".repeat(horizontal_width)),
        Style::default().fg(ACCENT),
    ));
    lines
}

fn style_transcript_line(line: &str, lang: Language) -> Line<'static> {
    if line.starts_with("◆ ") {
        Line::from(vec![
            Span::styled(
                lang.choose("Ты", "You"),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(line.trim_start_matches("◆ ").to_string()),
        ])
    } else if let Some(command) = line.strip_prefix("❯ ") {
        Line::from(vec![
            Span::styled(
                "❯ ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(command.to_string()),
        ])
    } else if line.starts_with("Final brief: ") {
        Line::from(vec![
            Span::styled(
                "⏺ brief ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.trim_start_matches("Final brief: ").to_string()),
        ])
    } else if line.contains("error") || line.contains("failed") || line.contains("Failed") {
        Line::styled(line.to_string(), Style::default().fg(Color::Red))
    } else if line.starts_with("Drafting")
        || line.starts_with("Review")
        || line.starts_with("Revision")
    {
        Line::from(vec![
            Span::styled(
                "⏺ ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.to_string()),
        ])
    } else if line.starts_with("⎿ ") || line.trim_start().starts_with('⎿') {
        Line::styled(line.to_string(), Style::default().fg(Color::DarkGray))
    } else if let Some(rest) = line.strip_prefix("⏺ ") {
        Line::from(vec![
            Span::styled(
                "⏺ ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(rest.to_string()),
        ])
    } else if line.starts_with("✻ ") || line.starts_with("✦ ") {
        Line::styled(
            line.to_string(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )
    } else {
        Line::from(line.to_string())
    }
}

fn centered_line(text: impl Into<String>, width: u16, style: Style) -> Line<'static> {
    let text = text.into();
    let left_pad = (width as usize).saturating_sub(text.chars().count()) / 2;
    Line::from(vec![
        Span::raw(" ".repeat(left_pad)),
        Span::styled(text, style),
    ])
}

fn separator_line(width: u16) -> Line<'static> {
    Line::styled("─".repeat(width as usize), Style::default().fg(ACCENT_DIM))
}

fn draw_welcome(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let card_height = area.height.min(12);
    let card = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: card_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled(
                " Duel Code ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("v0.1.0 ", Style::default().fg(MUTED)),
        ]))
        .border_style(Style::default().fg(ACCENT));

    frame.render_widget(block, card);

    if card.width < 30 || card.height < 5 {
        return;
    }

    let inner = Rect {
        x: card.x + 2,
        y: card.y + 1,
        width: card.width.saturating_sub(4),
        height: card.height.saturating_sub(2),
    };

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(41), Constraint::Percentage(59)])
        .split(inner);

    let user = env::var("USER").unwrap_or_else(|_| "friend".to_string());
    let left_width = columns[0].width;
    let left = vec![
        centered_line(
            app.lang.choose("С возвращением!", "Welcome back!"),
            left_width,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        centered_line(user, left_width, Style::default().fg(MUTED)),
        Line::from(""),
        centered_line("╭──╮", left_width, Style::default().fg(ACCENT)),
        centered_line(
            "›──◆──‹",
            left_width,
            Style::default()
                .fg(ACCENT_SOFT)
                .add_modifier(Modifier::BOLD),
        ),
        centered_line("╰──╯", left_width, Style::default().fg(ACCENT)),
        Line::from(""),
        centered_line(
            format!("{} · {}", app.mode.as_str(), app.compact_effort_summary()),
            left_width,
            Style::default().fg(Color::DarkGray),
        ),
    ];

    frame.render_widget(Paragraph::new(left).wrap(Wrap { trim: false }), columns[0]);

    let right_width = columns[1].width;
    let right = vec![
        Line::from(Span::styled(
            app.lang.choose("Быстрый старт", "Tips for getting started"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(app.lang.choose(
            "Введи сообщение и нажми Enter",
            "Type a message and press Enter",
        )),
        Line::from(app.lang.choose(
            "Для спеки используй /plan <задача>",
            "Use /plan <task> for spec-duel planning",
        )),
        separator_line(right_width),
        Line::from(Span::styled(
            app.lang.choose("Что нового", "What's new"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(app.lang.choose(
            "Обычный ввод отвечает напрямую моделью",
            "Plain input chats directly with the model",
        )),
        separator_line(right_width),
        Line::from(Span::styled(
            "Overview",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "{} {} · effort {}",
            app.lang.choose("Режим", "Mode"),
            app.mode.as_str(),
            app.effort_summary()
        )),
        Line::from(app.lang.choose(
            "/chats · /new · /resume · /effort",
            "/chats · /new · /resume · /effort",
        )),
    ];

    frame.render_widget(Paragraph::new(right).wrap(Wrap { trim: false }), columns[1]);
}

fn draw_onboarding(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let Some(onboarding) = app.onboarding.as_ref() else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(1)])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled(
                " Duel Setup ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("first run ", Style::default().fg(MUTED)),
        ]))
        .border_style(Style::default().fg(ACCENT));
    frame.render_widget(block, chunks[0]);

    let inner = Rect {
        x: chunks[0].x + 2,
        y: chunks[0].y + 1,
        width: chunks[0].width.saturating_sub(4),
        height: chunks[0].height.saturating_sub(2),
    };

    let lines = match onboarding.step {
        OnboardingStep::Provider => onboarding_provider_lines(app, onboarding),
        OnboardingStep::Auth => onboarding_auth_lines(app, onboarding),
        OnboardingStep::Settings => onboarding_settings_lines(app, onboarding),
    };

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    draw_footer(frame, chunks[1], app);
}

fn onboarding_provider_lines(app: &App, onboarding: &Onboarding) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::styled(
            app.lang
                .choose("Выбор связки моделей", "Choose model pairing"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(onboarding.message.clone()),
        Line::from(""),
    ];

    for index in 0..provider_count() {
        let selected = index == onboarding.provider_index;
        let mode = provider_mode(index);
        let style = if selected {
            Style::default()
                .fg(Color::White)
                .bg(ACCENT_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ACCENT_SOFT)
        };
        lines.push(Line::from(vec![
            Span::styled(
                if selected { "› " } else { "  " },
                Style::default().fg(ACCENT),
            ),
            Span::styled(format!("{:<14}", mode.as_str()), style),
            Span::raw(" "),
            Span::styled(
                provider_description(mode, app.lang),
                Style::default().fg(MUTED),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        app.lang.choose(
            "↑/↓ выбрать · Enter продолжить · Ctrl+C выйти",
            "↑/↓ choose · Enter continue · Ctrl+C exit",
        ),
        Style::default().fg(MUTED),
    ));
    lines
}

fn onboarding_auth_lines(app: &App, onboarding: &Onboarding) -> Vec<Line<'static>> {
    let codex_needed = app.mode.needs_codex();
    let claude_needed = app.mode.needs_claude();
    vec![
        Line::styled(
            app.lang.choose("Авторизация CLI", "CLI authentication"),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Line::from(onboarding.message.clone()),
        Line::from(""),
        auth_status_line(
            "Codex",
            codex_needed,
            onboarding.codex_installed,
            onboarding.codex_authenticated,
            &onboarding.codex_status,
            "codex login",
            "C",
            app.lang,
        ),
        auth_status_line(
            "Claude",
            claude_needed,
            onboarding.claude_installed,
            onboarding.claude_authenticated,
            &onboarding.claude_status,
            "claude auth login",
            "L",
            app.lang,
        ),
        Line::from(""),
        Line::styled(
            app.lang.choose(
                "C запустить Codex login · L запустить Claude auth login · Enter дальше · Esc назад",
                "C run Codex login · L run Claude auth login · Enter next · Esc back",
            ),
            Style::default().fg(MUTED),
        ),
    ]
}

fn onboarding_settings_lines(app: &App, onboarding: &Onboarding) -> Vec<Line<'static>> {
    let rows = [
        (
            app.lang.choose("Раунды ревью", "Review rounds").to_string(),
            app.rounds.to_string(),
        ),
        ("Effort".to_string(), app.effort_summary()),
        (
            app.lang.choose("Язык", "Language").to_string(),
            app.lang.as_str().to_string(),
        ),
    ];

    let mut lines = vec![
        Line::styled(
            app.lang.choose("Стартовые настройки", "Startup settings"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(onboarding.message.clone()),
        Line::from(""),
    ];

    for (index, (label, value)) in rows.into_iter().enumerate() {
        let selected = index == onboarding.setting_index;
        let style = if selected {
            Style::default()
                .fg(Color::White)
                .bg(ACCENT_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ACCENT_SOFT)
        };
        lines.push(Line::from(vec![
            Span::styled(
                if selected { "› " } else { "  " },
                Style::default().fg(ACCENT),
            ),
            Span::styled(format!("{label:<18}"), style),
            Span::raw(" "),
            Span::styled(value, Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            app.lang.choose("Режим ", "Mode "),
            Style::default().fg(MUTED),
        ),
        Span::styled(
            app.mode.as_str(),
            Style::default()
                .fg(ACCENT_SOFT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.lang.choose(" · Артефакты ", " · Artifacts "),
            Style::default().fg(MUTED),
        ),
        Span::styled(app.out_dir.clone(), Style::default().fg(ACCENT_SOFT)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::styled(
        app.lang.choose(
            "↑/↓ поле · ←/→ изменить · L язык · Enter сохранить · Esc назад",
            "↑/↓ field · ←/→ change · L language · Enter save · Esc back",
        ),
        Style::default().fg(MUTED),
    ));
    lines
}

fn auth_status_line(
    name: &'static str,
    needed: bool,
    installed: bool,
    authenticated: bool,
    status_text: &str,
    command: &'static str,
    key: &'static str,
    lang: Language,
) -> Line<'static> {
    let need_label = if needed {
        lang.choose("нужен", "needed")
    } else {
        lang.choose("опционально", "optional")
    };
    let status = if !installed {
        lang.choose("CLI не найден", "CLI missing").to_string()
    } else if authenticated {
        lang.choose("аккаунт готов", "account ready").to_string()
    } else {
        lang.choose("не авторизован", "not logged in").to_string()
    };
    let status_style = if installed && authenticated {
        Style::default()
            .fg(ACCENT_SOFT)
            .add_modifier(Modifier::BOLD)
    } else if installed {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    };
    let detail = truncate_chars(status_text, 36);

    Line::from(vec![
        Span::styled(
            format!("{name:<8}"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{need_label:<12}"), Style::default().fg(MUTED)),
        Span::styled(status, status_style),
        Span::raw(" · "),
        Span::styled(format!("{key}: {command}"), Style::default().fg(MUTED)),
        Span::raw(" · "),
        Span::styled(detail, Style::default().fg(Color::DarkGray)),
    ])
}

fn command_palette_fade_level(app: &App) -> usize {
    app.command_palette_opened_at
        .map(|opened_at| (opened_at.elapsed().as_millis() / 45).min(7) as usize)
        .unwrap_or(7)
}

fn command_palette_accent(level: usize) -> Color {
    match level {
        0 => Color::Indexed(236),
        1 => Color::Indexed(238),
        2 => Color::Indexed(240),
        3 => Color::Indexed(97),
        4 => Color::Indexed(141),
        _ => ACCENT_SOFT,
    }
}

fn command_palette_muted(level: usize) -> Color {
    match level {
        0 => Color::Indexed(236),
        1 => Color::Indexed(238),
        2 => Color::Indexed(240),
        3 => Color::Indexed(243),
        4 => Color::Indexed(246),
        _ => MUTED,
    }
}

fn command_palette_selected_bg(level: usize) -> Option<Color> {
    match level {
        0..=2 => None,
        3 => Some(Color::Indexed(236)),
        4 => Some(Color::Indexed(238)),
        _ => Some(ACCENT_BG),
    }
}

fn draw_command_screen(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    if area.height == 0 {
        return;
    }

    let suggestions = app.suggestions();
    let commands = if suggestions.is_empty() {
        COMMANDS.to_vec()
    } else {
        suggestions
    };
    let selected = app
        .selected_suggestion
        .min(commands.len().saturating_sub(1));
    let visible = area.height as usize;
    let fade_level = command_palette_fade_level(app);

    let lines = commands
        .iter()
        .take(visible)
        .enumerate()
        .map(|(index, command)| {
            let is_selected = index == selected;
            let row_fade = fade_level.saturating_sub(index / 3);
            let command_style = if is_selected {
                let mut style = Style::default()
                    .fg(if row_fade >= 5 {
                        Color::White
                    } else {
                        command_palette_accent(row_fade)
                    })
                    .add_modifier(Modifier::BOLD);
                if let Some(bg) = command_palette_selected_bg(row_fade) {
                    style = style.bg(bg);
                }
                style
            } else {
                Style::default().fg(command_palette_accent(row_fade))
            };
            let desc_style = if is_selected {
                Style::default().fg(if row_fade >= 5 {
                    Color::White
                } else {
                    command_palette_muted(row_fade)
                })
            } else {
                Style::default().fg(command_palette_muted(row_fade))
            };

            Line::from(vec![
                Span::styled(
                    if is_selected { "› " } else { "  " },
                    Style::default().fg(command_palette_muted(row_fade)),
                ),
                Span::styled(format!("{:<30}", command.usage), command_style),
                Span::raw("  "),
                Span::styled(command.description(app.lang), desc_style),
            ])
        })
        .collect::<Vec<_>>();

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn draw_prompt_bar(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = input_lines_wrapped(&app.input, area.width);
    let command_mode = app.input.starts_with('/');
    let tick = current_effort_tick();
    let mut rendered = Vec::new();

    rendered.push(prompt_rule_line(area.width, command_mode, tick));
    for (index, line) in lines.iter().enumerate() {
        let prefix = if index == 0 { "› " } else { "  " };
        rendered.push(Line::from(vec![
            Span::styled(
                prefix,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.clone()),
        ]));
    }
    rendered.push(prompt_rule_line(area.width, command_mode, tick + 3));

    frame.render_widget(Paragraph::new(rendered), area);

    let (line_index, col) = input_cursor_position_wrapped(&app.input, app.cursor, area.width);
    let cursor_y = area.y + 1 + (line_index as u16).min(area.height.saturating_sub(2));
    let cursor_x = area.x + 2 + col as u16;
    let max_x = area.x + area.width.saturating_sub(1);
    frame.set_cursor_position(Position::new(cursor_x.min(max_x), cursor_y));
}

fn prompt_rule_line(width: u16, active: bool, tick: u64) -> Line<'static> {
    if !active {
        return Line::styled("─".repeat(width as usize), Style::default().fg(ACCENT_DIM));
    }

    let mut spans = Vec::new();
    for index in 0..width as usize {
        spans.push(Span::styled(
            "─",
            Style::default().fg(shimmer_color("xhigh", index, tick)),
        ));
    }
    Line::from(spans)
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if area.height == 0 {
        return;
    }

    if let Some((message, shown_at)) = &app.footer_notice {
        if shown_at.elapsed() <= Duration::from_secs(2) {
            let text = truncate_chars(message, area.width as usize);
            frame.render_widget(
                Paragraph::new(text).style(
                    Style::default()
                        .fg(ACCENT_SOFT)
                        .add_modifier(Modifier::BOLD),
                ),
                area,
            );
            return;
        }
    }

    let left = app.lang.choose(
        "? подсказки · / команды · ↑↓ история",
        "? for shortcuts · / for commands · ↑↓ history",
    );
    let (right, right_style) = footer_right_segment(app);
    let width = area.width as usize;
    let right_slot_width = footer_right_slot_width(app).min(width);
    let right = truncate_chars(&right, right_slot_width);
    let right_width = right.chars().count();
    let left_width = left.chars().count();
    let min_gap = 2;
    let left = if left_width + right_slot_width + min_gap > width {
        truncate_chars(left, width.saturating_sub(right_slot_width + min_gap))
    } else {
        left.to_string()
    };
    let gap = width.saturating_sub(left.chars().count() + right_slot_width);
    let right_padding = right_slot_width.saturating_sub(right_width);
    let line = Line::from(vec![
        Span::styled(left, Style::default().fg(ACCENT_SOFT)),
        Span::raw(" ".repeat(gap)),
        Span::raw(" ".repeat(right_padding)),
        Span::styled(right, right_style),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

fn footer_right_segments(app: &App) -> Vec<String> {
    let ready = app.lang.choose("готов", "ready");
    let mut segments = Vec::new();

    if app.status != ready {
        segments.push(format!("status {}", app.status));
    }

    segments.push(format!("mode {}", app.mode.as_str()));
    segments.push(format!("effort {}", app.compact_effort_summary()));
    segments
}

fn footer_right_target(app: &App) -> String {
    let segments = footer_right_segments(app);

    let phase = rotating_phase(8, segments.len());
    segments.get(phase).cloned().unwrap_or_default()
}

fn footer_right_slot_width(app: &App) -> usize {
    let current_width = app.footer_right_text.chars().count();
    let previous_width = app
        .footer_right_previous_text
        .as_ref()
        .map(|previous| previous.chars().count())
        .unwrap_or(0);

    current_width.max(previous_width)
}

fn footer_right_segment(app: &App) -> (String, Style) {
    let base_style = Style::default().fg(ACCENT_SOFT);
    let Some(changed_at) = app.footer_right_changed_at else {
        return (app.footer_right_text.clone(), base_style);
    };

    let elapsed_ms = changed_at.elapsed().as_millis();
    let previous = app
        .footer_right_previous_text
        .as_ref()
        .unwrap_or(&app.footer_right_text);

    if elapsed_ms < 360 {
        (
            previous.clone(),
            Style::default().fg(footer_transition_color(elapsed_ms, false)),
        )
    } else {
        (
            app.footer_right_text.clone(),
            Style::default().fg(footer_transition_color(elapsed_ms - 360, true)),
        )
    }
}

fn footer_transition_color(elapsed_ms: u128, entering: bool) -> Color {
    let step = (elapsed_ms / 90).min(4) as usize;
    let palette: &[u8] = if entering {
        &[240, 243, 246, 183, 183]
    } else {
        &[183, 246, 243, 240, 240]
    };
    Color::Indexed(palette[step])
}

fn draw_effort_screen(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let tick = current_effort_tick();
    let scale_width = effort_scale_width(area.width);
    let scale_start = (area.width as usize).saturating_sub(scale_width) / 2;
    let mut lines = Vec::new();

    lines.push(Line::styled(
        "› /effort",
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    ));
    lines.push(separator_line(area.width));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        app.lang.choose("Усилие", "Effort"),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if matches!(app.mode, Mode::ClaudeCodex) {
        push_linked_effort_mode_line(&mut lines, area.width, app, tick);
    }
    push_effort_axis(&mut lines, area.width, scale_start, scale_width, app.lang);
    lines.push(Line::from(""));

    match app.mode {
        Mode::CodexOnly => push_effort_scale_block(
            &mut lines,
            EffortScaleBlock {
                width: area.width,
                scale_start,
                scale_width,
                title: "Codex",
                provider: "codex",
                allowed: CODEX_EFFORTS,
                selected_index: app.codex_effort_index,
                focused: true,
                tick,
                lang: app.lang,
            },
        ),
        Mode::ClaudeOnly => push_effort_scale_block(
            &mut lines,
            EffortScaleBlock {
                width: area.width,
                scale_start,
                scale_width,
                title: "Claude",
                provider: "claude",
                allowed: CLAUDE_EFFORTS,
                selected_index: app.claude_effort_index,
                focused: true,
                tick,
                lang: app.lang,
            },
        ),
        Mode::ClaudeCodex => {
            if app.linked_effort_split {
                push_effort_scale_block(
                    &mut lines,
                    EffortScaleBlock {
                        width: area.width,
                        scale_start,
                        scale_width,
                        title: "Claude",
                        provider: "claude",
                        allowed: CLAUDE_EFFORTS,
                        selected_index: app.claude_effort_index,
                        focused: app.effort_focus == 1,
                        tick: tick + 2,
                        lang: app.lang,
                    },
                );
                lines.push(Line::from(""));
                push_effort_scale_block(
                    &mut lines,
                    EffortScaleBlock {
                        width: area.width,
                        scale_start,
                        scale_width,
                        title: "Codex",
                        provider: "codex",
                        allowed: CODEX_EFFORTS,
                        selected_index: app.codex_effort_index,
                        focused: app.effort_focus == 2,
                        tick: tick + 4,
                        lang: app.lang,
                    },
                );
            } else {
                push_effort_scale_block(
                    &mut lines,
                    EffortScaleBlock {
                        width: area.width,
                        scale_start,
                        scale_width,
                        title: app.lang.choose("Общий", "Shared"),
                        provider: "shared",
                        allowed: COMMON_EFFORTS,
                        selected_index: app.effort_index,
                        focused: app.effort_focus == 1,
                        tick: tick + 2,
                        lang: app.lang,
                    },
                );
            }
        }
    }

    lines.push(Line::from(""));
    let hint = match app.mode {
        Mode::CodexOnly => app.lang.choose(
            "Codex: model_reasoning_effort, доступно low|medium|high|xhigh",
            "Codex: model_reasoning_effort, available low|medium|high|xhigh",
        ),
        Mode::ClaudeOnly => app.lang.choose(
            "Claude: --effort, доступно low|medium|high|max",
            "Claude: --effort, available low|medium|high|max",
        ),
        Mode::ClaudeCodex if app.linked_effort_split => app.lang.choose(
            "Раздельно: Claude и Codex настраиваются независимо",
            "Per-model: Claude and Codex are adjusted independently",
        ),
        Mode::ClaudeCodex => app.lang.choose(
            "Общий effort применяется к обеим моделям",
            "Shared effort is sent to both models",
        ),
    };
    lines.push(positioned_spans_line(
        area.width,
        vec![(
            scale_start,
            hint.chars().count(),
            vec![Span::styled(hint.to_string(), Style::default().fg(MUTED))],
        )],
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(app.lang.choose(
        "↑/↓ выбрать · ←/→ настроить · Enter подтвердить · Esc отменить",
        "↑/↓ select · ←/→ adjust · Enter confirm · Esc cancel",
    )));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

struct EffortScaleBlock<'a> {
    width: u16,
    scale_start: usize,
    scale_width: usize,
    title: &'a str,
    provider: &'a str,
    allowed: &'static [&'static str],
    selected_index: usize,
    focused: bool,
    tick: u64,
    lang: Language,
}

fn push_effort_axis(
    lines: &mut Vec<Line<'static>>,
    width: u16,
    scale_start: usize,
    scale_width: usize,
    lang: Language,
) {
    let faster = lang.choose("Быстрее", "Faster");
    let smarter = lang.choose("Умнее", "Smarter");
    let smarter_pos = scale_start + scale_width.saturating_sub(smarter.chars().count());
    lines.push(positioned_spans_line(
        width,
        vec![
            (
                scale_start,
                faster.chars().count(),
                vec![Span::styled(
                    faster.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )],
            ),
            (
                smarter_pos,
                smarter.chars().count(),
                vec![Span::styled(
                    smarter.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )],
            ),
        ],
    ));
}

fn push_linked_effort_mode_line(lines: &mut Vec<Line<'static>>, width: u16, app: &App, tick: u64) {
    let focused = app.effort_focus == 0;
    let split_label = app.lang.choose("раздельно", "per-model");
    let common_label = app.lang.choose("общий", "shared");
    let title = app.lang.choose("Режим", "Mode");
    let prefix = if focused { "› " } else { "  " };
    let mut spans = vec![
        Span::styled(prefix, Style::default().fg(ACCENT)),
        Span::styled(
            format!("{title:<10} "),
            Style::default().fg(Color::White).add_modifier(if focused {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
    ];

    if app.linked_effort_split {
        spans.push(Span::styled(
            common_label.to_string(),
            Style::default().fg(MUTED),
        ));
        spans.push(Span::styled("  |  ", Style::default().fg(MUTED)));
        spans.extend(shimmer_text_spans(split_label, "xhigh", focused, tick));
    } else {
        spans.extend(shimmer_text_spans(common_label, "xhigh", focused, tick));
        spans.push(Span::styled("  |  ", Style::default().fg(MUTED)));
        spans.push(Span::styled(
            split_label.to_string(),
            Style::default().fg(MUTED),
        ));
    }
    lines.push(positioned_spans_line(
        width,
        vec![(0, visible_width_from_spans(&spans), spans)],
    ));
    lines.push(Line::from(""));
}

fn push_effort_scale_block(lines: &mut Vec<Line<'static>>, block: EffortScaleBlock<'_>) {
    let selected_effort = effort_label(block.selected_index);
    let prefix = if block.focused { "› " } else { "  " };
    lines.push(Line::from(vec![
        Span::styled(prefix, Style::default().fg(ACCENT)),
        Span::styled(
            format!("{:<8}", block.title),
            Style::default()
                .fg(Color::White)
                .add_modifier(if block.focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        Span::styled(
            format!(
                " {}",
                effort_provider_hint(block.provider, block.lang, block.allowed)
            ),
            Style::default().fg(MUTED),
        ),
    ]));

    let tick_positions = effort_tick_positions(block.scale_width, block.allowed.len());
    lines.push(effort_scale_line(
        block.width,
        block.scale_start,
        block.scale_width,
        &tick_positions,
    ));

    let mut label_items = Vec::new();
    for (index, effort) in block.allowed.iter().enumerate() {
        let selected = *effort == selected_effort;
        let label = effort_scale_label(effort, selected);
        let label_width = label.chars().count();
        let position = block.scale_start
            + tick_positions[index]
                .saturating_sub(label_width / 2)
                .min(block.scale_width.saturating_sub(label_width));
        label_items.push((
            position,
            label_width,
            effort_label_spans(
                &label,
                effort,
                selected && block.focused,
                block.tick + index as u64,
            ),
        ));
    }
    lines.push(positioned_spans_line(block.width, label_items));

    let description = effort_description(selected_effort, block.lang);
    lines.push(positioned_spans_line(
        block.width,
        vec![(
            block.scale_start,
            description.chars().count(),
            vec![Span::styled(
                description.to_string(),
                Style::default().fg(MUTED),
            )],
        )],
    ));
    lines.push(Line::from(""));
}

fn effort_provider_hint(provider: &str, lang: Language, allowed: &[&str]) -> String {
    match provider {
        "codex" => format!("model_reasoning_effort {}", allowed.join("|")),
        "claude" => format!("--effort {}", allowed.join("|")),
        _ => format!(
            "{} {}",
            lang.choose("доступно", "available"),
            allowed.join("|")
        ),
    }
}

fn visible_width_from_spans(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|span| span.content.chars().count()).sum()
}

fn effort_scale_width(width: u16) -> usize {
    let available = (width as usize).saturating_sub(8);
    available.clamp(32, 74)
}

fn effort_tick_positions(scale_width: usize, count: usize) -> Vec<usize> {
    if count <= 1 {
        return vec![0];
    }

    let last = scale_width.saturating_sub(1);
    (0..count).map(|index| index * last / (count - 1)).collect()
}

fn effort_scale_line(
    width: u16,
    scale_start: usize,
    scale_width: usize,
    tick_positions: &[usize],
) -> Line<'static> {
    let width = width as usize;
    let mut cells = vec![' '; width];
    let end = (scale_start + scale_width).min(width);
    for cell in cells.iter_mut().take(end).skip(scale_start) {
        *cell = '─';
    }
    for tick in tick_positions {
        let position = scale_start + *tick;
        if position < width {
            cells[position] = '┬';
        }
    }

    Line::styled(
        cells.into_iter().collect::<String>(),
        Style::default().fg(ACCENT_DIM),
    )
}

fn positioned_spans_line(
    width: u16,
    mut items: Vec<(usize, usize, Vec<Span<'static>>)>,
) -> Line<'static> {
    let width = width as usize;
    items.sort_by_key(|(position, _, _)| *position);

    let mut spans = Vec::new();
    let mut cursor = 0usize;
    for (position, item_width, item_spans) in items {
        let position = position.min(width);
        if position < cursor {
            continue;
        }
        if position > cursor {
            spans.push(Span::raw(" ".repeat(position - cursor)));
        }
        spans.extend(item_spans);
        cursor = position.saturating_add(item_width);
    }

    Line::from(spans)
}

fn current_effort_tick() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| (duration.as_millis() / 120) as u64)
        .unwrap_or(0)
}

fn rotating_phase(seconds_per_phase: u64, phase_count: usize) -> usize {
    if phase_count == 0 {
        return 0;
    }

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| ((duration.as_secs() / seconds_per_phase) as usize) % phase_count)
        .unwrap_or(0)
}

fn effort_scale_label(effort: &str, selected: bool) -> String {
    let marker = if selected {
        match effort {
            "high" | "xhigh" | "max" => "✦",
            _ => "›",
        }
    } else {
        " "
    };
    format!("{marker} {effort:<6}")
}

fn effort_label_spans(label: &str, effort: &str, selected: bool, tick: u64) -> Vec<Span<'static>> {
    let animated = selected && matches!(effort, "high" | "xhigh" | "max");
    if !animated {
        return vec![Span::styled(
            label.to_string(),
            effort_style(effort, selected),
        )];
    }

    let mut spans = Vec::new();
    let chars = label.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().enumerate() {
        let color = shimmer_color(effort, index, tick);
        spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }
    spans
}

fn shimmer_text_spans(text: &str, effort: &str, active: bool, tick: u64) -> Vec<Span<'static>> {
    if !active {
        return vec![Span::styled(text.to_string(), effort_style(effort, true))];
    }

    text.chars()
        .enumerate()
        .map(|(index, ch)| {
            Span::styled(
                ch.to_string(),
                Style::default()
                    .fg(shimmer_color(effort, index, tick))
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect()
}

fn shimmer_color(effort: &str, index: usize, tick: u64) -> Color {
    let palette: &[u8] = match effort {
        "high" => &[136, 178, 220, 214, 220, 178, 136],
        "xhigh" => &[97, 141, 183, 219, 183, 141, 97],
        "max" => &[160, 198, 203, 205, 203, 198, 160],
        _ => &[250],
    };
    let phase = (tick as usize) % palette.len();
    let color_index = (index + palette.len() - phase) % palette.len();
    Color::Indexed(palette[color_index])
}

fn effort_style(effort: &str, selected: bool) -> Style {
    let color = match effort {
        "low" => Color::Indexed(114),
        "medium" => Color::Indexed(117),
        "high" => Color::Indexed(220),
        "xhigh" => ACCENT_SOFT,
        "max" => Color::Indexed(203),
        _ => MUTED,
    };

    let mut style = Style::default().fg(color);
    if selected {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

fn effort_description(effort: &str, lang: Language) -> &'static str {
    match effort {
        "low" => lang.choose("быстро и экономно", "fast and frugal"),
        "medium" => lang.choose("баланс скорости и качества", "balanced speed and quality"),
        "high" => lang.choose("глубже думает над задачей", "deeper task reasoning"),
        "xhigh" => lang.choose("максимум Codex reasoning", "maximum Codex reasoning"),
        "max" => lang.choose("максимум Claude effort", "maximum Claude effort"),
        _ => "",
    }
}

fn composer_height(app: &App, width: u16) -> u16 {
    let lines = input_lines_wrapped(&app.input, width).len() as u16;
    (lines + 2).clamp(3, 10)
}

fn initial_transcript(_lang: Language) -> Vec<String> {
    Vec::new()
}

fn provider_count() -> usize {
    3
}

fn provider_mode(index: usize) -> Mode {
    match index {
        0 => Mode::CodexOnly,
        1 => Mode::ClaudeCodex,
        2 => Mode::ClaudeOnly,
        _ => Mode::CodexOnly,
    }
}

fn provider_index(mode: Mode) -> usize {
    match mode {
        Mode::CodexOnly => 0,
        Mode::ClaudeCodex => 1,
        Mode::ClaudeOnly => 2,
    }
}

fn provider_description(mode: Mode, lang: Language) -> &'static str {
    match mode {
        Mode::CodexOnly => lang.choose("Codex пишет и ревьюит", "Codex drafts and reviews"),
        Mode::ClaudeCodex => lang.choose(
            "Claude пишет, Codex ревьюит",
            "Claude drafts, Codex reviews",
        ),
        Mode::ClaudeOnly => lang.choose("Claude пишет и ревьюит", "Claude drafts and reviews"),
    }
}

fn input_lines_wrapped(input: &str, width: u16) -> Vec<String> {
    let content_width = width.saturating_sub(2).max(1) as usize;
    if input.is_empty() {
        return vec![String::new()];
    }

    let mut rows = Vec::new();
    for line in input.split('\n') {
        rows.extend(wrap_terminal_text_preserving_spaces(line, content_width));
    }
    rows
}

fn input_cursor_position_wrapped(input: &str, cursor: usize, width: u16) -> (usize, usize) {
    let content_width = width.saturating_sub(2).max(1) as usize;
    let before = &input[..cursor];
    let parts = before.split('\n').collect::<Vec<_>>();
    let mut visual_line = 0usize;
    let mut visual_col = 0usize;

    for (index, line) in parts.iter().enumerate() {
        let len = line.chars().count();
        if index + 1 == parts.len() {
            visual_line += len / content_width;
            visual_col = len % content_width;
        } else {
            visual_line += (len / content_width) + 1;
        }
    }

    (visual_line, visual_col)
}

fn wrap_terminal_line(text: &str, width: u16) -> Vec<String> {
    let max_chars = width.saturating_sub(1).max(1) as usize;
    wrap_terminal_text_preserving_spaces(text, max_chars)
}

fn wrap_terminal_text_preserving_spaces(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut rows = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch == '\n' {
            rows.push(current);
            current = String::new();
            continue;
        }

        if current.chars().count() >= max_chars {
            rows.push(current);
            current = String::new();
        }
        current.push(ch);
    }

    rows.push(current);
    rows
}

fn wrap_chars(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let max_chars = max_chars.max(1);
    let mut rows = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let current_len = current.chars().count();
        let word_len = word.chars().count();
        let extra_space = usize::from(!current.is_empty());

        if current_len + extra_space + word_len > max_chars && !current.is_empty() {
            rows.push(current);
            current = String::new();
        }

        if word_len > max_chars {
            if !current.is_empty() {
                rows.push(current);
                current = String::new();
            }

            let mut chunk = String::new();
            for ch in word.chars() {
                if chunk.chars().count() >= max_chars {
                    rows.push(chunk);
                    chunk = String::new();
                }
                chunk.push(ch);
            }
            if !chunk.is_empty() {
                current = chunk;
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        rows.push(current);
    }
    rows
}

fn loader_line(app: &App) -> Line<'static> {
    let elapsed = app
        .run_started_at
        .map(|started| started.elapsed())
        .unwrap_or_else(|| Duration::from_secs(0));
    let phrase = LOADER_PHRASES
        .get(((elapsed.as_secs() / 6) as usize) % LOADER_PHRASES.len())
        .copied()
        .unwrap_or("Thinking");
    let label = if app.run_label.is_empty() {
        app.mode.as_str().to_string()
    } else {
        app.run_label.clone()
    };
    let token_detail = app
        .run_token_estimate
        .map(|tokens| {
            let live_tokens = live_token_estimate(tokens, elapsed, app.active_effort_for_tokens());
            format!(" · ≈ {} tokens", format_token_count(live_tokens))
        })
        .unwrap_or_default();
    let detail = format!(
        "({} · {} · effort {}{})",
        format_elapsed(elapsed),
        label,
        app.effort_summary(),
        token_detail
    );

    let mut spans = shimmer_text_spans(
        &format!("✳ {}… ", phrase),
        "xhigh",
        true,
        current_effort_tick(),
    );
    spans.push(Span::styled(
        detail,
        Style::default().fg(Color::Indexed(245)),
    ));
    Line::from(spans)
}

fn live_token_estimate(base: usize, elapsed: Duration, effort: &str) -> usize {
    let per_second = effort_weight(effort);
    base.saturating_add(elapsed.as_secs() as usize * per_second)
}

fn effort_weight(effort: &str) -> usize {
    match effort {
        "low" => 8,
        "medium" => 16,
        "high" => 28,
        "xhigh" => 44,
        "max" => 52,
        _ => 20,
    }
}

fn format_elapsed(duration: Duration) -> String {
    let total = duration.as_secs();
    if total < 60 {
        return format!("{}s", total.max(1));
    }

    let minutes = total / 60;
    let seconds = total % 60;
    if minutes < 60 {
        return format!("{}m {:02}s", minutes, seconds);
    }

    let hours = minutes / 60;
    let minutes = minutes % 60;
    format!("{}h {:02}m", hours, minutes)
}

fn main_area_height(
    app: &App,
    area: Rect,
    composer_height: u16,
    palette_height: u16,
    footer_height: u16,
    output_gap: u16,
    palette_gap: u16,
) -> u16 {
    let max_height = area
        .height
        .saturating_sub(composer_height)
        .saturating_sub(palette_height)
        .saturating_sub(footer_height)
        .saturating_sub(output_gap)
        .saturating_sub(palette_gap)
        .max(1);

    let desired = if app.transcript.is_empty() && !app.running && app.last_run.is_none() {
        area.height.min(12).max(1)
    } else {
        transcript_content_height(app, area.width).max(1)
    };

    desired.min(max_height)
}

fn transcript_content_height(app: &App, width: u16) -> u16 {
    let mut height = 1usize;
    for line in &app.transcript {
        height += transcript_entry_lines(line, app.lang, width).len();
    }
    if app.running {
        height += 2;
    }
    height.min(u16::MAX as usize) as u16
}

fn command_palette_height(app: &App, screen_height: u16, composer_height: u16) -> u16 {
    let suggestions = app.suggestions();
    let command_count = if suggestions.is_empty() {
        COMMANDS.len()
    } else {
        suggestions.len()
    };
    let available = screen_height
        .saturating_sub(composer_height)
        .saturating_sub(6)
        .max(3);
    (command_count as u16).min(available).min(12)
}

fn final_brief_lines_for_chat(path: &str, lang: Language) -> io::Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    let mut lines = Vec::new();
    let mut in_current_spec = false;
    let mut in_last_review = false;
    let mut emitted_any = false;

    for raw in content.lines() {
        let line = raw.trim_end();
        if line == "## Current Spec" {
            in_current_spec = true;
            in_last_review = false;
            lines.push(
                lang.choose("## Текущая спека", "## Current Spec")
                    .to_string(),
            );
            emitted_any = true;
            continue;
        }
        if line == "## Last Review" {
            in_current_spec = false;
            in_last_review = true;
            lines.push(
                lang.choose("## Последнее ревью", "## Last Review")
                    .to_string(),
            );
            emitted_any = true;
            continue;
        }
        if line.starts_with("## ") {
            in_current_spec = false;
            in_last_review = false;
        }

        if in_current_spec || in_last_review {
            lines.push(line.to_string());
        }
    }

    if !emitted_any
        || lines
            .iter()
            .all(|line| line.trim().is_empty() || line.starts_with("## "))
    {
        lines = content.lines().map(ToString::to_string).collect();
    }

    let mut compact = Vec::new();
    let mut previous_blank = false;
    for line in lines {
        let blank = line.trim().is_empty();
        if blank && previous_blank {
            continue;
        }
        previous_blank = blank;
        compact.push(truncate_chars(&line, 220));
        if compact.len() >= 140 {
            compact.push(
                lang.choose(
                    "… ответ обрезан, полный brief сохранён в файле выше",
                    "… answer truncated, full brief is saved in the file above",
                )
                .to_string(),
            );
            break;
        }
    }

    Ok(compact)
}

fn is_welcome_line(line: &str) -> bool {
    let line = line.trim();
    line.starts_with("✦ Добро пожаловать")
        || line.starts_with("✦ Welcome")
        || line.starts_with("Введите задачу")
        || line.starts_with("Type a task")
        || line.starts_with("Это Claude Code-style")
        || line.starts_with("This is a Claude Code-style")
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }

    if max_chars == 0 {
        return String::new();
    }

    let mut truncated = text
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

fn duel_state_dir() -> PathBuf {
    if let Ok(path) = env::var("DUEL_HOME") {
        return PathBuf::from(path);
    }

    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home).join(".duel");
    }

    PathBuf::from(".duel")
}

fn history_path() -> PathBuf {
    duel_state_dir().join("history")
}

fn chats_dir() -> PathBuf {
    duel_state_dir().join("chats")
}

fn config_path() -> PathBuf {
    if let Ok(path) = env::var("DUEL_CONFIG") {
        return PathBuf::from(path);
    }

    duel_state_dir().join("config")
}

fn load_config(path: &Path) -> AppConfig {
    let Ok(content) = fs::read_to_string(path) else {
        return AppConfig::default();
    };

    let mut config = AppConfig::default();
    let mut legacy_effort = None;
    let mut codex_effort_seen = false;
    let mut claude_effort_seen = false;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"');

        match key {
            "onboarding_done" => config.onboarding_done = value == "true",
            "mode" => {
                if let Some(mode) = Mode::from_str(value) {
                    config.mode = mode;
                }
            }
            "lang" => {
                if let Some(lang) = Language::from_str(value) {
                    config.lang = lang;
                }
            }
            "rounds" => {
                if let Ok(rounds) = value.parse::<usize>() {
                    config.rounds = rounds.max(1);
                }
            }
            "out_dir" => config.out_dir = value.to_string(),
            "effort" => {
                if let Some(index) = EFFORTS.iter().position(|effort| *effort == value) {
                    config.effort_index = index;
                    legacy_effort = Some(index);
                }
            }
            "codex_effort" => {
                if let Some(index) = EFFORTS.iter().position(|effort| *effort == value) {
                    config.codex_effort_index = index;
                    codex_effort_seen = true;
                }
            }
            "claude_effort" => {
                if let Some(index) = EFFORTS.iter().position(|effort| *effort == value) {
                    config.claude_effort_index = index;
                    claude_effort_seen = true;
                }
            }
            "linked_effort" => {
                config.linked_effort_split = match value {
                    "split" | "per-model" | "true" => true,
                    "shared" | "common" | "false" => false,
                    _ => config.linked_effort_split,
                };
            }
            "split_effort" => {
                config.linked_effort_split = value == "true";
            }
            "effort_split" => {
                config.linked_effort_split = value == "true";
            }
            "linked_effort_split" => {
                config.linked_effort_split = value == "true";
            }
            "per_model_effort" => {
                config.linked_effort_split = value == "true";
            }
            "model_effort_mode" => {
                config.linked_effort_split = matches!(value, "split" | "per-model");
            }
            "effort_mode" => {
                config.linked_effort_split = matches!(value, "split" | "per-model");
            }
            "effort_per_model" => {
                config.linked_effort_split = value == "true";
            }
            "effort_shared" => {
                config.linked_effort_split = value != "true";
            }
            "effort_common" => {
                config.linked_effort_split = value != "true";
            }
            "common_effort" => {
                if let Some(index) = EFFORTS.iter().position(|effort| *effort == value) {
                    config.effort_index = index;
                    legacy_effort = Some(index);
                }
            }
            "last_chat" => {
                let chat_id = sanitize_chat_id(value);
                if !chat_id.is_empty() {
                    config.last_chat_id = Some(chat_id);
                }
            }
            _ => {}
        }
    }

    if let Some(index) = legacy_effort {
        let effort = effort_label(index);
        if !codex_effort_seen && provider_supports_effort("codex", effort) {
            config.codex_effort_index = index;
        }
        if !claude_effort_seen && provider_supports_effort("claude", effort) {
            config.claude_effort_index = index;
        }
    }

    config
}

fn save_config(path: &Path, config: &AppConfig) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = format!(
        concat!(
            "onboarding_done={}\n",
            "mode=\"{}\"\n",
            "lang=\"{}\"\n",
            "rounds={}\n",
            "out_dir=\"{}\"\n",
            "effort=\"{}\"\n",
            "codex_effort=\"{}\"\n",
            "claude_effort=\"{}\"\n",
            "linked_effort=\"{}\"\n",
            "last_chat=\"{}\"\n",
        ),
        config.onboarding_done,
        config.mode.as_str(),
        config.lang.as_str(),
        config.rounds,
        config.out_dir,
        effort_label(config.effort_index),
        effort_label(config.codex_effort_index),
        effort_label(config.claude_effort_index),
        if config.linked_effort_split {
            "split"
        } else {
            "shared"
        },
        config.last_chat_id.as_deref().unwrap_or(""),
    );
    fs::write(path, content)
}

fn load_history(path: &Path) -> io::Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    let mut history = content
        .lines()
        .map(decode_field)
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();

    if history.len() > MAX_HISTORY_LINES {
        let remove_count = history.len() - MAX_HISTORY_LINES;
        history.drain(0..remove_count);
    }

    Ok(history)
}

fn save_history(path: &Path, history: &[String]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(path)?;
    for line in history
        .iter()
        .rev()
        .take(MAX_HISTORY_LINES)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        writeln!(file, "{}", encode_field(line))?;
    }
    Ok(())
}

#[derive(Clone)]
struct ChatSummary {
    id: String,
    title: String,
    lines: usize,
    modified: SystemTime,
}

fn restore_or_create_chat(
    chats_dir: &Path,
    _last_chat_id: Option<&str>,
    lang: Language,
) -> (String, PathBuf, Vec<String>) {
    let chat_id = new_chat_id();
    let path = chat_path_for_id(chats_dir, &chat_id);
    let transcript = initial_transcript(lang);
    (chat_id, path, transcript)
}

fn new_chat_id() -> String {
    format!("chat-{}", unix_millis())
}

fn chat_path_for_id(chats_dir: &Path, chat_id: &str) -> PathBuf {
    chats_dir.join(format!(
        "{}.{}",
        sanitize_chat_id(chat_id),
        CHAT_FILE_EXTENSION
    ))
}

fn sanitize_chat_id(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(&format!(".{}", CHAT_FILE_EXTENSION))
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect()
}

fn save_chat_transcript(path: &Path, chat_id: &str, transcript: &[String]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(path)?;
    writeln!(file, "# Duel Chat")?;
    writeln!(file, "id={}", chat_id)?;
    writeln!(file, "created={}", unix_millis())?;
    writeln!(file, "---")?;
    for line in transcript {
        writeln!(file, "v1\t{}", encode_field(line))?;
    }
    Ok(())
}

fn append_chat_line(path: &Path, line: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if !path.exists() {
        let chat_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| "unknown".to_string());
        save_chat_transcript(path, &chat_id, &[])?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "v1\t{}", encode_field(line))
}

fn load_chat_transcript(path: &Path) -> io::Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter_map(|line| line.strip_prefix("v1\t"))
        .map(decode_field)
        .filter(|line| !is_welcome_line(line))
        .collect())
}

fn list_saved_chats(chats_dir: &Path, limit: usize) -> Vec<ChatSummary> {
    let Ok(entries) = fs::read_dir(chats_dir) else {
        return Vec::new();
    };

    let mut chats = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some(CHAT_FILE_EXTENSION))
        .filter_map(|path| chat_summary(&path))
        .collect::<Vec<_>>();

    chats.sort_by(|left, right| right.modified.cmp(&left.modified));
    chats.truncate(limit);
    chats
}

fn chat_summary(path: &Path) -> Option<ChatSummary> {
    let id = path.file_stem()?.to_string_lossy().to_string();
    let modified = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(UNIX_EPOCH);
    let lines = load_chat_transcript(path).ok()?;
    let title = lines
        .iter()
        .find_map(|line| line.strip_prefix("◆ ").map(str::trim))
        .or_else(|| {
            lines
                .iter()
                .find(|line| !line.trim().is_empty())
                .map(String::as_str)
        })
        .map(|line| truncate_chars(line, 72))
        .unwrap_or_else(|| "empty chat".to_string());

    Some(ChatSummary {
        id,
        title,
        lines: lines.len(),
        modified,
    })
}

fn find_last_run(transcript: &[String]) -> Option<String> {
    transcript
        .iter()
        .rev()
        .find_map(|line| line.strip_prefix("Final brief: ").map(ToString::to_string))
}

fn encode_field(value: &str) -> String {
    let mut encoded = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => encoded.push_str("\\\\"),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            _ => encoded.push(ch),
        }
    }
    encoded
}

fn decode_field(value: &str) -> String {
    let mut decoded = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            decoded.push(ch);
            continue;
        }

        match chars.next() {
            Some('n') => decoded.push('\n'),
            Some('r') => decoded.push('\r'),
            Some('t') => decoded.push('\t'),
            Some('\\') => decoded.push('\\'),
            Some(other) => decoded.push(other),
            None => decoded.push('\\'),
        }
    }
    decoded
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn auth_requirements_ready(mode: Mode, onboarding: &Onboarding) -> bool {
    (!mode.needs_codex() || onboarding.codex_authenticated)
        && (!mode.needs_claude() || onboarding.claude_authenticated)
}

fn missing_auth_text(mode: Mode, onboarding: &Onboarding, lang: Language) -> String {
    let mut missing = Vec::new();
    if mode.needs_codex() && !onboarding.codex_authenticated {
        missing.push(if onboarding.codex_installed {
            "Codex"
        } else {
            lang.choose("Codex CLI не найден", "Codex CLI missing")
        });
    }
    if mode.needs_claude() && !onboarding.claude_authenticated {
        missing.push(if onboarding.claude_installed {
            "Claude"
        } else {
            lang.choose("Claude CLI не найден", "Claude CLI missing")
        });
    }

    if missing.is_empty() {
        lang.choose("всё готово", "all ready").to_string()
    } else {
        missing.join(" + ")
    }
}

fn codex_auth_probe() -> AuthProbe {
    match Command::new("codex").args(["login", "status"]).output() {
        Ok(output) => {
            let text = command_output_text(&output.stdout, &output.stderr);
            AuthProbe {
                installed: true,
                authenticated: auth_output_looks_ready(output.status.success(), &text),
                status: first_nonempty_line(&text)
                    .unwrap_or_else(|| "status unavailable".to_string()),
            }
        }
        Err(err) => AuthProbe {
            installed: false,
            authenticated: false,
            status: err.to_string(),
        },
    }
}

fn claude_auth_probe() -> AuthProbe {
    match Command::new("claude")
        .args(["auth", "status", "--text"])
        .output()
    {
        Ok(output) => {
            let text = command_output_text(&output.stdout, &output.stderr);
            AuthProbe {
                installed: true,
                authenticated: auth_output_looks_ready(output.status.success(), &text),
                status: first_nonempty_line(&text)
                    .unwrap_or_else(|| "status unavailable".to_string()),
            }
        }
        Err(err) => AuthProbe {
            installed: false,
            authenticated: false,
            status: err.to_string(),
        },
    }
}

fn auth_output_looks_ready(success: bool, text: &str) -> bool {
    if !success {
        return false;
    }

    let lower = text.to_lowercase();
    !lower.contains("not logged")
        && !lower.contains("not authenticated")
        && !lower.contains("not signed")
        && !lower.contains("login required")
        && !lower.contains("logged out")
        && !lower.contains("no credentials")
}

fn command_output_text(stdout: &[u8], stderr: &[u8]) -> String {
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(stdout));
    if !stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(stderr));
    }
    text
}

fn first_nonempty_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("WARNING:"))
        .map(ToString::to_string)
}

fn run_external_command(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    command: &ExternalCommand,
) -> AnyResult<i32> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!();
    println!(
        "Duel: running {} {}",
        command.program,
        command.args.join(" ")
    );
    println!();

    let result = Command::new(command.program).args(command.args).status();
    let code = match result {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            println!("Duel: failed to start command: {err}");
            1
        }
    };

    println!();
    println!("Duel: press Enter to return...");
    let mut wait = String::new();
    let _ = io::stdin().read_line(&mut wait);

    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.clear()?;

    Ok(code)
}

fn spawn_reader<R>(reader: R, tx: Sender<WorkerEvent>)
where
    R: io::Read + Send + 'static,
{
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let _ = tx.send(WorkerEvent::Line(line));
                }
                Err(err) => {
                    let _ = tx.send(WorkerEvent::Line(format!("read error: {err}")));
                    break;
                }
            }
        }
    });
}

fn prefix_chars(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}

fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    let words = text.split_whitespace().count();
    ((chars / 4).max(words)).max(1)
}

fn format_token_count(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}m", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

fn chat_provider(mode: Mode) -> &'static str {
    match mode {
        Mode::CodexOnly => "codex",
        Mode::ClaudeOnly | Mode::ClaudeCodex => "claude",
    }
}

fn provider_display(provider: &str, lang: Language) -> &'static str {
    match provider {
        "codex" => "Codex",
        "claude" => "Claude",
        _ => lang.choose("Модель", "Model"),
    }
}

fn chat_prompt(message: &str, context: &str, lang: Language) -> String {
    let language_hint = lang.choose(
        "Отвечай на русском, если пользователь не просит другой язык.",
        "Reply in English unless the user asks for another language.",
    );
    format!(
        concat!(
            "You are Duel Code, a direct chat assistant inside a terminal UI.\n",
            "Answer the user's message directly. Do not create a spec, do not run a planning loop, and do not modify files.\n",
            "Keep the answer concise and useful. {language_hint}\n\n",
            "Recent chat context:\n{context}\n\n",
            "User message:\n{message}"
        ),
        language_hint = language_hint,
        context = if context.trim().is_empty() { "(empty)" } else { context },
        message = message
    )
}

fn recent_chat_context(transcript: &[String], max_lines: usize) -> String {
    transcript
        .iter()
        .rev()
        .filter(|line| !line.starts_with("⏺ Отправляю") && !line.starts_with("⏺ Sending"))
        .take(max_lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|line| truncate_chars(line, 240))
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_chat_provider(
    provider: &'static str,
    effort: &str,
    prompt: &str,
    work_dir: &Path,
    cancel_rx: Receiver<()>,
) -> io::Result<ChatRunResult> {
    let mut command = if provider == "claude" {
        let program = env::var("AI_ORCHESTRATOR_CLAUDE").unwrap_or_else(|_| "claude".to_string());
        let mut command = Command::new(program);
        command.args([
            "-p",
            "--effort",
            effort,
            "--no-session-persistence",
            "--tools",
            "",
            "--max-turns",
            "3",
            "--output-format",
            "text",
            prompt,
        ]);
        command
    } else {
        let program = env::var("AI_ORCHESTRATOR_CODEX").unwrap_or_else(|_| "codex".to_string());
        let mut command = Command::new(program);
        command.args([
            "exec",
            "-c",
            &format!("model_reasoning_effort=\"{}\"", effort),
            "--skip-git-repo-check",
            "--ephemeral",
            "--color",
            "never",
            "-s",
            "read-only",
            prompt,
        ]);
        command
    };

    let mut child = command
        .current_dir(work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout_handle = child.stdout.take().map(spawn_capture_reader);
    let stderr_handle = child.stderr.take().map(spawn_capture_reader);

    loop {
        if cancel_rx.try_recv().is_ok() {
            let _ = child.kill();
            let _ = child.wait();
            if let Some(handle) = stdout_handle {
                let _ = handle.join();
            }
            if let Some(handle) = stderr_handle {
                let _ = handle.join();
            }
            return Ok(ChatRunResult::Cancelled);
        }

        match child.try_wait()? {
            Some(status) => {
                let stdout = stdout_handle
                    .map(|handle| handle.join().unwrap_or_default())
                    .unwrap_or_default();
                let stderr = stderr_handle
                    .map(|handle| handle.join().unwrap_or_default())
                    .unwrap_or_default();
                return Ok(ChatRunResult::Completed(
                    status.code().unwrap_or(1),
                    stdout,
                    stderr,
                ));
            }
            None => thread::sleep(Duration::from_millis(80)),
        }
    }
}

fn spawn_capture_reader<R>(reader: R) -> thread::JoinHandle<String>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut text = String::new();
        let _ = reader.read_to_string(&mut text);
        text
    })
}

fn emit_chat_lines(tx: &Sender<WorkerEvent>, text: &str) {
    let mut first_content = true;
    for line in text.lines() {
        let rendered = if first_content && !line.trim().is_empty() {
            first_content = false;
            format!("⏺ {}", line.trim_start())
        } else {
            line.to_string()
        };
        let _ = tx.send(WorkerEvent::ChatLine(rendered));
    }
}

fn emit_error_lines(tx: &Sender<WorkerEvent>, text: &str) {
    let mut emitted = 0;
    for line in text.lines().filter(|line| !line.trim().is_empty()).take(40) {
        let _ = tx.send(WorkerEvent::Line(format!("⎿ {}", line)));
        emitted += 1;
    }
    if emitted == 0 {
        let _ = tx.send(WorkerEvent::Line("⎿ no stderr output".to_string()));
    }
}

fn engine_path() -> Option<PathBuf> {
    if let Ok(path) = env::var("DUEL_ENGINE") {
        if let Some(path) = existing_path(PathBuf::from(path)) {
            return Some(path);
        }
    }

    if let Ok(current_dir) = env::current_dir() {
        if let Some(path) = existing_path(current_dir.join("spec-duel")) {
            return Some(path);
        }
    }

    if let Ok(exe) = env::current_exe() {
        for dir in exe.ancestors().skip(1).take(4) {
            if let Some(path) = existing_path(dir.join("spec-duel")) {
                return Some(path);
            }
        }
    }

    None
}

fn existing_path(path: PathBuf) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    Some(path.canonicalize().unwrap_or(path))
}

fn engine_work_dir(engine: &Path) -> PathBuf {
    engine
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn previous_boundary(input: &str, cursor: usize) -> usize {
    input[..cursor]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_boundary(input: &str, cursor: usize) -> usize {
    input[cursor..]
        .char_indices()
        .nth(1)
        .map(|(index, _)| cursor + index)
        .unwrap_or_else(|| input.len())
}

fn previous_word_boundary(input: &str, cursor: usize) -> usize {
    if cursor == 0 {
        return 0;
    }

    let mut position = cursor;
    while position > 0 {
        let previous = previous_boundary(input, position);
        let ch = input[previous..position].chars().next().unwrap_or(' ');
        if !ch.is_whitespace() {
            break;
        }
        position = previous;
    }

    if position == 0 {
        return 0;
    }

    let previous = previous_boundary(input, position);
    let word_mode = is_word_char(input[previous..position].chars().next().unwrap_or(' '));

    while position > 0 {
        let previous = previous_boundary(input, position);
        let ch = input[previous..position].chars().next().unwrap_or(' ');
        if ch.is_whitespace() || is_word_char(ch) != word_mode {
            break;
        }
        position = previous;
    }

    position
}

fn next_word_boundary(input: &str, cursor: usize) -> usize {
    if cursor >= input.len() {
        return input.len();
    }

    let mut position = cursor;
    while position < input.len() {
        let next = next_boundary(input, position);
        let ch = input[position..next].chars().next().unwrap_or(' ');
        if !ch.is_whitespace() {
            break;
        }
        position = next;
    }

    if position >= input.len() {
        return input.len();
    }

    let next = next_boundary(input, position);
    let word_mode = is_word_char(input[position..next].chars().next().unwrap_or(' '));

    while position < input.len() {
        let next = next_boundary(input, position);
        let ch = input[position..next].chars().next().unwrap_or(' ');
        if ch.is_whitespace() || is_word_char(ch) != word_mode {
            break;
        }
        position = next;
    }

    position
}

fn line_start_boundary(input: &str, cursor: usize) -> usize {
    input[..cursor]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn line_end_boundary(input: &str, cursor: usize) -> usize {
    input[cursor..]
        .find('\n')
        .map(|index| cursor + index)
        .unwrap_or_else(|| input.len())
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}
