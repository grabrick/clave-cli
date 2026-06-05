use super::*;

#[derive(Clone, Copy)]
pub(crate) struct SettingsSnapshot {
    pub(crate) direct_provider: Provider,
    pub(crate) theme: Theme,
    pub(crate) mode: Mode,
    pub(crate) rounds: usize,
    pub(crate) lang: Language,
}

impl App {
    pub(crate) fn settings_snapshot(&self) -> SettingsSnapshot {
        SettingsSnapshot {
            direct_provider: self.direct_provider,
            theme: self.theme,
            mode: self.mode,
            rounds: self.rounds,
            lang: self.lang,
        }
    }

    pub(crate) fn restore_settings_snapshot(&mut self, snapshot: SettingsSnapshot) {
        self.direct_provider = snapshot.direct_provider;
        self.theme = snapshot.theme;
        self.set_mode(snapshot.mode);
        self.rounds = snapshot.rounds;
        self.lang = snapshot.lang;
    }

    pub(crate) fn open_settings(&mut self) {
        self.open_settings_from("/settings");
    }

    pub(crate) fn open_settings_from(&mut self, command: &str) {
        self.push_command_invocation(command);
        self.settings_original = Some(self.settings_snapshot());
        self.settings_focus = 0;
        self.settings_open = true;
        self.status = "settings".to_string();
    }

    pub(crate) fn settings_rows(&self) -> usize {
        6
    }

    pub(crate) fn adjust_settings_focus(&mut self, direction: isize) {
        if direction < 0 {
            self.settings_focus = self.settings_focus.saturating_sub(1);
        } else {
            self.settings_focus = (self.settings_focus + 1).min(self.settings_rows() - 1);
        }
    }

    pub(crate) fn adjust_settings_value(&mut self, direction: isize) {
        match self.settings_focus {
            0 => self.direct_provider = self.direct_provider.toggled(),
            1 => {
                let architect = self.mode.architect_provider().toggled();
                let reviewer = self.mode.reviewer_provider();
                self.set_mode(Mode::from_roles(architect, reviewer));
            }
            2 => {
                let architect = self.mode.architect_provider();
                let reviewer = self.mode.reviewer_provider().toggled();
                self.set_mode(Mode::from_roles(architect, reviewer));
            }
            3 => self.theme = self.theme.shifted(direction),
            4 => {
                if direction < 0 {
                    self.rounds = self.rounds.saturating_sub(1).max(1);
                } else {
                    self.rounds = (self.rounds + 1).min(9);
                }
            }
            5 => {
                self.lang = if self.lang == Language::Ru {
                    Language::En
                } else {
                    Language::Ru
                };
            }
            _ => {}
        }
    }

    pub(crate) fn set_direct_provider(&mut self, provider: Provider) {
        self.direct_provider = provider;
        self.save_current_config(true);
        self.push_system(format!(
            "{} {}.",
            self.lang
                .choose("Модель для простых сообщений:", "Direct chat model set to:"),
            provider.title()
        ));
    }

    pub(crate) fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.save_current_config(true);
        self.push_system(format!(
            "{} {}.",
            self.lang.choose("Цветовая гамма:", "Theme set to:"),
            theme.title()
        ));
    }

    pub(crate) fn set_roles(&mut self, architect: Provider, reviewer: Provider) {
        self.set_mode(Mode::from_roles(architect, reviewer));
        self.save_current_config(true);
        self.push_system(format!(
            "{} {} → {}.",
            self.lang.choose("Роли планирования:", "Planning roles:"),
            architect.title(),
            reviewer.title()
        ));
        self.ensure_auth_ready_for_current_mode();
    }

    pub(crate) fn settings_summary(&self) -> String {
        format!(
            "chat {} · theme {} · roles {}>{}",
            self.direct_provider.as_str(),
            self.theme.as_str(),
            self.mode.architect_provider().as_str(),
            self.mode.reviewer_provider().as_str()
        )
    }
}
