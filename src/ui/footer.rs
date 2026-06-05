use super::*;

pub(crate) fn draw_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if area.height == 0 {
        return;
    }

    if let Some((message, shown_at)) = &app.footer_notice {
        if shown_at.elapsed() <= Duration::from_secs(2) {
            let text = truncate_chars(message, area.width as usize);
            frame.render_widget(
                Paragraph::new(text).style(
                    Style::default()
                        .fg(app.theme.accent_soft())
                        .add_modifier(Modifier::BOLD),
                ),
                area,
            );
            return;
        }
    }

    let left = app.lang.choose(
        "? подсказки · / команды · ↑↓ история",
        "? for shortcuts · / for commands · ↑↓ history",
    );
    let (right, right_style) = footer_right_segment(app);
    let width = area.width as usize;
    let right_slot_width = footer_right_slot_width(app).min(width);
    let right = truncate_chars(&right, right_slot_width);
    let right_width = right.chars().count();
    let left_width = left.chars().count();
    let min_gap = 2;
    let left = if left_width + right_slot_width + min_gap > width {
        truncate_chars(left, width.saturating_sub(right_slot_width + min_gap))
    } else {
        left.to_string()
    };
    let gap = width.saturating_sub(left.chars().count() + right_slot_width);
    let right_padding = right_slot_width.saturating_sub(right_width);
    let line = Line::from(vec![
        Span::styled(left, Style::default().fg(app.theme.accent_soft())),
        Span::raw(" ".repeat(gap)),
        Span::raw(" ".repeat(right_padding)),
        Span::styled(right, right_style),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

pub(crate) fn footer_right_segments(app: &App) -> Vec<String> {
    let ready = app.lang.choose("готов", "ready");
    let mut segments = Vec::new();

    if app.status != ready {
        segments.push(format!("status {}", app.status));
    }

    segments.push(format!("mode {}", app.mode.as_str()));
    segments.push(format!("chat {}", app.direct_provider.as_str()));
    segments.push(format!(
        "roles {}>{}",
        app.mode.architect_provider().as_str(),
        app.mode.reviewer_provider().as_str()
    ));
    segments.push(format!("theme {}", app.theme.as_str()));
    segments.push(format!("effort {}", app.compact_effort_summary()));
    if app.usage.total_tokens() > 0 {
        segments.push(format!(
            "usage {} · ${:.3}",
            format_token_count(app.usage.total_tokens() as usize),
            app.usage.total_cost_usd()
        ));
    }
    segments
}

pub(crate) fn footer_right_target(app: &App) -> String {
    let segments = footer_right_segments(app);

    let phase = rotating_phase(8, segments.len());
    segments.get(phase).cloned().unwrap_or_default()
}

pub(crate) fn footer_right_slot_width(app: &App) -> usize {
    let current_width = app.footer_right_text.chars().count();
    let previous_width = app
        .footer_right_previous_text
        .as_ref()
        .map(|previous| previous.chars().count())
        .unwrap_or(0);

    current_width.max(previous_width)
}

pub(crate) fn footer_right_segment(app: &App) -> (String, Style) {
    let base_style = Style::default().fg(app.theme.accent_soft());
    let Some(changed_at) = app.footer_right_changed_at else {
        return (app.footer_right_text.clone(), base_style);
    };

    let elapsed_ms = changed_at.elapsed().as_millis();
    let previous = app
        .footer_right_previous_text
        .as_ref()
        .unwrap_or(&app.footer_right_text);

    if elapsed_ms < 360 {
        (
            previous.clone(),
            Style::default().fg(footer_transition_color(app.theme, elapsed_ms, false)),
        )
    } else {
        (
            app.footer_right_text.clone(),
            Style::default().fg(footer_transition_color(app.theme, elapsed_ms - 360, true)),
        )
    }
}

pub(crate) fn footer_transition_color(theme: Theme, elapsed_ms: u128, entering: bool) -> Color {
    let step = (elapsed_ms / 90).min(4) as usize;
    let palette = if entering {
        [
            theme.accent_dim(),
            Color::DarkGray,
            Color::Gray,
            theme.accent_soft(),
            theme.accent_soft(),
        ]
    } else {
        [
            theme.accent_soft(),
            Color::Gray,
            Color::DarkGray,
            theme.accent_dim(),
            theme.accent_dim(),
        ]
    };
    palette[step]
}
