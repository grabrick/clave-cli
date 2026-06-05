use super::*;

impl App {
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
        let active = normalized_command_query(&self.input).is_some()
            && self.onboarding.is_none()
            && !self.overlay.is_open();
        if active {
            if self.command_palette_opened_at.is_none() {
                self.command_palette_opened_at = Some(Instant::now());
            }
            self.command_palette_query = self.input.clone();
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
