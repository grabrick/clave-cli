use crate::prelude::*;
use crate::*;

#[derive(Clone)]
pub(crate) struct AppConfig {
    pub(crate) onboarding_done: bool,
    pub(crate) mode: Mode,
    pub(crate) lang: Language,
    pub(crate) rounds: usize,
    pub(crate) out_dir: String,
    pub(crate) effort_index: usize,
    pub(crate) codex_effort_index: usize,
    pub(crate) claude_effort_index: usize,
    pub(crate) linked_effort_split: bool,
    pub(crate) last_chat_id: Option<String>,
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
pub(crate) enum OnboardingStep {
    Provider,
    Auth,
    Settings,
}

pub(crate) struct Onboarding {
    pub(crate) step: OnboardingStep,
    pub(crate) provider_index: usize,
    pub(crate) setting_index: usize,
    pub(crate) codex_installed: bool,
    pub(crate) claude_installed: bool,
    pub(crate) codex_authenticated: bool,
    pub(crate) claude_authenticated: bool,
    pub(crate) codex_status: String,
    pub(crate) claude_status: String,
    pub(crate) message: String,
}

impl Onboarding {
    pub(crate) fn new(mode: Mode) -> Self {
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

    pub(crate) fn refresh_auth(&mut self) {
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

pub(crate) struct AuthProbe {
    pub(crate) installed: bool,
    pub(crate) authenticated: bool,
    pub(crate) status: String,
}

#[derive(Debug)]
pub(crate) enum WorkerEvent {
    Line(String),
    ChatLine(String),
    Done(i32),
    ChatDone(&'static str, i32),
    Cancelled,
    Failed(String),
}

pub(crate) enum ChatRunResult {
    Completed(i32, String, String),
    Cancelled,
}

pub(crate) struct RevealLine {
    pub(crate) text: String,
    pub(crate) visible_chars: usize,
    pub(crate) transcript_index: usize,
    pub(crate) last_tick: Instant,
}

#[derive(Clone, Copy)]
pub(crate) struct EffortSnapshot {
    pub(crate) effort_index: usize,
    pub(crate) codex_effort_index: usize,
    pub(crate) claude_effort_index: usize,
    pub(crate) linked_effort_split: bool,
    pub(crate) effort_focus: usize,
}

pub(crate) struct ExternalCommand {
    pub(crate) program: &'static str,
    pub(crate) args: &'static [&'static str],
    pub(crate) label_ru: &'static str,
    pub(crate) label_en: &'static str,
}

pub(crate) struct App {
    pub(crate) mode: Mode,
    pub(crate) lang: Language,
    pub(crate) rounds: usize,
    pub(crate) out_dir: String,
    pub(crate) config_path: PathBuf,
    pub(crate) history_path: PathBuf,
    pub(crate) chats_dir: PathBuf,
    pub(crate) chat_id: String,
    pub(crate) chat_path: PathBuf,
    pub(crate) onboarding: Option<Onboarding>,
    pub(crate) pending_external: Option<ExternalCommand>,
    pub(crate) input: String,
    pub(crate) cursor: usize,
    pub(crate) transcript: Vec<String>,
    pub(crate) reveal_queue: VecDeque<String>,
    pub(crate) reveal_active: Option<RevealLine>,
    pub(crate) status: String,
    pub(crate) last_run: Option<String>,
    pub(crate) running: bool,
    pub(crate) run_started_at: Option<Instant>,
    pub(crate) run_label: String,
    pub(crate) run_token_estimate: Option<usize>,
    pub(crate) cancel_tx: Option<Sender<()>>,
    pub(crate) last_ctrl_c_at: Option<Instant>,
    pub(crate) footer_notice: Option<(String, Instant)>,
    pub(crate) footer_right_text: String,
    pub(crate) footer_right_previous_text: Option<String>,
    pub(crate) footer_right_changed_at: Option<Instant>,
    pub(crate) should_quit: bool,
    pub(crate) history: Vec<String>,
    pub(crate) history_index: Option<usize>,
    pub(crate) selected_suggestion: usize,
    pub(crate) command_palette_opened_at: Option<Instant>,
    pub(crate) command_palette_query: String,
    pub(crate) effort_picker: bool,
    pub(crate) effort_original: Option<EffortSnapshot>,
    pub(crate) effort_focus: usize,
    pub(crate) effort_index: usize,
    pub(crate) codex_effort_index: usize,
    pub(crate) claude_effort_index: usize,
    pub(crate) linked_effort_split: bool,
    pub(crate) tx: Sender<WorkerEvent>,
    pub(crate) rx: Receiver<WorkerEvent>,
}

impl App {
    pub(crate) fn new() -> Self {
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

    pub(crate) fn current_config(&self, onboarding_done: bool) -> AppConfig {
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

    pub(crate) fn save_current_config(&mut self, onboarding_done: bool) {
        if let Err(err) = save_config(&self.config_path, &self.current_config(onboarding_done)) {
            self.push_system(format!(
                "{} {}",
                self.lang
                    .choose("Не удалось сохранить конфиг:", "Failed to save config:"),
                err
            ));
        }
    }

    pub(crate) fn remember_history_entry(&mut self, line: &str) {
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

    pub(crate) fn start_new_chat(&mut self) {
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

    pub(crate) fn resume_chat(&mut self, chat_id: &str) {
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

    pub(crate) fn show_saved_chats(&mut self) {
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

    pub(crate) fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.effort_focus = 0;
        self.effort_index = normalize_common_effort_index(self.effort_index);
        self.codex_effort_index = normalize_provider_effort_index("codex", self.codex_effort_index);
        self.claude_effort_index =
            normalize_provider_effort_index("claude", self.claude_effort_index);
    }

    pub(crate) fn effort_summary(&self) -> String {
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

    pub(crate) fn compact_effort_summary(&self) -> String {
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

    pub(crate) fn provider_effort(&self, provider: &str) -> &'static str {
        if self.mode == Mode::ClaudeCodex && !self.linked_effort_split {
            return effort_label(self.effort_index);
        }

        match provider {
            "claude" => effort_label(self.claude_effort_index),
            "codex" => effort_label(self.codex_effort_index),
            _ => effort_label(self.effort_index),
        }
    }

    pub(crate) fn active_effort_for_tokens(&self) -> &'static str {
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

    pub(crate) fn effort_snapshot(&self) -> EffortSnapshot {
        EffortSnapshot {
            effort_index: self.effort_index,
            codex_effort_index: self.codex_effort_index,
            claude_effort_index: self.claude_effort_index,
            linked_effort_split: self.linked_effort_split,
            effort_focus: self.effort_focus,
        }
    }

    pub(crate) fn restore_effort_snapshot(&mut self, snapshot: EffortSnapshot) {
        self.effort_index = snapshot.effort_index;
        self.codex_effort_index = snapshot.codex_effort_index;
        self.claude_effort_index = snapshot.claude_effort_index;
        self.linked_effort_split = snapshot.linked_effort_split;
        self.effort_focus = snapshot.effort_focus;
    }

    pub(crate) fn effort_picker_rows(&self) -> usize {
        match self.mode {
            Mode::ClaudeCodex if self.linked_effort_split => 3,
            Mode::ClaudeCodex => 2,
            _ => 1,
        }
    }

    pub(crate) fn adjust_effort_focus(&mut self, direction: isize) {
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

    pub(crate) fn adjust_startup_effort(&mut self, direction: isize) {
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

    pub(crate) fn drain_worker_events(&mut self) {
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

    pub(crate) fn push_final_brief(&mut self, path: &str) {
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

    pub(crate) fn push_system(&mut self, line: impl Into<String>) {
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

    pub(crate) fn enqueue_reveal(&mut self, line: impl Into<String>) {
        self.reveal_queue.push_back(line.into());
    }

    pub(crate) fn advance_reveal(&mut self) {
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

    pub(crate) fn start_next_reveal_line(&mut self) {
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

    pub(crate) fn open_auth_screen(&mut self, message: String, force_next_start: bool) {
        let mut onboarding = Onboarding::new(self.mode);
        onboarding.step = OnboardingStep::Auth;
        onboarding.message = message;
        self.onboarding = Some(onboarding);
        self.status = self.lang.choose("авторизация", "auth").to_string();
        if force_next_start {
            self.save_current_config(false);
        }
    }

    pub(crate) fn ensure_auth_ready_for_current_mode(&mut self) -> bool {
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

    pub(crate) fn suggestions(&self) -> Vec<CommandSpec> {
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

    pub(crate) fn complete_command(&mut self) {
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

    pub(crate) fn submit_input(&mut self) {
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

    pub(crate) fn handle_command(&mut self, line: &str) {
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

    pub(crate) fn apply_mode(&mut self, mode: Mode) {
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

    pub(crate) fn start_chat(&mut self, message: String) {
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

    pub(crate) fn start_task(&mut self, task: String) {
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

    pub(crate) fn insert_char(&mut self, ch: char) {
        self.input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub(crate) fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let prev = previous_boundary(&self.input, self.cursor);
        self.input.drain(prev..self.cursor);
        self.cursor = prev;
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn delete(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }

        let next = next_boundary(&self.input, self.cursor);
        self.input.drain(self.cursor..next);
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn move_left(&mut self) {
        self.cursor = previous_boundary(&self.input, self.cursor);
    }

    pub(crate) fn move_right(&mut self) {
        self.cursor = next_boundary(&self.input, self.cursor);
    }

    pub(crate) fn move_word_left(&mut self) {
        self.cursor = previous_word_boundary(&self.input, self.cursor);
    }

    pub(crate) fn move_word_right(&mut self) {
        self.cursor = next_word_boundary(&self.input, self.cursor);
    }

    pub(crate) fn move_line_start(&mut self) {
        self.cursor = line_start_boundary(&self.input, self.cursor);
    }

    pub(crate) fn move_line_end(&mut self) {
        self.cursor = line_end_boundary(&self.input, self.cursor);
    }

    pub(crate) fn delete_word_back(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = previous_word_boundary(&self.input, self.cursor);
        self.input.drain(start..self.cursor);
        self.cursor = start;
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn delete_word_forward(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        let end = next_word_boundary(&self.input, self.cursor);
        self.input.drain(self.cursor..end);
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn kill_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.input.drain(..self.cursor);
        self.cursor = 0;
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn kill_after_cursor(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        self.input.drain(self.cursor..);
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn history_prev(&mut self) {
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

    pub(crate) fn history_next(&mut self) {
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

    pub(crate) fn push_command_invocation(&mut self, command: &str) {
        self.push_system(format!("❯ {command}"));
    }

    pub(crate) fn push_command_result(&mut self, result: impl Into<String>) {
        self.push_system(format!("  ⎿  {}", result.into()));
    }

    pub(crate) fn show_footer_notice(&mut self, message: impl Into<String>) {
        self.footer_notice = Some((message.into(), Instant::now()));
    }

    pub(crate) fn expire_footer_notice(&mut self) {
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

    pub(crate) fn refresh_command_palette_state(&mut self) {
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

    pub(crate) fn refresh_footer_right_state(&mut self) {
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

    pub(crate) fn handle_ctrl_c(&mut self) {
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
