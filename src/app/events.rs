use super::*;

#[derive(Debug)]
pub(crate) enum WorkerEvent {
    Line(String),
    ChatLine(String),
    Activity(String),
    Done(i32),
    ChatDone(&'static str, i32, Option<RunUsage>),
    PlanReady(&'static str, String, i32, Option<RunUsage>),
    Cancelled,
    Failed(String),
    /// Провайдер не залогинен — проверка ушла в воркер, чтобы не морозить UI.
    AuthMissing(&'static str),
}

pub(crate) enum ChatRunResult {
    Completed(i32, String, String, Option<RunUsage>),
    Cancelled,
}

/// Плавная «печатная машинка» для ответа: целиком готовый текст вскрывается
/// по символам со временем, пока полностью не уйдёт в историю.
pub(crate) struct Reveal {
    text: String,
    shown: usize,
    started: Instant,
}

/// Скорость «печати» (символов/сек). Короткие ответы появляются почти сразу,
/// длинные — заметно набираются. Прерывается любой клавишей (finish_reveal_now).
const REVEAL_CHARS_PER_SEC: usize = 600;

/// Сколько символов ответа должно быть «вскрыто» к моменту `elapsed_ms` при
/// текущей скорости, но не больше длины всего текста (`total`). Чистая функция —
/// чтобы раскадровку «печати» можно было проверить без таймеров и терминала.
fn reveal_chars_for(elapsed_ms: u128, total: usize) -> usize {
    let target = elapsed_ms.saturating_mul(REVEAL_CHARS_PER_SEC as u128) / 1000;
    (target as usize).min(total)
}

impl Reveal {
    /// Уже вскрытая часть текста (для отрисовки в живом блоке).
    pub(crate) fn shown_text(&self) -> String {
        self.text.chars().take(self.shown).collect()
    }
}

impl App {
    /// Идёт ли сейчас анимация (loader / reveal / footer-notice / shimmer / палитра).
    pub(crate) fn is_animating(&self) -> bool {
        // footer_right_changed_at намеренно НЕ учитываем (ротация раз в 8с не должна
        // будить простой). Палитру и reveal учитываем — у них живая анимация.
        self.running
            || self.reveal.is_some()
            || self.footer_notice.is_some()
            || self.overlay == Overlay::Effort
            || normalized_command_query(&self.input).is_some()
    }

    /// Двигает «печать» ответа по времени; по завершении фиксирует его в истории.
    pub(crate) fn advance_reveal(&mut self) {
        let finished = match &mut self.reveal {
            Some(reveal) => {
                let total = reveal.text.chars().count();
                reveal.shown = reveal_chars_for(reveal.started.elapsed().as_millis(), total);
                reveal.shown >= total
            }
            None => false,
        };
        if finished {
            self.commit_reveal();
        }
    }

    /// Мгновенно дописать ответ и зафиксировать (по любому нажатию клавиши).
    pub(crate) fn finish_reveal_now(&mut self) {
        if self.reveal.is_some() {
            self.commit_reveal();
        }
    }

    /// Переносит готовый reveal в историю (скроллбэк) и запускает отложенную очередь.
    fn commit_reveal(&mut self) {
        if let Some(reveal) = self.reveal.take() {
            for line in reveal.text.split('\n') {
                self.push_system(line.to_string());
            }
        }
        self.process_pending_messages();
    }

    /// Накопленный буфер ответа — сразу в историю (для не-чатовых путей: план, отмена).
    fn flush_reveal_buffer(&mut self) {
        for line in std::mem::take(&mut self.reveal_buffer) {
            self.push_system(line);
        }
    }

    pub(crate) fn push_run_activity(&mut self, activity: impl Into<String>) {
        let activity = activity.into();
        if activity.trim().is_empty() {
            return;
        }

        self.run_activity.push_back(activity);
        while self.run_activity.len() > 5 {
            self.run_activity.pop_front();
        }
    }

    pub(crate) fn record_worker_activity(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }

        if let Some(path) = trimmed.strip_prefix("Final brief: ") {
            self.push_run_activity(format!(
                "{} {}",
                self.lang.choose("итог:", "final:"),
                truncate_chars(path, 96)
            ));
        } else {
            self.push_run_activity(truncate_chars(trimmed, 120));
        }
    }

    pub(crate) fn drain_worker_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                WorkerEvent::Line(line) => {
                    self.record_worker_activity(&line);
                    if let Some(path) = line.strip_prefix("Final brief: ") {
                        let path = path.to_string();
                        self.last_run = Some(path.clone());
                        self.push_system(line);
                        self.push_final_brief(&path);
                    } else {
                        self.push_system(line);
                    }
                }
                // Строки ответа копим — покажем «печатной машинкой» на ChatDone.
                WorkerEvent::ChatLine(line) => self.reveal_buffer.push(line),
                WorkerEvent::Activity(line) => self.push_run_activity(line),
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
                    self.flush_reveal_buffer();
                    self.push_system(format!(
                        "{} {code}.",
                        self.lang
                            .choose("Clave завершился с кодом", "Clave finished with exit code")
                    ));
                    self.process_pending_messages();
                }
                WorkerEvent::ChatDone(provider, code, usage) => {
                    if let Some(usage) = usage {
                        self.usage.record(provider, usage);
                    }
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
                    // Запускаем «печать» ответа; пустой ответ → сразу очередь.
                    if self.reveal_buffer.is_empty() {
                        self.process_pending_messages();
                    } else {
                        self.reveal = Some(Reveal {
                            text: std::mem::take(&mut self.reveal_buffer).join("\n"),
                            shown: 0,
                            started: Instant::now(),
                        });
                    }
                }
                WorkerEvent::PlanReady(provider, plan, code, usage) => {
                    if let Some(usage) = usage {
                        self.usage.record(provider, usage);
                    }
                    self.running = false;
                    self.run_started_at = None;
                    self.run_label.clear();
                    self.run_token_estimate = None;
                    self.cancel_tx = None;
                    self.flush_reveal_buffer();

                    let task = match std::mem::replace(&mut self.plan_flow, PlanFlow::None) {
                        PlanFlow::Planning { task } => Some(task),
                        _ => None,
                    };

                    if code == 0 && !plan.trim().is_empty() {
                        if let Some(task) = task {
                            self.pending_plan = Some(PendingPlan { task, plan });
                            self.status = self.lang.choose("план готов", "plan ready").to_string();
                        }
                    } else {
                        self.pending_plan = None;
                        self.status = self.lang.choose("ошибка плана", "plan failed").to_string();
                    }
                }
                WorkerEvent::Cancelled => {
                    self.running = false;
                    self.run_started_at = None;
                    self.run_label.clear();
                    self.run_token_estimate = None;
                    self.cancel_tx = None;
                    self.reveal_buffer.clear();
                    self.reveal = None;
                    self.status = self.lang.choose("остановлено", "stopped").to_string();
                    self.push_system(
                        self.lang
                            .choose("⏹ Выполнение остановлено.", "⏹ Run stopped."),
                    );
                    // Отмена очищает очередь — пользователь нажал стоп.
                    self.pending_messages.clear();
                }
                WorkerEvent::Failed(message) => {
                    self.running = false;
                    self.run_started_at = None;
                    self.run_label.clear();
                    self.run_token_estimate = None;
                    self.cancel_tx = None;
                    self.flush_reveal_buffer();
                    self.status = self.lang.choose("ошибка", "failed").to_string();
                    self.push_system(message);
                    self.process_pending_messages();
                }
                WorkerEvent::AuthMissing(provider) => {
                    self.running = false;
                    self.run_started_at = None;
                    self.run_label.clear();
                    self.run_token_estimate = None;
                    self.run_activity.clear();
                    self.cancel_tx = None;
                    self.reveal_buffer.clear();
                    self.reveal = None;
                    self.pending_messages.clear();
                    if let Some(provider) = Provider::from_str(provider) {
                        self.prompt_provider_login(provider);
                    }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_unveils_gradually_then_caps_at_total() {
        let total = 300;
        // В нулевой момент ещё ничего не вскрыто.
        assert_eq!(reveal_chars_for(0, total), 0);
        // Со временем вскрывается строго больше — это и есть «печать», а не вспышка.
        let early = reveal_chars_for(100, total);
        let later = reveal_chars_for(250, total);
        assert!(
            early > 0 && early < total,
            "за 100мс — часть текста: {early}"
        );
        assert!(later > early, "позже вскрыто больше: {later} > {early}");
        // 600 симв/сек: 100мс ⇒ 60 символов, 250мс ⇒ 150.
        assert_eq!(early, 60);
        assert_eq!(later, 150);
        // Дольше длины текста расти нельзя — переполнения нет.
        assert_eq!(reveal_chars_for(10_000, total), total);
    }

    #[test]
    fn reveal_shown_text_is_a_char_prefix() {
        let reveal = Reveal {
            text: "Привет, мир".to_string(),
            shown: 6,
            started: Instant::now(),
        };
        // Режем по символам, а не байтам — кириллица не должна ломаться.
        assert_eq!(reveal.shown_text(), "Привет");
    }
}
