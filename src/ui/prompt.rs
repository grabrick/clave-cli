use super::*;

pub(crate) fn draw_prompt_bar(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = input_lines_wrapped(&app.input, area.width);
    let command_mode = app.input.starts_with('/');
    let tick = current_effort_tick();
    let mut rendered = Vec::new();

    rendered.push(prompt_rule_line(area.width, command_mode, tick));
    for (index, line) in lines.iter().enumerate() {
        let prefix = if index == 0 { "› " } else { "  " };
        rendered.push(Line::from(vec![
            Span::styled(
                prefix,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.clone()),
        ]));
    }
    rendered.push(prompt_rule_line(area.width, command_mode, tick + 3));

    frame.render_widget(Paragraph::new(rendered), area);

    let (line_index, col) = input_cursor_position_wrapped(&app.input, app.cursor, area.width);
    let cursor_y = area.y + 1 + (line_index as u16).min(area.height.saturating_sub(2));
    let cursor_x = area.x + 2 + col as u16;
    let max_x = area.x + area.width.saturating_sub(1);
    frame.set_cursor_position(Position::new(cursor_x.min(max_x), cursor_y));
}

pub(crate) fn prompt_rule_line(width: u16, active: bool, tick: u64) -> Line<'static> {
    if !active {
        return Line::styled("─".repeat(width as usize), Style::default().fg(ACCENT_DIM));
    }

    let mut spans = Vec::new();
    for index in 0..width as usize {
        spans.push(Span::styled(
            "─",
            Style::default().fg(shimmer_color("xhigh", index, tick)),
        ));
    }
    Line::from(spans)
}
