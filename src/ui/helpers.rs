use super::*;

pub(crate) fn composer_height(app: &App, width: u16) -> u16 {
    let lines = input_lines_wrapped(&app.input, width).len() as u16;
    (lines + 3).clamp(4, 11)
}

pub(crate) fn initial_transcript(_lang: Language) -> Vec<String> {
    Vec::new()
}

pub(crate) fn provider_count() -> usize {
    4
}

pub(crate) fn provider_mode(index: usize) -> Mode {
    match index {
        0 => Mode::CodexOnly,
        1 => Mode::ClaudeCodex,
        2 => Mode::CodexClaude,
        3 => Mode::ClaudeOnly,
        _ => Mode::CodexOnly,
    }
}

pub(crate) fn provider_index(mode: Mode) -> usize {
    match mode {
        Mode::CodexOnly => 0,
        Mode::ClaudeCodex => 1,
        Mode::CodexClaude => 2,
        Mode::ClaudeOnly => 3,
    }
}

pub(crate) fn provider_description(mode: Mode, lang: Language) -> &'static str {
    match mode {
        Mode::CodexOnly => lang.choose("Codex пишет и ревьюит", "Codex drafts and reviews"),
        Mode::ClaudeCodex => lang.choose(
            "Claude пишет, Codex ревьюит",
            "Claude drafts, Codex reviews",
        ),
        Mode::CodexClaude => lang.choose(
            "Codex пишет, Claude ревьюит",
            "Codex drafts, Claude reviews",
        ),
        Mode::ClaudeOnly => lang.choose("Claude пишет и ревьюит", "Claude drafts and reviews"),
    }
}

pub(crate) fn input_lines_wrapped(input: &str, width: u16) -> Vec<String> {
    let content_width = width.saturating_sub(2).max(1) as usize;
    if input.is_empty() {
        return vec![String::new()];
    }

    let mut rows = Vec::new();
    for line in input.split('\n') {
        rows.extend(wrap_terminal_text_preserving_spaces(line, content_width));
    }
    rows
}

pub(crate) fn input_cursor_position_wrapped(
    input: &str,
    cursor: usize,
    width: u16,
) -> (usize, usize) {
    let content_width = width.saturating_sub(2).max(1) as usize;
    let before = &input[..cursor];
    let parts = before.split('\n').collect::<Vec<_>>();
    let mut visual_line = 0usize;
    let mut visual_col = 0usize;

    for (index, line) in parts.iter().enumerate() {
        let len = line.chars().count();
        if index + 1 == parts.len() {
            visual_line += len / content_width;
            visual_col = len % content_width;
        } else {
            visual_line += (len / content_width) + 1;
        }
    }

    (visual_line, visual_col)
}

pub(crate) fn wrap_terminal_line(text: &str, width: u16) -> Vec<String> {
    let max_chars = width.saturating_sub(1).max(1) as usize;
    wrap_terminal_text_preserving_spaces(text, max_chars)
}

pub(crate) fn wrap_terminal_text_preserving_spaces(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut rows = Vec::new();
    let mut current = String::new();
    // Длину ведём инкрементально: `current.chars().count()` в цикле давал O(n²)
    // на длинных строках, а функция вызывается для каждой строки на каждом кадре.
    let mut current_len = 0usize;

    for ch in text.chars() {
        if ch == '\n' {
            rows.push(std::mem::take(&mut current));
            current_len = 0;
            continue;
        }

        if current_len >= max_chars {
            rows.push(std::mem::take(&mut current));
            current_len = 0;
        }
        current.push(ch);
        current_len += 1;
    }

    rows.push(current);
    rows
}

pub(crate) fn wrap_chars(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let max_chars = max_chars.max(1);
    let mut rows = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let current_len = current.chars().count();
        let word_len = word.chars().count();
        let extra_space = usize::from(!current.is_empty());

        if current_len + extra_space + word_len > max_chars && !current.is_empty() {
            rows.push(current);
            current = String::new();
        }

        if word_len > max_chars {
            if !current.is_empty() {
                rows.push(current);
                current = String::new();
            }

            let mut chunk = String::new();
            for ch in word.chars() {
                if chunk.chars().count() >= max_chars {
                    rows.push(chunk);
                    chunk = String::new();
                }
                chunk.push(ch);
            }
            if !chunk.is_empty() {
                current = chunk;
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        rows.push(current);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_preserving_spaces_is_stable() {
        // Характеризующий тест: оптимизация (O(n)) обязана давать тот же результат.
        assert_eq!(wrap_terminal_text_preserving_spaces("", 5), vec![""]);
        assert_eq!(wrap_terminal_text_preserving_spaces("abc", 5), vec!["abc"]);
        assert_eq!(
            wrap_terminal_text_preserving_spaces("abcde", 5),
            vec!["abcde"]
        );
        assert_eq!(
            wrap_terminal_text_preserving_spaces("abcdef", 5),
            vec!["abcde", "f"]
        );
        assert_eq!(
            wrap_terminal_text_preserving_spaces("ab\ncd", 5),
            vec!["ab", "cd"]
        );
        // Юникод считается по символам, а не байтам.
        assert_eq!(
            wrap_terminal_text_preserving_spaces("абвгде", 5),
            vec!["абвгд", "е"]
        );
    }

    #[test]
    fn wrap_chars_keeps_words_whole() {
        // Слово не рвётся посреди буквы: «world» уезжает на новую строку целиком.
        assert_eq!(wrap_chars("hello world", 7), vec!["hello", "world"]);
        // Слово длиннее ширины влезть целиком не может — дробится по символам.
        assert_eq!(wrap_chars("abcdefgh", 5), vec!["abcde", "fgh"]);
        // Путь со спецсимволами короче ширины — переносится целиком, не по буквам.
        assert_eq!(
            wrap_chars("see src/app.rs now", 10),
            vec!["see", "src/app.rs", "now"]
        );
    }
}
