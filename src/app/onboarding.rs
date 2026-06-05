use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OnboardingStep {
    Provider,
    Auth,
    Settings,
}

pub(crate) struct Onboarding {
    pub(crate) step: OnboardingStep,
    pub(crate) provider_index: usize,
    pub(crate) setting_index: usize,
    pub(crate) codex_installed: bool,
    pub(crate) claude_installed: bool,
    pub(crate) codex_authenticated: bool,
    pub(crate) claude_authenticated: bool,
    pub(crate) codex_status: String,
    pub(crate) claude_status: String,
    pub(crate) message: String,
}

impl Onboarding {
    pub(crate) fn new(mode: Mode) -> Self {
        let codex = codex_auth_probe();
        let claude = claude_auth_probe();

        Self {
            step: OnboardingStep::Provider,
            provider_index: provider_index(mode),
            setting_index: 0,
            codex_installed: codex.installed,
            claude_installed: claude.installed,
            codex_authenticated: codex.authenticated,
            claude_authenticated: claude.authenticated,
            codex_status: codex.status,
            claude_status: claude.status,
            message: "Выбери, какие модели будут работать в Clave.".to_string(),
        }
    }

    pub(crate) fn refresh_auth(&mut self) {
        let codex = codex_auth_probe();
        let claude = claude_auth_probe();
        self.codex_installed = codex.installed;
        self.claude_installed = claude.installed;
        self.codex_authenticated = codex.authenticated;
        self.claude_authenticated = claude.authenticated;
        self.codex_status = codex.status;
        self.claude_status = claude.status;
    }
}

pub(crate) struct AuthProbe {
    pub(crate) installed: bool,
    pub(crate) authenticated: bool,
    pub(crate) status: String,
}

impl App {
    pub(crate) fn open_auth_screen(&mut self, message: String, force_next_start: bool) {
        let mut onboarding = Onboarding::new(self.mode);
        onboarding.step = OnboardingStep::Auth;
        onboarding.message = message;
        self.onboarding = Some(onboarding);
        self.status = self.lang.choose("авторизация", "auth").to_string();
        if force_next_start {
            self.save_current_config(false);
        }
    }

    pub(crate) fn ensure_auth_ready_for_current_mode(&mut self) -> bool {
        let onboarding = Onboarding::new(self.mode);
        if auth_requirements_ready(self.mode, &onboarding) {
            return true;
        }

        let missing = missing_auth_text(self.mode, &onboarding, self.lang);
        let message = format!(
            "{} {}. {}",
            self.lang
                .choose("Для режима нужен логин:", "Login required for mode:"),
            missing,
            self.lang.choose(
                "Нажми C для Codex login или L для Claude auth login.",
                "Press C for Codex login or L for Claude auth login."
            )
        );
        self.open_auth_screen(message.clone(), true);
        self.show_footer_notice(message);
        false
    }

    pub(crate) fn ensure_auth_ready_for_provider(&mut self, provider: Provider) -> bool {
        let onboarding = Onboarding::new(self.mode);
        if provider_auth_ready(provider, &onboarding) {
            return true;
        }

        let missing = missing_provider_auth_text(provider, &onboarding, self.lang);
        let message = format!(
            "{} {}. {}",
            self.lang.choose(
                "Для простого чата нужен логин:",
                "Login required for direct chat:"
            ),
            missing,
            self.lang.choose(
                "Нажми C для Codex login или L для Claude auth login.",
                "Press C for Codex login or L for Claude auth login."
            )
        );
        self.open_auth_screen(message.clone(), true);
        self.show_footer_notice(message);
        false
    }
}
