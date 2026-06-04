#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    CodexOnly,
    ClaudeOnly,
    ClaudeCodex,
}

impl Mode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Mode::CodexOnly => "codex-only",
            Mode::ClaudeOnly => "claude-only",
            Mode::ClaudeCodex => "claude-codex",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "codex-only" => Some(Mode::CodexOnly),
            "claude-only" => Some(Mode::ClaudeOnly),
            "claude-codex" => Some(Mode::ClaudeCodex),
            _ => None,
        }
    }

    pub(crate) fn needs_codex(self) -> bool {
        matches!(self, Mode::CodexOnly | Mode::ClaudeCodex)
    }

    pub(crate) fn needs_claude(self) -> bool {
        matches!(self, Mode::ClaudeOnly | Mode::ClaudeCodex)
    }
}
