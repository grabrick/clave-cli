use super::Language;

/// Хоткей: клавиши + локализованное описание. Единый источник для панели
/// подсказок (`?`) и для футера — чтобы не дублировать строки по рендерам.
pub(crate) struct ShortcutSpec {
    pub(crate) keys: &'static str,
    pub(crate) ru: &'static str,
    pub(crate) en: &'static str,
}

impl ShortcutSpec {
    pub(crate) fn describe(&self, lang: Language) -> &'static str {
        lang.choose(self.ru, self.en)
    }
}

/// Клавиши переключения режима чата (показываются и в футере, и в подсказках).
pub(crate) const MODE_SWITCH_KEYS: &str = "Shift+Tab";

pub(crate) const SHORTCUTS: &[ShortcutSpec] = &[
    ShortcutSpec {
        keys: MODE_SWITCH_KEYS,
        ru: "режим",
        en: "mode",
    },
    ShortcutSpec {
        keys: "Enter",
        ru: "отправить",
        en: "send",
    },
    ShortcutSpec {
        keys: "Ctrl+J",
        ru: "новая строка",
        en: "newline",
    },
    ShortcutSpec {
        keys: "Tab",
        ru: "автодополнение",
        en: "complete",
    },
    ShortcutSpec {
        keys: "↑↓",
        ru: "скролл / выбор",
        en: "scroll / pick",
    },
    ShortcutSpec {
        keys: "Ctrl+P/N",
        ru: "история ввода",
        en: "input history",
    },
    ShortcutSpec {
        keys: "PageUp/PageDown",
        ru: "скролл",
        en: "scroll",
    },
    ShortcutSpec {
        keys: "Ctrl+R",
        ru: "поиск",
        en: "search",
    },
    ShortcutSpec {
        keys: "Ctrl+A/E",
        ru: "начало/конец",
        en: "start/end",
    },
    ShortcutSpec {
        keys: "Ctrl+W/U/K",
        ru: "удалить",
        en: "delete",
    },
    ShortcutSpec {
        keys: "Alt+←→",
        ru: "по словам",
        en: "by word",
    },
    ShortcutSpec {
        keys: "Esc",
        ru: "сброс",
        en: "clear",
    },
    ShortcutSpec {
        keys: "Ctrl+C ×2",
        ru: "выход",
        en: "exit",
    },
    ShortcutSpec {
        keys: "?",
        ru: "скрыть",
        en: "hide",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_include_mode_switch_and_are_filled() {
        assert!(SHORTCUTS.iter().any(|s| s.keys == MODE_SWITCH_KEYS));
        assert!(SHORTCUTS
            .iter()
            .all(|s| !s.keys.is_empty() && !s.ru.is_empty() && !s.en.is_empty()));
        assert_eq!(
            SHORTCUTS[0].describe(Language::En),
            "mode",
            "переключение режима — первым в списке"
        );
    }
}
