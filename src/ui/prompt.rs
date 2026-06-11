use super::*;

pub(crate) fn draw_prompt_bar(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = input_lines_wrapped(&app.input, area.width);
    let command_mode = normalized_command_query(&app.input).is_some();
    let tick = current_effort_tick();
    let mut rendered = Vec::new();

    rendered.push(chat_title_label_line(area.width, app));
    rendered.push(prompt_rule_line(area.width, command_mode, tick, app.theme));
    for (index, line) in lines.iter().enumerate() {
        let prefix = if index == 0 { "› " } else { "  " };
        rendered.push(Line::from(vec![
            Span::styled(
                prefix,
                Style::default()
                    .fg(app.theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.clone()),
        ]));
    }
    rendered.push(prompt_rule_line(
        area.width,
        command_mode,
        tick + 3,
        app.theme,
    ));

    frame.render_widget(Paragraph::new(rendered), area);

    let (line_index, col) = input_cursor_position_wrapped(&app.input, app.cursor, area.width);
    let cursor_y = area.y + 2 + (line_index as u16).min(area.height.saturating_sub(3));
    let cursor_x = area.x + 2 + col as u16;
    let max_x = area.x + area.width.saturating_sub(1);
    frame.set_cursor_position(Position::new(cursor_x.min(max_x), cursor_y));
}

/// Плашка с названием чата: отдельная строка над верхней полоской композера,
/// прижатая к правому краю (с отступом в 1 символ от края).
fn chat_title_label_line(width: u16, app: &App) -> Line<'static> {
    let width = width as usize;
    // Резерв: 2 внутренних пробела плашки + 1 символ отступа справа.
    let title_room = width.saturating_sub(3);
    let title = truncate_chars(&app.chat_title, title_room);
    let badge = format!(" {title} ");
    let left_pad = width.saturating_sub(badge.chars().count() + 1);
    Line::from(vec![
        Span::raw(" ".repeat(left_pad)),
        Span::styled(
            badge,
            Style::default()
                .fg(Color::Black)
                .bg(app.theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ])
}

pub(crate) fn prompt_rule_line(width: u16, active: bool, tick: u64, theme: Theme) -> Line<'static> {
    if !active {
        return Line::styled(
            "─".repeat(width as usize),
            Style::default().fg(theme.accent_dim()),
        );
    }

    let mut spans = Vec::new();
    for index in 0..width as usize {
        let color = match ((index as u64 + tick) % 6) as usize {
            0 => theme.accent_dim(),
            1 | 5 => theme.accent(),
            2..=4 => theme.accent_soft(),
            _ => theme.accent(),
        };
        spans.push(Span::styled("─", Style::default().fg(color)));
    }
    Line::from(spans)
}
