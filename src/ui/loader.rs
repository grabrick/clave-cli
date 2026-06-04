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
    let token_detail = app
        .run_token_estimate
        .map(|tokens| {
            let live_tokens = live_token_estimate(tokens, elapsed, app.active_effort_for_tokens());
            format!(" · ≈ {} tokens", format_token_count(live_tokens))
        })
        .unwrap_or_default();
    let detail = format!(
        "({} · {} · effort {}{})",
        format_elapsed(elapsed),
        label,
        app.effort_summary(),
        token_detail
    );

    let mut spans = shimmer_text_spans(
        &format!("✳ {}… ", phrase),
        "xhigh",
        true,
        current_effort_tick(),
    );
    spans.push(Span::styled(
        detail,
        Style::default().fg(Color::Indexed(245)),
    ));
    Line::from(spans)
}

pub(crate) fn live_token_estimate(base: usize, elapsed: Duration, effort: &str) -> usize {
    let per_second = effort_weight(effort);
    base.saturating_add(elapsed.as_secs() as usize * per_second)
}

pub(crate) fn effort_weight(effort: &str) -> usize {
    match effort {
        "low" => 8,
        "medium" => 16,
        "high" => 28,
        "xhigh" => 44,
        "max" => 52,
        _ => 20,
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
