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
}

pub(crate) enum ChatRunResult {
    Completed(i32, String, String, Option<RunUsage>),
    Cancelled,
}

pub(crate) struct RevealLine {
    pub(crate) text: String,
    pub(crate) visible_chars: usize,
    pub(crate) transcript_index: usize,
    pub(crate) last_tick: Instant,
}

impl App {
    /// Идёт ли сейчас какая-либо анимация (typewriter, loader, переходы футера, shimmer effort).
    pub(crate) fn is_animating(&self) -> bool {
        // Намеренно НЕ включаем footer_right_changed_at: правый сегмент футера
        // ротируется каждые 8с и в простое будил 60fps-перерисовку всего экрана.
        // Его переход доживёт на 100мс-тике (чуть грубее), зато простой реально спит.
        self.running
            || self.reveal_active.is_some()
            || !self.reveal_queue.is_empty()
            || self.footer_notice.is_some()
            || self.overlay == Overlay::Effort
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
                WorkerEvent::ChatLine(line) => self.push_system(line),
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
                    self.process_pending_messages();
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
                    self.status = self.lang.choose("ошибка", "failed").to_string();
                    self.push_system(message);
                    self.process_pending_messages();
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
        self.scroll_offset = 0;
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
        // 6 символов за тик (было 3): печать завершается вдвое быстрее, значит
        // меньше времени держим 60fps-анимацию ради typewriter-эффекта.
        active.visible_chars = active
            .visible_chars
            .saturating_add(steps * 6)
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

    /// Пересобирает кэш рендера транскрипта только при изменении содержимого
    /// (хэш по width/theme/lang/строкам). Зовётся раз за кадр до отрисовки;
    /// в фазе выполнения транскрипт стабилен, поэтому дорогой рендер пропускается.
    /// Потолок скролла — по числу ОТРИСОВАННЫХ строк (из кэша), а не сообщений,
    /// иначе до верха длинного чата с переносами/боксами не долистать.
    pub(crate) fn scroll_ceiling(&self) -> usize {
        self.transcript_cache
            .as_ref()
            .map(|(_, lines)| lines.len() + 2)
            .unwrap_or(self.transcript.len())
    }

    pub(crate) fn refresh_transcript_cache(&mut self, width: u16) {
        let sig = transcript_signature(&self.transcript, width, self.theme, self.lang);
        let fresh = matches!(&self.transcript_cache, Some((cached, _)) if *cached == sig);
        if !fresh {
            let lines = transcript_lines(&self.transcript, self.lang, width, self.theme);
            self.transcript_cache = Some((sig, lines));
        }
    }
}
