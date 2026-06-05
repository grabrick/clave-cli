use super::Language;
use crate::prelude::*;

/// Режим прямого чата, переключается по Shift+Tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ChatMode {
    #[default]
    Discussion,
    Plan,
    FullAccess,
}

impl ChatMode {
    pub(crate) fn next(self) -> Self {
        match self {
            ChatMode::Discussion => ChatMode::Plan,
            ChatMode::Plan => ChatMode::FullAccess,
            ChatMode::FullAccess => ChatMode::Discussion,
        }
    }

    pub(crate) fn label(self, lang: Language) -> &'static str {
        match self {
            ChatMode::Discussion => lang.choose(">> Обсуждение", ">> Discussion"),
            ChatMode::Plan => lang.choose(">> Режим плана", ">> Plan Mode"),
            ChatMode::FullAccess => lang.choose(">> Полный доступ", ">> Full Access"),
        }
    }

    pub(crate) fn color(self) -> Color {
        match self {
            ChatMode::Discussion => Color::Gray,
            ChatMode::Plan => Color::Indexed(80),        // cyan
            ChatMode::FullAccess => Color::Indexed(120), // green
        }
    }

    /// Инструменты claude (`--tools`). Пусто = чистый чат. Plan правит файлы, но без Bash.
    pub(crate) fn claude_tools(self) -> &'static str {
        match self {
            ChatMode::Discussion => "",
            ChatMode::Plan => "Read Edit Write Grep Glob",
            ChatMode::FullAccess => "Read Edit Write Bash Grep Glob",
        }
    }

    pub(crate) fn claude_permission(self) -> &'static str {
        match self {
            ChatMode::Discussion => "default",
            ChatMode::Plan => "acceptEdits",
            ChatMode::FullAccess => "bypassPermissions",
        }
    }

    pub(crate) fn codex_sandbox(self) -> &'static str {
        match self {
            ChatMode::Discussion => "read-only",
            ChatMode::Plan | ChatMode::FullAccess => "workspace-write",
        }
    }

    pub(crate) fn prompt_hint(self, lang: Language) -> &'static str {
        match self {
            ChatMode::Discussion => lang.choose(
                "Просто ответь на сообщение. Не используй инструменты и не меняй файлы.",
                "Just answer the message. Do not use tools or modify files.",
            ),
            ChatMode::Plan => lang.choose(
                "Составь план и реализуй его: можешь читать и править файлы, но не выполняй произвольные команды (Bash).",
                "Make a plan and carry it out: you may read and edit files, but do not run arbitrary shell commands (Bash).",
            ),
            ChatMode::FullAccess => lang.choose(
                "Ты автономный агент: читай, создавай и правь файлы и выполняй команды в рабочей директории — решай всё сам.",
                "You are an autonomous agent: read, create and edit files and run commands in the working directory — decide everything yourself.",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_mode_cycles_and_maps_flags() {
        assert_eq!(ChatMode::default(), ChatMode::Discussion);
        assert_eq!(ChatMode::Discussion.next(), ChatMode::Plan);
        assert_eq!(ChatMode::Plan.next(), ChatMode::FullAccess);
        assert_eq!(ChatMode::FullAccess.next(), ChatMode::Discussion);

        // Discussion — чистый чат
        assert_eq!(ChatMode::Discussion.claude_tools(), "");
        assert_eq!(ChatMode::Discussion.codex_sandbox(), "read-only");

        // Plan правит файлы, но без Bash
        assert!(ChatMode::Plan.claude_tools().contains("Edit"));
        assert!(!ChatMode::Plan.claude_tools().contains("Bash"));
        assert_eq!(ChatMode::Plan.codex_sandbox(), "workspace-write");

        // Full — всё, включая Bash
        assert!(ChatMode::FullAccess.claude_tools().contains("Bash"));
        assert_eq!(
            ChatMode::FullAccess.claude_permission(),
            "bypassPermissions"
        );
    }
}
