use super::*;

pub(crate) fn search_panel_height() -> u16 {
    4
}

pub(crate) fn draw_search_panel(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let matches = app.search_matches();
    let count = matches.len();
    let position = if count == 0 {
        0
    } else {
        app.search_index.min(count - 1) + 1
    };

    let header = format!(
        "{}: {}  ({}/{})",
        app.lang.choose("Поиск", "Search"),
        app.search_query,
        position,
        count
    );

    let preview_width = (area.width as usize).saturating_sub(4).max(1);
    let preview = matches
        .get(app.search_index.min(count.saturating_sub(1)))
        .and_then(|&index| app.transcript.get(index))
        .map(|line| truncate_chars(line.trim(), preview_width))
        .unwrap_or_else(|| app.lang.choose("нет совпадений", "no matches").to_string());

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(Span::styled(
            app.lang
                .choose(" Поиск (Enter/↑↓, Esc) ", " Search (Enter/↑↓, Esc) "),
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::default().fg(app.theme.accent_dim()));

    let lines = vec![
        Line::from(Span::styled(
            header,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(preview, Style::default().fg(MUTED))),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
