use super::*;

impl App {
    pub(crate) fn insert_char(&mut self, ch: char) {
        self.input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub(crate) fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let prev = previous_boundary(&self.input, self.cursor);
        self.input.drain(prev..self.cursor);
        self.cursor = prev;
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn delete(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }

        let next = next_boundary(&self.input, self.cursor);
        self.input.drain(self.cursor..next);
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn move_left(&mut self) {
        self.cursor = previous_boundary(&self.input, self.cursor);
    }

    pub(crate) fn move_right(&mut self) {
        self.cursor = next_boundary(&self.input, self.cursor);
    }

    pub(crate) fn move_word_left(&mut self) {
        self.cursor = previous_word_boundary(&self.input, self.cursor);
    }

    pub(crate) fn move_word_right(&mut self) {
        self.cursor = next_word_boundary(&self.input, self.cursor);
    }

    pub(crate) fn move_line_start(&mut self) {
        self.cursor = line_start_boundary(&self.input, self.cursor);
    }

    pub(crate) fn move_line_end(&mut self) {
        self.cursor = line_end_boundary(&self.input, self.cursor);
    }

    pub(crate) fn delete_word_back(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = previous_word_boundary(&self.input, self.cursor);
        self.input.drain(start..self.cursor);
        self.cursor = start;
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn delete_word_forward(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        let end = next_word_boundary(&self.input, self.cursor);
        self.input.drain(self.cursor..end);
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn kill_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.input.drain(..self.cursor);
        self.cursor = 0;
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn kill_after_cursor(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        self.input.drain(self.cursor..);
        self.history_index = None;
        self.selected_suggestion = 0;
    }

    pub(crate) fn history_prev(&mut self) {
        if self.history_index.is_none() {
            let suggestions = self.suggestions();
            if !suggestions.is_empty() {
                self.selected_suggestion = self.selected_suggestion.saturating_sub(1);
                return;
            }
        }

        if self.history.is_empty() {
            return;
        }

        let next_index = match self.history_index {
            Some(index) => index.saturating_sub(1),
            None => self.history.len() - 1,
        };
        self.history_index = Some(next_index);
        self.input = self.history[next_index].clone();
        self.cursor = self.input.len();
    }

    pub(crate) fn history_next(&mut self) {
        if self.history_index.is_none() {
            let suggestions = self.suggestions();
            if !suggestions.is_empty() {
                self.selected_suggestion =
                    (self.selected_suggestion + 1).min(suggestions.len() - 1);
                return;
            }
        }

        let Some(index) = self.history_index else {
            return;
        };

        if index + 1 >= self.history.len() {
            self.history_index = None;
            self.input.clear();
        } else {
            let next_index = index + 1;
            self.history_index = Some(next_index);
            self.input = self.history[next_index].clone();
        }
        self.cursor = self.input.len();
    }
}
