use super::*;

impl App {
    pub(crate) fn suggestions(&self) -> Vec<CommandSpec> {
        let Some(needle) = normalized_command_query(&self.input) else {
            return Vec::new();
        };

        COMMANDS
            .iter()
            .copied()
            .filter(|command| {
                command.usage.starts_with(&needle) || command.insert.starts_with(&needle)
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

        let normalized_plain = normalized_plain_command(&line);
        if line.eq_ignore_ascii_case("logout") || normalized_plain == "logout" {
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
        } else if let Some(command_line) = normalize_command_line_for_execution(&line) {
            self.handle_command(&command_line);
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
                "codex-claude" => self.apply_mode(Mode::CodexClaude),
                _ => self.push_system(self.lang.choose(
                    "Использование: /mode codex-only|claude-only|claude-codex|codex-claude",
                    "Usage: /mode codex-only|claude-only|claude-codex|codex-claude",
                )),
            },
            "/settings" => self.open_settings(),
            "/chat-model" => match Provider::from_str(rest.trim()) {
                Some(provider) => self.set_direct_provider(provider),
                None => self.push_system(self.lang.choose(
                    "Использование: /chat-model codex|claude",
                    "Usage: /chat-model codex|claude",
                )),
            },
            "/theme" => match Theme::from_str(rest.trim()) {
                Some(theme) => self.set_theme(theme),
                None => self.push_system(self.lang.choose(
                    "Использование: /theme purple|cyan|rose|amber|mono",
                    "Usage: /theme purple|cyan|rose|amber|mono",
                )),
            },
            "/roles" => {
                let providers = rest
                    .split(|ch: char| ch.is_whitespace() || matches!(ch, '>' | '-' | '→'))
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>();
                match providers.as_slice() {
                    [architect, reviewer] => {
                        match (Provider::from_str(architect), Provider::from_str(reviewer)) {
                            (Some(architect), Some(reviewer)) => {
                                self.set_roles(architect, reviewer);
                            }
                            _ => self.push_system(self.lang.choose(
                                "Использование: /roles codex|claude codex|claude",
                                "Usage: /roles codex|claude codex|claude",
                            )),
                        }
                    }
                    _ => self.push_system(self.lang.choose(
                        "Использование: /roles <исполнитель> <ревьюер>",
                        "Usage: /roles <executor> <reviewer>",
                    )),
                }
            }
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
                    "{}={} {}={} {}={} chat={} theme={} roles={}>{} {}={} {}={}",
                    self.lang.choose("режим", "mode"),
                    self.mode.as_str(),
                    self.lang.choose("язык", "lang"),
                    self.lang.as_str(),
                    self.lang.choose("раунды", "rounds"),
                    self.rounds,
                    self.direct_provider.as_str(),
                    self.theme.as_str(),
                    self.mode.architect_provider().as_str(),
                    self.mode.reviewer_provider().as_str(),
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
}
