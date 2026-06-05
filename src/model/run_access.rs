use super::ChatMode;

/// Права запуска CLI для одного вызова. `ChatMode` отвечает за UI (label/цвет),
/// `RunAccess` — за то, какие инструменты/песочница уходят провайдеру. Фазам
/// плана нужны наборы, которых нет среди пользовательских режимов.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunAccess {
    /// Обычный чат: права берутся из текущего режима.
    Chat(ChatMode),
    /// Фаза 1 плана: только чтение кода, без правок и команд.
    PlanReadonly,
    /// Фаза 2 плана: полный доступ для выполнения одобренного плана.
    PlanExecute,
}

impl RunAccess {
    pub(crate) fn claude_tools(self) -> &'static str {
        match self {
            RunAccess::Chat(mode) => mode.claude_tools(),
            RunAccess::PlanReadonly => "Read Grep Glob",
            RunAccess::PlanExecute => "Read Edit Write Bash Grep Glob",
        }
    }

    pub(crate) fn claude_permission(self) -> &'static str {
        match self {
            RunAccess::Chat(mode) => mode.claude_permission(),
            RunAccess::PlanReadonly => "default",
            RunAccess::PlanExecute => "bypassPermissions",
        }
    }

    pub(crate) fn codex_sandbox(self) -> &'static str {
        match self {
            RunAccess::Chat(mode) => mode.codex_sandbox(),
            RunAccess::PlanReadonly => "read-only",
            RunAccess::PlanExecute => "workspace-write",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_access_maps_phase_flags() {
        // Фаза 1 — чтение без правок и без Bash.
        assert!(RunAccess::PlanReadonly.claude_tools().contains("Read"));
        assert!(!RunAccess::PlanReadonly.claude_tools().contains("Edit"));
        assert!(!RunAccess::PlanReadonly.claude_tools().contains("Bash"));
        assert_eq!(RunAccess::PlanReadonly.codex_sandbox(), "read-only");

        // Фаза 2 — полный доступ.
        assert!(RunAccess::PlanExecute.claude_tools().contains("Bash"));
        assert_eq!(
            RunAccess::PlanExecute.claude_permission(),
            "bypassPermissions"
        );
        assert_eq!(RunAccess::PlanExecute.codex_sandbox(), "workspace-write");

        // Chat делегирует режиму.
        assert_eq!(RunAccess::Chat(ChatMode::Discussion).claude_tools(), "");
        assert_eq!(
            RunAccess::Chat(ChatMode::FullAccess).codex_sandbox(),
            "workspace-write"
        );
    }
}
