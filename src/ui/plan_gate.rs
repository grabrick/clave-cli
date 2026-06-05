use super::*;

pub(crate) fn plan_gate_panel_height() -> u16 {
    3 // верх/низ рамки + одна строка подсказки
}

pub(crate) fn draw_plan_gate_panel(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let color = ChatMode::Plan.color();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(Span::styled(
            app.lang.choose(" План готов ", " Plan ready "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::default().fg(color));

    let hint = app.lang.choose(
        "Enter — выполнить · текст + Enter — доработать · Esc — отмена",
        "Enter — execute · text + Enter — refine · Esc — cancel",
    );

    let paragraph = Paragraph::new(Line::from(hint))
        .block(block)
        .style(Style::default().fg(color))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
