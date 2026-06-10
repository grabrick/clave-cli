use super::*;

/// Стилизует и переносит одну строку истории в готовые `Line` для `insert_before`.
/// `state` ведёт code-block между строками (история append-only — state монотонен).
pub(crate) fn history_line_render(
    line: &str,
    lang: Language,
    width: u16,
    theme: Theme,
    state: &mut TranscriptRenderState,
) -> Vec<Line<'static>> {
    transcript_entry_lines_with_state(line, lang, width, theme, state)
}

#[derive(Default, Clone, Copy)]
pub(crate) struct TranscriptRenderState {
    in_code_block: bool,
}

pub(crate) fn transcript_entry_lines_with_state(
    line: &str,
    lang: Language,
    width: u16,
    theme: Theme,
    state: &mut TranscriptRenderState,
) -> Vec<Line<'static>> {
    if let Some(message) = line.strip_prefix("◆ ") {
        state.in_code_block = false;
        // Пустая строка перед репликой пользователя — отделяет ход от предыдущего.
        let mut out = vec![Line::from("")];
        out.extend(user_message_lines(message, width, theme));
        return out;
    }

    if is_markdown_fence(line) {
        state.in_code_block = !state.in_code_block;
        return Vec::new();
    }

    if state.in_code_block {
        return code_block_lines(line, width, theme);
    }

    // Воздух перед началом ответа (⏺) и эхо команды (❯), чтобы реплики не слипались.
    let mut out = Vec::new();
    if line.starts_with("⏺ ") || line.starts_with("❯ ") {
        out.push(Line::from(""));
    }
    out.extend(
        wrap_terminal_line(line, width)
            .into_iter()
            .map(|wrapped| style_transcript_line(&wrapped, lang, theme)),
    );
    out
}

/// Реплика пользователя: стрелка-маркер + текст на залитом фоном «пузыре».
/// Без рамки и подписи «Ты» — отправленное сообщение видно по фону.
pub(crate) fn user_message_lines(message: &str, width: u16, theme: Theme) -> Vec<Line<'static>> {
    let arrow_style = Style::default()
        .fg(theme.accent())
        .add_modifier(Modifier::BOLD);
    let bubble_style = Style::default()
        .fg(Color::White)
        .bg(theme.accent_bg())
        .add_modifier(Modifier::BOLD);

    // «➤ » (2 ячейки) + по пробелу-полю слева/справа внутри пузыря = 4 ячейки.
    let content_width = (width as usize).saturating_sub(4).max(8);
    let wrapped = wrap_chars(message, content_width);
    // Пузырь обнимает текст: ширина = самая длинная строка (не на всю ширину экрана).
    let bubble = wrapped
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    wrapped
        .iter()
        .enumerate()
        .map(|(index, line)| {
            // Стрелка только на первой строке, продолжения — отступ под текст.
            let prefix = if index == 0 { "➤ " } else { "  " };
            let pad = " ".repeat(bubble.saturating_sub(line.chars().count()));
            Line::from(vec![
                Span::styled(prefix, arrow_style),
                Span::styled(format!(" {line}{pad} "), bubble_style),
            ])
        })
        .collect()
}

/// Разбивает строку на спаны, подсвечивая inline-код в обратных кавычках.
/// Незакрытые кавычки оставляются как есть (перенос строки мог разорвать пару).
fn inline_code_spans(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close_rel) = after.find('`') else {
            break;
        };
        if open > 0 {
            spans.push(Span::raw(rest[..open].to_string()));
        }
        spans.push(Span::styled(
            after[..close_rel].to_string(),
            Style::default().fg(Color::Indexed(180)),
        ));
        rest = &after[close_rel + 1..];
    }
    if !rest.is_empty() {
        spans.push(Span::raw(rest.to_string()));
    }
    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }
    spans
}

/// Разбивает строку на спаны, подсвечивая inline-код (`код`) и **жирный** текст.
/// Маркеры удаляются — остаётся только стиль. Inline-код имеет приоритет над
/// жирным, поэтому `**` внутри кода трактуется буквально. Незакрытые маркеры
/// остаются обычным текстом (перенос строки мог разорвать пару).
fn inline_md_spans(text: &str) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Inline-код в обратных кавычках.
        if bytes[i] == b'`' {
            if let Some(close) = text[i + 1..].find('`') {
                if !buf.is_empty() {
                    spans.push(Span::raw(std::mem::take(&mut buf)));
                }
                spans.push(Span::styled(
                    text[i + 1..i + 1 + close].to_string(),
                    Style::default().fg(Color::Indexed(180)),
                ));
                i += 1 + close + 1;
                continue;
            }
        }
        // **Жирный** текст (с возможным inline-кодом внутри).
        if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'*') {
            if let Some(close) = text[i + 2..].find("**") {
                let inner = &text[i + 2..i + 2 + close];
                if !inner.is_empty() {
                    if !buf.is_empty() {
                        spans.push(Span::raw(std::mem::take(&mut buf)));
                    }
                    for mut span in inline_code_spans(inner) {
                        span.style = span.style.add_modifier(Modifier::BOLD);
                        spans.push(span);
                    }
                    i += 2 + close + 2;
                    continue;
                }
            }
        }
        // Обычный символ — копим в буфер, шагаем по границе UTF-8.
        let ch = text[i..].chars().next().unwrap();
        buf.push(ch);
        i += ch.len_utf8();
    }
    if !buf.is_empty() {
        spans.push(Span::raw(buf));
    }
    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }
    spans
}

pub(crate) fn style_transcript_line(line: &str, lang: Language, theme: Theme) -> Line<'static> {
    if line.starts_with("◆ ") {
        Line::from(vec![
            Span::styled(
                lang.choose("Ты", "You"),
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(line.trim_start_matches("◆ ").to_string()),
        ])
    } else if let Some(command) = line.strip_prefix("❯ ") {
        Line::from(vec![
            Span::styled(
                "❯ ",
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(command.to_string()),
        ])
    } else if line.starts_with("Final brief: ") {
        Line::from(vec![
            Span::styled(
                "⏺ brief ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.trim_start_matches("Final brief: ").to_string()),
        ])
    } else if is_error_status_line(line) {
        Line::styled(line.to_string(), Style::default().fg(Color::Red))
    } else if line.starts_with("Drafting")
        || line.starts_with("Review")
        || line.starts_with("Revision")
    {
        Line::from(vec![
            Span::styled(
                "⏺ ",
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.to_string()),
        ])
    } else if line.starts_with("⎿ ") || line.trim_start().starts_with('⎿') {
        Line::styled(line.to_string(), Style::default().fg(Color::DarkGray))
    } else if let Some(rest) = line.strip_prefix("⏺ ") {
        let mut spans = vec![Span::styled(
            "⏺ ",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )];
        spans.extend(inline_md_spans(rest));
        Line::from(spans)
    } else if line.starts_with("✻ ") || line.starts_with("✦ ") {
        Line::styled(
            line.to_string(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )
    } else if line.starts_with("🅐 ") {
        // Заголовок шага исполнителя в тандеме — цветом акцента.
        Line::styled(
            line.to_string(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )
    } else if line.starts_with("🅒 ") {
        // Заголовок шага критика в тандеме — отдельным цветом (как режим Tandem).
        Line::styled(
            line.to_string(),
            Style::default()
                .fg(Color::Indexed(170))
                .add_modifier(Modifier::BOLD),
        )
    } else if let Some(heading) = line.strip_prefix("### ") {
        Line::styled(
            format!("  {heading}"),
            Style::default()
                .fg(theme.accent_soft())
                .add_modifier(Modifier::BOLD),
        )
    } else if let Some(heading) = line.strip_prefix("## ") {
        Line::styled(
            heading.to_string(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )
    } else if let Some(heading) = line.strip_prefix("# ") {
        Line::styled(
            heading.to_string(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
    } else if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        let mut spans = vec![Span::styled("• ", Style::default().fg(theme.accent()))];
        spans.extend(inline_md_spans(item));
        Line::from(spans)
    } else if let Some(quote) = line.strip_prefix("> ") {
        Line::styled(
            format!("▏ {quote}"),
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        )
    } else {
        Line::from(inline_md_spans(line))
    }
}

pub(crate) fn is_markdown_fence(line: &str) -> bool {
    let trimmed = line.trim_start();
    let without_status = trimmed
        .strip_prefix("⏺ ")
        .map(str::trim_start)
        .unwrap_or(trimmed);

    without_status.starts_with("```") || without_status.starts_with("~~~")
}

pub(crate) fn code_block_lines(line: &str, width: u16, theme: Theme) -> Vec<Line<'static>> {
    let content_width = width.saturating_sub(3).max(1);
    wrap_terminal_line(line, content_width)
        .into_iter()
        .map(|wrapped| {
            Line::from(vec![
                Span::styled("  ", Style::default().fg(theme.accent_dim())),
                Span::styled(wrapped, Style::default().fg(Color::Gray)),
            ])
        })
        .collect()
}

pub(crate) fn is_error_status_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    let lower = trimmed.to_ascii_lowercase();

    lower.starts_with("error:")
        || lower.starts_with("failed:")
        || lower.starts_with("failed ")
        || lower.starts_with("wait failed:")
        || lower.starts_with("engine missing")
        || lower.contains("returned an error")
        || lower.contains("failed to spawn")
        || lower.contains("завершился с кодом")
        || lower.contains("вернул ошибку")
        || (trimmed.starts_with("⎿ ")
            && (lower.contains("error")
                || lower.contains("failed")
                || lower.contains("read-only file system")))
}

pub(crate) fn separator_line(width: u16, theme: Theme) -> Line<'static> {
    Line::styled(
        "─".repeat(width as usize),
        Style::default().fg(theme.accent_dim()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn user_message_uses_arrow_and_background_not_box() {
        let lines = user_message_lines("привет мир", 80, Theme::Purple);
        assert_eq!(lines.len(), 1, "короткое сообщение — одна строка");
        let text: String = plain(&lines[0]);
        // Стрелка-маркер есть, рамки и подписи «Ты» — нет.
        assert!(text.starts_with("➤ "), "ведущая стрелка: {text:?}");
        assert!(!text.contains("Ты") && !text.contains("You"));
        for ch in ['╭', '╮', '╰', '╯', '│', '─'] {
            assert!(!text.contains(ch), "нет символов рамки: {ch}");
        }
        // Текст лежит на залитом фоном «пузыре» (bg = accent_bg темы).
        let bubble = lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("привет мир"))
            .expect("есть спан с текстом");
        assert_eq!(
            bubble.style.bg,
            Some(Theme::Purple.accent_bg()),
            "фон-пузырь"
        );

        // Многострочное сообщение: стрелка только на первой строке.
        let many = user_message_lines(&"слово ".repeat(60), 40, Theme::Purple);
        assert!(many.len() > 1);
        assert!(plain(&many[0]).starts_with("➤ "));
        assert!(
            plain(&many[1]).starts_with("  "),
            "продолжение — отступ, без стрелки"
        );
    }

    #[test]
    fn hides_markdown_code_fence_markers() {
        let transcript = vec![
            "⏺ Вот пример:".to_string(),
            "```text".to_string(),
            "Покажи текущее состояние проекта".to_string(),
            "```".to_string(),
            "Готово.".to_string(),
        ];
        let mut state = TranscriptRenderState::default();
        let rendered = transcript
            .iter()
            .flat_map(|line| {
                transcript_entry_lines_with_state(line, Language::Ru, 80, Theme::Purple, &mut state)
            })
            .map(|line| plain(&line))
            .collect::<Vec<_>>();

        assert!(!rendered.iter().any(|line| line.contains("```")));
        assert!(rendered
            .iter()
            .any(|line| line.contains("Покажи текущее состояние проекта")));
        assert!(rendered.iter().any(|line| line.contains("Готово.")));
    }

    #[test]
    fn code_block_state_persists_across_lines() {
        // История append-only: один `state` ведёт fence между вызовами
        // `history_line_render`. Внутри fence строки — как код, после — обычные.
        let lines = ["```rust", "let x = 1;", "```", "обычный текст"];
        let mut state = TranscriptRenderState::default();
        let rendered = lines
            .iter()
            .flat_map(|line| history_line_render(line, Language::Ru, 80, Theme::Purple, &mut state))
            .collect::<Vec<_>>();

        // Маркеры fence сами по себе не дают строк.
        assert!(!rendered.iter().any(|l| plain(l).contains("```")));

        // Строка внутри fence отрисована как код: серое содержимое и отступ.
        let code = rendered
            .iter()
            .find(|l| plain(l).contains("let x = 1;"))
            .expect("строка кода отрисована");
        assert!(plain(code).starts_with("  "), "код имеет отступ");
        assert!(
            code.spans.iter().any(|s| s.style.fg == Some(Color::Gray)),
            "содержимое кода — серым"
        );

        // Строка после закрывающего fence — обычная, без серой подсветки кода.
        let normal = rendered
            .iter()
            .find(|l| plain(l).contains("обычный текст"))
            .expect("обычная строка отрисована");
        assert!(
            normal.spans.iter().all(|s| s.style.fg != Some(Color::Gray)),
            "после fence подсветка кода снята"
        );

        // state вернулся в обычный режим.
        assert!(!state.in_code_block, "fence закрыт — state сброшен");
    }

    #[test]
    fn does_not_treat_plain_error_words_as_status_errors() {
        assert!(!is_error_status_line(
            "- слово error внутри обычного ответа не должно красить строку"
        ));
        assert!(is_error_status_line("Failed to spawn codex"));
        assert!(is_error_status_line("⎿ Read-only file system"));
    }

    #[test]
    fn inline_code_splits_backticks() {
        let spans = inline_code_spans("use `cargo build` now");
        let joined: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(joined, "use cargo build now");
        assert!(spans.len() >= 2);
        // незакрытая кавычка не ломает рендер
        let one = inline_code_spans("broken ` tail");
        let joined: String = one.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(joined, "broken ` tail");
    }

    #[test]
    fn inline_bold_strips_markers_and_styles() {
        // **жирный** → спан с модификатором BOLD без звёздочек.
        let spans = inline_md_spans("есть **важное** слово");
        let joined: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(joined, "есть важное слово");
        assert!(
            spans
                .iter()
                .any(|s| s.content == "важное" && s.style.add_modifier.contains(Modifier::BOLD)),
            "жирный фрагмент несёт модификатор BOLD"
        );
        // Маркеры `**` не должны просочиться ни в один спан.
        assert!(spans.iter().all(|s| !s.content.contains('*')));

        // Регрессия из реального бага: нумерованный пункт «1. **Память:** …»
        // идёт в общую ветку и должен потерять звёздочки, но не цифры.
        let line = style_transcript_line("1. **Память:** важно", Language::Ru, Theme::Purple);
        let joined: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(joined, "1. Память: важно");
        assert!(line
            .spans
            .iter()
            .any(|s| s.content == "Память:" && s.style.add_modifier.contains(Modifier::BOLD)));

        // Незакрытый `**` остаётся буквальным и не съедает последующий inline-код.
        let spans = inline_md_spans("a ** b `c`");
        let joined: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(joined, "a ** b c");
        assert!(spans
            .iter()
            .any(|s| s.content == "c" && s.style.fg == Some(Color::Indexed(180))));
    }

    #[test]
    fn separator_line_follows_active_theme() {
        // Разделитель должен брать цвет из активной темы, а не из захардкоженной палитры.
        for theme in [
            Theme::Purple,
            Theme::Cyan,
            Theme::Rose,
            Theme::Amber,
            Theme::Mono,
        ] {
            assert_eq!(separator_line(12, theme).style.fg, Some(theme.accent_dim()));
        }
        // Регрессия на «вечно фиолетовый»: смена темы должна менять цвет разделителя.
        assert_ne!(
            separator_line(12, Theme::Cyan).style.fg,
            separator_line(12, Theme::Purple).style.fg,
        );
    }
}
