use super::*;

/// План, ожидающий решения пользователя на гейте.
pub(crate) struct PendingPlan {
    pub(crate) task: String,
    pub(crate) plan: String,
}

/// Какая фаза плана сейчас выполняется (для роутинга завершения рана).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PlanFlow {
    None,
    Planning { task: String },
    Executing,
}

/// Решение на гейте по содержимому инпута.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PlanGateAction {
    Approve,
    Refine(String),
}

/// Пустой инпут — одобрить; непустой — доработать с замечанием.
pub(crate) fn plan_gate_action(input: &str) -> PlanGateAction {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        PlanGateAction::Approve
    } else {
        PlanGateAction::Refine(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_gate_action_routes_by_input() {
        assert_eq!(plan_gate_action("   "), PlanGateAction::Approve);
        assert_eq!(
            plan_gate_action("сделай проще"),
            PlanGateAction::Refine("сделай проще".to_string())
        );
    }
}
