use super::*;

impl App {
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
}
