use super::*;

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

impl App {
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
}
