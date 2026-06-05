use super::*;

pub(crate) fn draw_chats_screen(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);

    let mut lines = vec![
        Line::styled(
            "› /resume",
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        separator_line(area.width, app.theme),
        Line::from(""),
        Line::from(Span::styled(
            app.lang.choose("Сохранённые чаты", "Saved chats"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let ln = app.lang.choose("стр", "ln");
    for (index, chat) in app.chats_picker.iter().enumerate() {
        let selected = index == app.chats_index;
        let marker = if chat.id == app.chat_id { "●" } else { " " };
        let style = if selected {
            Style::default()
                .fg(Color::White)
                .bg(app.theme.accent_bg())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.accent_soft())
        };
        lines.push(Line::from(vec![
            Span::styled(
                if selected { "› " } else { "  " },
                Style::default().fg(app.theme.accent()),
            ),
            Span::styled(
                format!(
                    "{marker} {} · {} {ln} · {}",
                    chat.id,
                    chat.lines,
                    truncate_chars(&chat.title, 40)
                ),
                style,
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        app.lang.choose(
            "↑↓ выбрать · Enter открыть · Esc отмена",
            "↑↓ select · Enter open · Esc cancel",
        ),
        Style::default().fg(MUTED),
    ));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}
