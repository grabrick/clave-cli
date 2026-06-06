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

impl App {
    /// Идёт ли сейчас анимация (loader / footer-notice / shimmer effort).
    pub(crate) fn is_animating(&self) -> bool {
        // footer_right_changed_at намеренно НЕ учитываем (ротация раз в 8с не должна
        // будить простой). Reveal удалён — история append-only (insert_before).
        self.running || self.footer_notice.is_some() || self.overlay == Overlay::Effort
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
}
