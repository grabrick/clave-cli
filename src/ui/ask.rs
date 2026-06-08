use super::*;

/// Высота панели селектора: рамка + строка вопроса + видимые варианты + подсказка.
pub(crate) fn ask_panel_height(state: &AskState, cap: u16) -> u16 {
    let list = (state.rows() as u16).min(8); // варианты + «Свой вариант», максимум видимых
    (2 + 1 + list + 1).min(cap).max(4)
}

pub(crate) fn draw_ask_panel(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Some(state) = &app.ask else {
        return;
    };
    if area.height == 0 {
        return;
    }

    let color = app.theme.accent();
    let title = if state.prompt.multi {
        app.lang
            .choose(" Выбор (несколько) ", " Choose (multiple) ")
    } else {
        app.lang.choose(" Выбор ", " Choose ")
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(Span::styled(
            title,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::default().fg(color));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    let inner_width = inner.width as usize;

    let mut lines: Vec<Line<'static>> = Vec::new();
    // Вопрос модели (жирный, обрезан по ширине).
    lines.push(Line::styled(
        truncate_chars(&state.prompt.question, inner_width),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ));

    // Список вариантов + «Свой вариант» со скроллом, удерживающим курсор в зоне.
    let list_capacity = (inner.height as usize).saturating_sub(2).max(1); // минус вопрос и подсказка
    let total = state.rows();
    let offset = command_palette_scroll_offset(state.cursor, list_capacity, total);
    for idx in offset..(offset + list_capacity).min(total) {
        let selected = idx == state.cursor;
        let marker = if selected { "› " } else { "  " };
        if idx < state.prompt.options.len() {
            let opt = &state.prompt.options[idx];
            let mut spans = vec![Span::styled(marker, Style::default().fg(color))];
            if state.prompt.multi {
                let checked = state.checked[idx];
                spans.push(Span::styled(
                    if checked { "[x] " } else { "[ ] " },
                    Style::default().fg(if checked { color } else { MUTED }),
                ));
            }
            spans.push(Span::styled(
                opt.label.clone(),
                if selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(app.theme.accent_soft())
                },
            ));
            if let Some(note) = &opt.note {
                let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                let room = inner_width.saturating_sub(used + 3);
                if room > 4 {
                    spans.push(Span::styled(
                        format!(" — {}", truncate_chars(note, room)),
                        Style::default().fg(MUTED),
                    ));
                }
            }
            lines.push(Line::from(spans));
        } else {
            // Строка «Свой ответ» — инлайн-поле ввода: печать идёт сюда, Enter отправляет.
            let label = app.lang.choose("Свой ответ: ", "Custom: ");
            let mut spans = vec![
                Span::styled(marker, Style::default().fg(color)),
                Span::styled(
                    label,
                    if selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(MUTED)
                    },
                ),
            ];
            let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
            let room = inner_width.saturating_sub(used + 1);
            if selected {
                // Показываем хвост (где печатаем) + курсор поля.
                if !state.custom.is_empty() && room > 0 {
                    let chars: Vec<char> = state.custom.chars().collect();
                    let shown: String = if chars.len() > room {
                        chars[chars.len() - room..].iter().collect()
                    } else {
                        state.custom.clone()
                    };
                    spans.push(Span::styled(shown, Style::default().fg(Color::White)));
                }
                spans.push(Span::styled("▌", Style::default().fg(color)));
            } else if state.custom.is_empty() {
                spans.push(Span::styled(
                    app.lang.choose("впишите свой вариант", "type your own"),
                    Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                ));
            } else {
                spans.push(Span::styled(
                    truncate_chars(&state.custom, room),
                    Style::default().fg(MUTED),
                ));
            }
            lines.push(Line::from(spans));
        }
    }

    // Подсказка по клавишам — зависит от того, где курсор.
    let hint = if state.on_custom_row() {
        app.lang.choose(
            "впишите ответ · Enter отправить · ↑↓ к списку · Esc отмена",
            "type · Enter send · ↑↓ list · Esc cancel",
        )
    } else if state.prompt.multi {
        app.lang.choose(
            "↑↓ · Space отметить · Enter подтвердить · Esc отмена",
            "↑↓ · Space toggle · Enter confirm · Esc cancel",
        )
    } else {
        app.lang.choose(
            "↑↓ выбрать · Enter подтвердить · Esc отмена",
            "↑↓ move · Enter confirm · Esc cancel",
        )
    };
    lines.push(Line::styled(
        truncate_chars(hint, inner_width),
        Style::default().fg(MUTED),
    ));

    frame.render_widget(Paragraph::new(lines), inner);
}
