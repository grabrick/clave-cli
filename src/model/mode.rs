use super::Provider;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    CodexOnly,
    ClaudeOnly,
    ClaudeCodex,
    CodexClaude,
}

impl Mode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Mode::CodexOnly => "codex-only",
            Mode::ClaudeOnly => "claude-only",
            Mode::ClaudeCodex => "claude-codex",
            Mode::CodexClaude => "codex-claude",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "codex-only" => Some(Mode::CodexOnly),
            "claude-only" => Some(Mode::ClaudeOnly),
            "claude-codex" => Some(Mode::ClaudeCodex),
            "codex-claude" => Some(Mode::CodexClaude),
            _ => None,
        }
    }

    pub(crate) fn needs_codex(self) -> bool {
        matches!(
            self,
            Mode::CodexOnly | Mode::ClaudeCodex | Mode::CodexClaude
        )
    }

    pub(crate) fn needs_claude(self) -> bool {
        matches!(
            self,
            Mode::ClaudeOnly | Mode::ClaudeCodex | Mode::CodexClaude
        )
    }

    pub(crate) fn architect_provider(self) -> Provider {
        match self {
            Mode::CodexOnly | Mode::CodexClaude => Provider::Codex,
            Mode::ClaudeOnly | Mode::ClaudeCodex => Provider::Claude,
        }
    }

    pub(crate) fn reviewer_provider(self) -> Provider {
        match self {
            Mode::CodexOnly | Mode::ClaudeCodex => Provider::Codex,
            Mode::ClaudeOnly | Mode::CodexClaude => Provider::Claude,
        }
    }

    pub(crate) fn from_roles(architect: Provider, reviewer: Provider) -> Self {
        match (architect, reviewer) {
            (Provider::Codex, Provider::Codex) => Mode::CodexOnly,
            (Provider::Claude, Provider::Claude) => Mode::ClaudeOnly,
            (Provider::Claude, Provider::Codex) => Mode::ClaudeCodex,
            (Provider::Codex, Provider::Claude) => Mode::CodexClaude,
        }
    }
}
