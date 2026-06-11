use super::*;

impl App {
    pub(crate) fn refresh_current_chat_title(&mut self) {
        self.chat_title_custom = read_chat_title(&self.chat_path).is_some();
        self.chat_title = chat_display_title(&self.chat_path, &self.transcript, &self.chat_id);
    }

    pub(crate) fn set_chat_title_from_prompt_if_needed(&mut self, prompt: &str) {
        if self.chat_title_custom || first_prompt_title(&self.transcript).is_some() {
            return;
        }

        let title = truncate_chars(prompt.trim(), 72);
        if !title.is_empty() {
            self.chat_title = title;
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
        self.chat_title = self.chat_id.clone();
        self.chat_title_custom = false;
        self.transcript.clear();
        self.reset_scrollback();
        self.last_run = None;
        self.pending_plan = None;
        self.plan_flow = PlanFlow::None;
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
        self.push_command_result(format!(
            "{} {}",
            self.lang.choose("Новый чат:", "New chat:"),
            self.chat_id
        ));
    }

    pub(crate) fn resume_chat(&mut self, chat_id: &str) {
        let chat_id = sanitize_chat_id(chat_id);
        if chat_id.is_empty() {
            self.push_command_result(self.lang.choose(
                "Использование: /resume <id-чата>",
                "Usage: /resume <chat-id>",
            ));
            return;
        }

        let Some(path) = existing_chat_path(&self.chats_dir, &chat_id) else {
            self.push_command_result(self.lang.choose("Чат не найден.", "Chat not found."));
            return;
        };
        match load_chat_transcript(&path) {
            Ok(lines) if !lines.is_empty() => {
                self.chat_id = chat_id;
                self.chat_path = path;
                self.transcript = lines;
                self.refresh_current_chat_title();
                self.reset_scrollback();
                self.last_run = find_last_run(&self.transcript);
                self.pending_plan = None;
                self.plan_flow = PlanFlow::None;
                self.status = self.lang.choose("чат открыт", "chat resumed").to_string();
                self.save_current_config(true);
                self.push_command_result(format!(
                    "{} {}",
                    self.lang.choose("Чат открыт:", "Chat resumed:"),
                    self.chat_id
                ));
            }
            Ok(_) => self.push_command_result(
                self.lang
                    .choose("Чат пустой или повреждён.", "Chat is empty or corrupted."),
            ),
            Err(err) => self.push_command_result(format!(
                "{} {}",
                self.lang
                    .choose("Не удалось открыть чат:", "Failed to open chat:"),
                err
            )),
        }
    }

    pub(crate) fn open_chats_picker(&mut self) {
        let chats = list_saved_chats(&self.chats_dir, 20);
        if chats.is_empty() {
            self.push_command_result(
                self.lang
                    .choose("Сохранённых чатов пока нет.", "No saved chats yet."),
            );
            return;
        }
        self.chats_index = chats
            .iter()
            .position(|chat| chat.id == self.chat_id)
            .unwrap_or(0);
        self.chats_picker = chats;
        self.overlay = Overlay::Chats;
        self.status = self.lang.choose("чаты", "chats").to_string();
    }

    pub(crate) fn clear_small_chats(&mut self) {
        let chats = list_saved_chats(&self.chats_dir, usize::MAX);
        let mut removed = 0;
        for chat in chats {
            if chat.id == self.chat_id || chat.lines >= 3 {
                continue;
            }
            if let Some(path) = existing_chat_path(&self.chats_dir, &chat.id) {
                if fs::remove_file(&path).is_ok() {
                    removed += 1;
                }
            }
        }
        self.push_command_result(format!(
            "{} {}",
            self.lang
                .choose("Удалено мелких чатов:", "Removed small chats:"),
            removed
        ));
    }

    pub(crate) fn clear_all_chats(&mut self) {
        let chats = list_saved_chats(&self.chats_dir, usize::MAX);
        let mut removed = 0;
        for chat in chats {
            if chat.id == self.chat_id {
                continue;
            }
            if let Some(path) = existing_chat_path(&self.chats_dir, &chat.id) {
                if fs::remove_file(&path).is_ok() {
                    removed += 1;
                }
            }
        }
        self.push_command_result(format!(
            "{} {}",
            self.lang.choose("Удалено чатов:", "Removed chats:"),
            removed
        ));
    }

    pub(crate) fn rename_current_chat(&mut self, title: &str) {
        let title = title.trim();
        if title.is_empty() {
            self.push_command_result(
                self.lang
                    .choose("Использование: /name <заголовок>", "Usage: /name <title>"),
            );
            return;
        }
        match set_chat_title(&self.chat_path, &self.chat_id, title) {
            Ok(()) => {
                self.chat_title = truncate_chars(title, 72);
                self.chat_title_custom = true;
                self.push_command_result(format!(
                    "{} {}",
                    self.lang.choose("Чат назван:", "Chat named:"),
                    title
                ));
            }
            Err(err) => self.push_command_result(format!(
                "{} {}",
                self.lang
                    .choose("Не удалось переименовать:", "Failed to rename:"),
                err
            )),
        }
    }
}

impl App {
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

        // Строка добавляется только в transcript; нижний viewport покажет её в
        // хвосте, а runtime::flush_overflow вытеснит старое в скроллбэк по мере
        // надобности (append-only история).
        self.transcript.push(line);
        if self.transcript.len() > MAX_TRANSCRIPT_LINES {
            let remove_count = self.transcript.len() - MAX_TRANSCRIPT_LINES;
            self.transcript.drain(0..remove_count);
            // Срезанные строки были из уже вытесненной «головы» — сдвигаем границу.
            self.scrollback_count = self.scrollback_count.saturating_sub(remove_count);
        }
    }

    /// Сбрасывает границу вытеснения: вызывать при ПОЛНОЙ замене transcript
    /// (новый чат, /resume, /clear) — содержимое сменилось, прошлая «голова»
    /// больше не относится к текущей ленте.
    pub(crate) fn reset_scrollback(&mut self) {
        self.scrollback_count = 0;
        self.flush_state = TranscriptRenderState::default();
        // Уже напечатанную историю из нативного скроллбэка иначе не убрать —
        // просим рендер полностью очистить терминал (экран + скроллбэк).
        self.pending_clear_screen = true;
    }
}
