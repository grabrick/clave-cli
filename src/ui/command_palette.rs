use super::*;

pub(crate) fn command_palette_fade_level(app: &App) -> usize {
    app.command_palette_opened_at
        .map(|opened_at| (opened_at.elapsed().as_millis() / 45).min(7) as usize)
        .unwrap_or(7)
}

pub(crate) fn command_palette_accent(level: usize) -> Color {
    match level {
        0 => Color::Indexed(236),
        1 => Color::Indexed(238),
        2 => Color::Indexed(240),
        3 => Color::Indexed(97),
        4 => Color::Indexed(141),
        _ => ACCENT_SOFT,
    }
}

pub(crate) fn command_palette_muted(level: usize) -> Color {
    match level {
        0 => Color::Indexed(236),
        1 => Color::Indexed(238),
        2 => Color::Indexed(240),
        3 => Color::Indexed(243),
        4 => Color::Indexed(246),
        _ => MUTED,
    }
}

pub(crate) fn command_palette_selected_bg(level: usize) -> Option<Color> {
    match level {
        0..=2 => None,
        3 => Some(Color::Indexed(236)),
        4 => Some(Color::Indexed(238)),
        _ => Some(ACCENT_BG),
    }
}

pub(crate) fn draw_command_screen(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    if area.height == 0 {
        return;
    }

    let suggestions = app.suggestions();
    let commands = if suggestions.is_empty() {
        COMMANDS.to_vec()
    } else {
        suggestions
    };
    let selected = app
        .selected_suggestion
        .min(commands.len().saturating_sub(1));
    let visible = area.height as usize;
    let fade_level = command_palette_fade_level(app);

    let lines = commands
        .iter()
        .take(visible)
        .enumerate()
        .map(|(index, command)| {
            let is_selected = index == selected;
            let row_fade = fade_level.saturating_sub(index / 3);
            let command_style = if is_selected {
                let mut style = Style::default()
                    .fg(if row_fade >= 5 {
                        Color::White
                    } else {
                        command_palette_accent(row_fade)
                    })
                    .add_modifier(Modifier::BOLD);
                if let Some(bg) = command_palette_selected_bg(row_fade) {
                    style = style.bg(bg);
                }
                style
            } else {
                Style::default().fg(command_palette_accent(row_fade))
            };
            let desc_style = if is_selected {
                Style::default().fg(if row_fade >= 5 {
                    Color::White
                } else {
                    command_palette_muted(row_fade)
                })
            } else {
                Style::default().fg(command_palette_muted(row_fade))
            };

            Line::from(vec![
                Span::styled(
                    if is_selected { "› " } else { "  " },
                    Style::default().fg(command_palette_muted(row_fade)),
                ),
                Span::styled(format!("{:<30}", command.usage), command_style),
                Span::raw("  "),
                Span::styled(command.description(app.lang), desc_style),
            ])
        })
        .collect::<Vec<_>>();

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}
