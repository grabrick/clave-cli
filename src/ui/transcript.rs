use super::*;

pub(crate) fn draw_transcript(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let visible = area.height.saturating_sub(1) as usize;
    let mut lines = vec![Line::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(app.theme.accent_dim()),
    )];
    for line in &app.transcript {
        lines.extend(transcript_entry_lines(
            line, app.lang, area.width, app.theme,
        ));
    }

    if app.running {
        lines.push(Line::from(""));
        lines.push(loader_line(app));
    }

    let start = lines.len().saturating_sub(visible);
    let lines = lines[start..].to_vec();

    let transcript = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(transcript, area);
}

pub(crate) fn transcript_entry_lines(
    line: &str,
    lang: Language,
    width: u16,
    theme: Theme,
) -> Vec<Line<'static>> {
    if let Some(message) = line.strip_prefix("◆ ") {
        return user_message_box(message, lang, width, theme);
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
    } else if line.contains("error") || line.contains("failed") || line.contains("Failed") {
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
