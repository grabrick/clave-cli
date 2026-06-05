use super::*;

pub(crate) fn draw_welcome(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let card_height = area.height.min(12);
    let card = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: card_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled(
                format!(" {APP_NAME} "),
                Style::default()
                    .fg(app.theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("v0.1.0 ", Style::default().fg(MUTED)),
        ]))
        .border_style(Style::default().fg(app.theme.accent()));

    frame.render_widget(block, card);

    if card.width < 30 || card.height < 5 {
        return;
    }

    let inner = Rect {
        x: card.x + 2,
        y: card.y + 1,
        width: card.width.saturating_sub(4),
        height: card.height.saturating_sub(2),
    };

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(41), Constraint::Percentage(59)])
        .split(inner);

    let user = env::var("USER").unwrap_or_else(|_| "friend".to_string());
    let left_width = columns[0].width;
    let left = vec![
        centered_line(
            app.lang.choose("С возвращением!", "Welcome back!"),
            left_width,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        centered_line(user, left_width, Style::default().fg(MUTED)),
        Line::from(""),
        centered_line("╭──╮", left_width, Style::default().fg(app.theme.accent())),
        centered_line(
            "›──◆──‹",
            left_width,
            Style::default()
                .fg(app.theme.accent_soft())
                .add_modifier(Modifier::BOLD),
        ),
        centered_line("╰──╯", left_width, Style::default().fg(app.theme.accent())),
        Line::from(""),
        centered_line(
            format!(
                "{} · chat {}",
                app.mode.as_str(),
                app.direct_provider.as_str()
            ),
            left_width,
            Style::default().fg(Color::DarkGray),
        ),
    ];

    frame.render_widget(Paragraph::new(left).wrap(Wrap { trim: false }), columns[0]);

    let right_width = columns[1].width;
    let right = vec![
        Line::from(Span::styled(
            app.lang.choose("Быстрый старт", "Tips for getting started"),
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(app.lang.choose(
            "Введи сообщение и нажми Enter",
            "Type a message and press Enter",
        )),
        Line::from(app.lang.choose(
            "Для спеки используй /plan <задача>",
            "Use /plan <task> for Clave planning",
        )),
        separator_line(right_width, app.theme),
        Line::from(Span::styled(
            app.lang.choose("Что нового", "What's new"),
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(app.lang.choose(
            "Обычный ввод отвечает напрямую моделью",
            "Plain input chats directly with the model",
        )),
        separator_line(right_width, app.theme),
        Line::from(Span::styled(
            "Overview",
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "{} {} · chat {} · effort {}",
            app.lang.choose("Режим", "Mode"),
            app.mode.as_str(),
            app.direct_provider.as_str(),
            app.effort_summary()
        )),
        Line::from(app.lang.choose(
            "/settings · /chats · /new · /effort",
            "/settings · /chats · /new · /effort",
        )),
    ];

    frame.render_widget(Paragraph::new(right).wrap(Wrap { trim: false }), columns[1]);
}
