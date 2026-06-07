use super::*;

/// Активный inline-селектор: разобранный запрос модели + позиция курсора и отметки.
/// Курсор ходит по `0..=options.len()`; последний индекс — строка «Свой вариант».
pub(crate) struct AskState {
    pub(crate) prompt: AskPrompt,
    pub(crate) cursor: usize,
    pub(crate) checked: Vec<bool>,
}

impl AskState {
    fn new(prompt: AskPrompt) -> Self {
        let n = prompt.options.len();
        Self {
            prompt,
            cursor: 0,
            checked: vec![false; n],
        }
    }

    /// Всего строк в списке: варианты + строка «Свой вариант».
    pub(crate) fn rows(&self) -> usize {
        self.prompt.options.len() + 1
    }

    /// Курсор стоит на строке «Свой вариант»?
    pub(crate) fn on_custom_row(&self) -> bool {
        self.cursor == self.prompt.options.len()
    }
}

impl App {
    pub(crate) fn ask_active(&self) -> bool {
        self.ask.is_some()
    }

    /// Открывает селектор из отложенного запроса (после того как «допечаталась» проза).
    pub(crate) fn open_pending_ask(&mut self) {
        if let Some(prompt) = self.ask_prompt_pending.take() {
            self.ask = Some(AskState::new(prompt));
            self.status = self.lang.choose("выбор", "choose").to_string();
        }
    }

    fn clear_ask(&mut self) {
        self.ask = None;
        self.ask_prompt_pending = None;
    }

    pub(crate) fn ask_move(&mut self, delta: isize) {
        if let Some(state) = &mut self.ask {
            let rows = state.rows() as isize;
            state.cursor = (state.cursor as isize + delta).rem_euclid(rows) as usize;
        }
    }

    /// Space: отметить/снять вариант (только для множественного выбора).
    pub(crate) fn ask_toggle(&mut self) {
        if let Some(state) = &mut self.ask {
            if state.prompt.multi && !state.on_custom_row() {
                let i = state.cursor;
                state.checked[i] = !state.checked[i];
            }
        }
    }

    /// Enter: «Свой вариант» → закрыть (свободный ответ); иначе отправить выбор модели.
    pub(crate) fn ask_submit(&mut self) {
        let Some(state) = &self.ask else {
            return;
        };
        if state.on_custom_row() {
            self.ask = None;
            self.status = self.lang.choose("свой ответ", "custom").to_string();
            return;
        }
        let labels: Vec<String> = if state.prompt.multi {
            state
                .prompt
                .options
                .iter()
                .zip(&state.checked)
                .filter(|(_, &checked)| checked)
                .map(|(opt, _)| opt.label.clone())
                .collect()
        } else {
            vec![state.prompt.options[state.cursor].label.clone()]
        };
        if labels.is_empty() {
            return; // множественный без единой отметки — подтверждать нечего
        }
        self.ask = None;
        let joined = labels
            .iter()
            .map(|label| format!("«{label}»"))
            .collect::<Vec<_>>()
            .join(", ");
        let message = format!("{} {}", self.lang.choose("Выбрано:", "Selected:"), joined);
        // Выбор уходит обычным ходом: реплика «◆ Выбрано: …», модель продолжает с
        // учётом своего вопроса (он уже в контексте ленты).
        self.start_chat(message);
    }

    /// Esc: закрыть селектор и дать ответить текстом (вопрос остаётся в ленте).
    pub(crate) fn ask_cancel(&mut self) {
        if self.ask.take().is_some() {
            self.status = self.lang.choose("свой ответ", "custom").to_string();
        }
    }

    /// Сбрасывает селектор (отмена/смена чата) — публичная точка для событий.
    pub(crate) fn reset_ask(&mut self) {
        self.clear_ask();
    }
}
