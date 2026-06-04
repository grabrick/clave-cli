#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Provider {
    Codex,
    Claude,
}

impl Provider {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Provider::Codex => "codex",
            Provider::Claude => "claude",
        }
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            Provider::Codex => "Codex",
            Provider::Claude => "Claude",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "codex" | "gpt" | "openai" => Some(Provider::Codex),
            "claude" | "anthropic" => Some(Provider::Claude),
            _ => None,
        }
    }

    pub(crate) fn toggled(self) -> Self {
        match self {
            Provider::Codex => Provider::Claude,
            Provider::Claude => Provider::Codex,
        }
    }
}
