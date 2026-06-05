use super::*;

pub(crate) fn shortcuts_panel_height() -> u16 {
    5
}

pub(crate) fn draw_shortcuts_panel(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(Span::styled(
            app.lang.choose(" Управление ", " Controls "),
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::default().fg(app.theme.accent_dim()));

    let rows = vec![
        Line::from(app.lang.choose(
            "Tab автодоп · ↑↓ история · PageUp/PageDown скролл · Esc сброс",
            "Tab complete · ↑↓ history · PageUp/PageDown scroll · Esc clear",
        )),
        Line::from(app.lang.choose(
            "Ctrl+A/E начало/конец · Ctrl+W/U/K удалить · Alt+←→ по словам",
            "Ctrl+A/E start/end · Ctrl+W/U/K delete · Alt+←→ by word",
        )),
        Line::from(app.lang.choose(
            "Enter отправить · Ctrl+J новая строка · Ctrl+C ×2 выход · ? скрыть",
            "Enter send · Ctrl+J newline · Ctrl+C ×2 exit · ? hide",
        )),
    ];

    let paragraph = Paragraph::new(rows)
        .block(block)
        .style(Style::default().fg(MUTED))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
