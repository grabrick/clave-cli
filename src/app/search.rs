use super::*;

impl App {
    pub(crate) fn open_search(&mut self) {
        self.overlay = Overlay::Search;
        self.search_query.clear();
        self.search_index = 0;
        self.status = self.lang.choose("поиск", "search").to_string();
    }

    pub(crate) fn close_search(&mut self) {
        self.overlay = Overlay::None;
        self.search_query.clear();
        self.status = self.lang.choose("готов", "ready").to_string();
    }

    pub(crate) fn search_matches(&self) -> Vec<usize> {
        if self.search_query.trim().is_empty() {
            return Vec::new();
        }
        let needle = self.search_query.to_lowercase();
        self.transcript
            .iter()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&needle))
            .map(|(index, _)| index)
            .collect()
    }

    pub(crate) fn search_input(&mut self, ch: char) {
        self.search_query.push(ch);
        self.search_index = 0;
        self.sync_search_scroll();
    }

    pub(crate) fn search_backspace(&mut self) {
        self.search_query.pop();
        self.search_index = 0;
        self.sync_search_scroll();
    }

    pub(crate) fn search_step(&mut self, direction: isize) {
        let matches = self.search_matches();
        if matches.is_empty() {
            return;
        }
        let len = matches.len();
        self.search_index = if direction < 0 {
            (self.search_index + len - 1) % len
        } else {
            (self.search_index + 1) % len
        };
        self.sync_search_scroll();
    }

    /// В inline-режиме скроллом владеет терминал, поэтому поиск только удерживает
    /// корректный индекс совпадения (без программной прокрутки ленты).
    pub(crate) fn sync_search_scroll(&mut self) {
        let matches = self.search_matches();
        if matches.is_empty() {
            return;
        }
        self.search_index = self.search_index.min(matches.len() - 1);
    }
}
