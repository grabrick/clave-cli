use super::*;

pub(crate) fn draw_transcript(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let visible = area.height.saturating_sub(1) as usize;
    let mut lines = vec![Line::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(app.theme.accent_dim()),
    )];
    lines.extend(transcript_lines(
        &app.transcript,
        app.lang,
        area.width,
        app.theme,
    ));

    if app.running {
        lines.push(Line::from(""));
        lines.extend(loader_lines(app, area.width));
    }

    let start = lines.len().saturating_sub(visible);
    let lines = lines[start..].to_vec();

    let transcript = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(transcript, area);
}

#[derive(Default)]
pub(crate) struct TranscriptRenderState {
    in_code_block: bool,
}

pub(crate) fn transcript_lines(
    transcript: &[String],
    lang: Language,
    width: u16,
    theme: Theme,
) -> Vec<Line<'static>> {
    let mut state = TranscriptRenderState::default();
    let mut lines = Vec::new();

    for line in transcript {
        lines.extend(transcript_entry_lines_with_state(
            line, lang, width, theme, &mut state,
        ));
    }

    lines
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
        return user_message_box(message, lang, width, theme);
    }

    if is_markdown_fence(line) {
        state.in_code_block = !state.in_code_block;
        return Vec::new();
    }

    if state.in_code_block {
        return code_block_lines(line, width, theme);
    }

    wrap_terminal_line(line, width)
        .into_iter()
        .map(|wrapped| style_transcript_line(&wrapped, lang, theme))
        .collect()
}

pub(crate) fn user_message_box(
    message: &str,
    lang: Language,
    width: u16,
    theme: Theme,
) -> Vec<Line<'static>> {
    let width = width as usize;
    if width < 12 {
        return vec![Line::styled(
            format!("{} {}", lang.choose("Ты", "You"), message),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )];
    }

    let label = format!(" {} ", lang.choose("Ты", "You"));
    let content_width = width.saturating_sub(4).max(8);
    let horizontal_width = content_width + 2;
    let mut lines = Vec::new();
    let top_tail = "─".repeat(horizontal_width.saturating_sub(label.chars().count()));
    lines.push(Line::styled(
        format!("╭{label}{top_tail}╮"),
        Style::default().fg(theme.accent()),
    ));

    for wrapped in wrap_chars(message, content_width) {
        let padding = content_width.saturating_sub(wrapped.chars().count());
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(theme.accent())),
            Span::styled(
                wrapped,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ".repeat(padding)),
            Span::styled(" │", Style::default().fg(theme.accent())),
        ]));
    }

    lines.push(Line::styled(
        format!("╰{}╯", "─".repeat(horizontal_width)),
        Style::default().fg(theme.accent()),
    ));
    lines
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
        Line::from(vec![
            Span::styled(
                "⏺ ",
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(rest.to_string()),
        ])
    } else if line.starts_with("✻ ") || line.starts_with("✦ ") {
        Line::styled(
            line.to_string(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Line::from(line.to_string())
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

pub(crate) fn centered_line(text: impl Into<String>, width: u16, style: Style) -> Line<'static> {
    let text = text.into();
    let left_pad = (width as usize).saturating_sub(text.chars().count()) / 2;
    Line::from(vec![
        Span::raw(" ".repeat(left_pad)),
        Span::styled(text, style),
    ])
}

pub(crate) fn separator_line(width: u16) -> Line<'static> {
    Line::styled("─".repeat(width as usize), Style::default().fg(ACCENT_DIM))
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
    fn hides_markdown_code_fence_markers() {
        let transcript = vec![
            "⏺ Вот пример:".to_string(),
            "```text".to_string(),
            "Покажи текущее состояние проекта".to_string(),
            "```".to_string(),
            "Готово.".to_string(),
        ];
        let rendered = transcript_lines(&transcript, Language::Ru, 80, Theme::Purple)
            .iter()
            .map(plain)
            .collect::<Vec<_>>();

        assert!(!rendered.iter().any(|line| line.contains("```")));
        assert!(rendered
            .iter()
            .any(|line| line.contains("Покажи текущее состояние проекта")));
        assert!(rendered.iter().any(|line| line.contains("Готово.")));
    }

    #[test]
    fn does_not_treat_plain_error_words_as_status_errors() {
        assert!(!is_error_status_line(
            "- слово error внутри обычного ответа не должно красить строку"
        ));
        assert!(is_error_status_line("Failed to spawn codex"));
        assert!(is_error_status_line("⎿ Read-only file system"));
    }
}
