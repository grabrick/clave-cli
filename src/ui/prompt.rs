use super::*;

pub(crate) fn draw_prompt_bar(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = input_lines_wrapped(&app.input, area.width);
    let command_mode = normalized_command_query(&app.input).is_some();
    let tick = current_effort_tick();
    let mut rendered = Vec::new();

    // Плашка названия чата — отдельной строкой справа НАД верхней полоской.
    rendered.push(chat_title_badge_line(area.width, app));
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
    // +2: над первой строкой ввода — строка плашки и верхняя полоска.
    let cursor_y = area.y + 2 + (line_index as u16).min(area.height.saturating_sub(3));
    let cursor_x = area.x + 2 + col as u16;
    let max_x = area.x + area.width.saturating_sub(1);
    frame.set_cursor_position(Position::new(cursor_x.min(max_x), cursor_y));
}

/// Отдельная строка НАД верхней полоской: плашка названия чата, прижата к правому
/// краю (заливка акцентом, как «пузырь» реплики). Пустое название или слишком узкий
/// терминал → пустая строка. Чистая по входам (width + theme + title) — тестируемо.
fn chat_title_badge_line(width: u16, app: &App) -> Line<'static> {
    badge_line(width, app.theme, &app.chat_title)
}

fn badge_line(width: u16, theme: Theme, title: &str) -> Line<'static> {
    let total = width as usize;
    let title = title.trim();
    // Отступ от правого края, чтобы плашка не липла к границе.
    const RIGHT_PAD: usize = 1;

    if title.is_empty() || total < RIGHT_PAD + 4 {
        return Line::from("");
    }

    // Бюджет текста: минус правый отступ и 2 внутренних пробела плашки.
    let title = truncate_chars(title, total - (RIGHT_PAD + 2));
    let badge = format!(" {title} ");
    let left_pad = total.saturating_sub(badge.chars().count() + RIGHT_PAD);

    Line::from(vec![
        Span::raw(" ".repeat(left_pad)),
        Span::styled(
            badge,
            Style::default()
                .fg(Color::Black)
                .bg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

/// Один символ горизонтальной полоски для столбца `index`. В командном режиме
/// столбцы переливаются акцентными оттенками в зависимости от `tick`.
fn rule_span(index: usize, active: bool, tick: u64, theme: Theme) -> Span<'static> {
    if !active {
        return Span::styled("─", Style::default().fg(theme.accent_dim()));
    }
    let color = match ((index as u64 + tick) % 6) as usize {
        0 => theme.accent_dim(),
        1 | 5 => theme.accent(),
        2..=4 => theme.accent_soft(),
        _ => theme.accent(),
    };
    Span::styled("─", Style::default().fg(color))
}

pub(crate) fn prompt_rule_line(width: u16, active: bool, tick: u64, theme: Theme) -> Line<'static> {
    if !active {
        return Line::styled(
            "─".repeat(width as usize),
            Style::default().fg(theme.accent_dim()),
        );
    }

    let spans = (0..width as usize)
        .map(|index| rule_span(index, active, tick, theme))
        .collect::<Vec<_>>();
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_line_right_aligns_title_and_pads_left() {
        let line = badge_line(20, Theme::Purple, "chat");
        // Плашка несёт текст с внутренними пробелами.
        let badge = line
            .spans
            .iter()
            .find(|s| s.content.contains("chat"))
            .expect("плашка есть");
        assert_eq!(badge.content.as_ref(), " chat ");
        // Прижата вправо: занятая ширина = total − правый отступ (1).
        let used: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
        assert_eq!(used, 19, "плашка у правого края, 1 колонка отступа");
        // Слева — пробелы-подложка.
        assert!(line.spans[0].content.chars().all(|c| c == ' '));
    }

    #[test]
    fn badge_line_is_empty_without_title_or_when_too_narrow() {
        for (w, title) in [(20u16, ""), (3, "chat")] {
            let line = badge_line(w, Theme::Purple, title);
            assert!(
                line.spans.iter().all(|s| s.content.trim().is_empty()),
                "пустая строка при w={w}, title={title:?}"
            );
        }
    }
}
