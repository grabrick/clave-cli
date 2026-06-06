use super::*;

impl App {
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
                self.last_run = find_last_run(&self.transcript);
                // Inline append-only: восстановленный чат надо перепечатать в буфер,
                // иначе flush_history его не покажет (печатает только pending_output).
                self.pending_output.extend(self.transcript.iter().cloned());
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
            Ok(()) => self.push_command_result(format!(
                "{} {}",
                self.lang.choose("Чат назван:", "Chat named:"),
                title
            )),
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
            let err_line = format!(
                "{} {}",
                self.lang
                    .choose("Не удалось сохранить чат:", "Failed to save chat:"),
                err
            );
            self.transcript.push(err_line.clone());
            self.pending_output.push_back(err_line);
        }

        self.transcript.push(line.clone());
        if self.transcript.len() > MAX_TRANSCRIPT_LINES {
            let remove_count = self.transcript.len() - MAX_TRANSCRIPT_LINES;
            self.transcript.drain(0..remove_count);
        }
        // Append-only история: строка уходит в обычный буфер терминала через
        // insert_before (см. runtime::flush_history); внутри истории не мутируется.
        self.pending_output.push_back(line);
    }
}
