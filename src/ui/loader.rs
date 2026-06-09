use super::*;

pub(crate) fn loader_line(app: &App) -> Line<'static> {
    let elapsed = app
        .run_started_at
        .map(|started| started.elapsed())
        .unwrap_or_else(|| Duration::from_secs(0));
    let phrase = LOADER_PHRASES
        .get(((elapsed.as_secs() / 6) as usize) % LOADER_PHRASES.len())
        .copied()
        .unwrap_or("Thinking");
    let label = if app.run_label.is_empty() {
        app.mode.as_str().to_string()
    } else {
        app.run_label.clone()
    };
    // Живая оценка расхода: реально отправленный промт (run_token_estimate) плюс
    // уже принятый текст ответа (live_answer растёт по токенам у claude). Цифра
    // приблизительная (≈, токенизация по символам), но опирается на реальный
    // текст и растёт по факту. Точный usage·$ — в футере по завершении.
    let out_tokens = if app.live_answer.is_empty() {
        0
    } else {
        estimate_tokens(&app.live_answer)
    };
    let tokens = app.run_token_estimate.unwrap_or(0) + out_tokens;
    let detail = if tokens > 0 {
        format!(
            "({} · {} · ≈ {} {})",
            format_elapsed(elapsed),
            label,
            format_token_count(tokens),
            app.lang.choose("токенов", "tokens"),
        )
    } else {
        format!("({} · {})", format_elapsed(elapsed), label)
    };

    let mut spans =
        theme_shimmer_text_spans(&format!("✳ {}… ", phrase), app.theme, current_effort_tick());
    spans.push(Span::styled(
        detail,
        Style::default().fg(Color::Indexed(245)),
    ));
    Line::from(spans)
}

pub(crate) fn loader_lines(app: &App, width: u16) -> Vec<Line<'static>> {
    let mut lines = vec![loader_line(app)];
    lines.extend(loader_activity_lines(app, width));
    lines
}

pub(crate) fn loader_activity_lines(app: &App, width: u16) -> Vec<Line<'static>> {
    let content_width = width.saturating_sub(5).max(1) as usize;
    // По ОДНОЙ строке на активность и не более трёх последних: высота loader
    // должна быть предсказуемой. Иначе при каждом апдейте активности менялась бы
    // высота viewport, а его смена в inline-режиме = пересоздание терминала
    // (скролл-дрожь во время прогона).
    const MAX_ACTIVITY_LINES: usize = 3;
    let skip = app.run_activity.len().saturating_sub(MAX_ACTIVITY_LINES);
    app.run_activity
        .iter()
        .skip(skip)
        .map(|activity| {
            Line::from(vec![
                Span::styled("  ⎿ ", Style::default().fg(app.theme.accent_dim())),
                Span::styled(
                    truncate_chars(activity, content_width),
                    Style::default().fg(Color::Indexed(245)),
                ),
            ])
        })
        .collect()
}

pub(crate) fn theme_shimmer_text_spans(text: &str, theme: Theme, tick: u64) -> Vec<Span<'static>> {
    text.chars()
        .enumerate()
        .map(|(index, ch)| {
            Span::styled(
                ch.to_string(),
                Style::default()
                    .fg(theme_shimmer_color(theme, index, tick))
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect()
}

pub(crate) fn theme_shimmer_color(theme: Theme, index: usize, tick: u64) -> Color {
    let palette = [
        theme.accent_dim(),
        theme.accent(),
        theme.accent_soft(),
        theme.accent(),
        theme.accent_dim(),
    ];
    let phase = (tick as usize) % palette.len();
    let color_index = (index + palette.len() - phase) % palette.len();
    palette[color_index]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn loader_shows_token_estimate_when_known() {
        let mut app = App::new();
        app.run_started_at = Some(Instant::now());
        app.run_label = "Claude".to_string();
        app.run_token_estimate = Some(1200);
        // Поток ответа пуст — показываем оценку промта.
        let text = line_text(&loader_line(&app));
        assert!(text.contains('≈'), "есть пометка оценки: {text}");
        assert!(text.contains("1.2k"), "форматированный счётчик: {text}");
        assert!(text.contains("токенов"), "подпись по-русски: {text}");

        // Без оценки — старый вид без счётчика.
        app.run_token_estimate = None;
        app.live_answer.clear();
        let text = line_text(&loader_line(&app));
        assert!(!text.contains('≈'), "нет токенов — нет пометки: {text}");
    }

    #[test]
    fn loader_shimmer_uses_current_theme_palette() {
        assert_eq!(
            theme_shimmer_color(Theme::Amber, 1, 0),
            Theme::Amber.accent()
        );
        assert_ne!(
            theme_shimmer_color(Theme::Amber, 1, 0),
            Theme::Purple.accent()
        );
    }
}

pub(crate) fn format_elapsed(duration: Duration) -> String {
    let total = duration.as_secs();
    if total < 60 {
        return format!("{}s", total.max(1));
    }

    let minutes = total / 60;
    let seconds = total % 60;
    if minutes < 60 {
        return format!("{}m {:02}s", minutes, seconds);
    }

    let hours = minutes / 60;
    let minutes = minutes % 60;
    format!("{}h {:02}m", hours, minutes)
}
