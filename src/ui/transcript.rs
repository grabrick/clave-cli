use super::*;

/// Дешёвая сигнатура содержимого транскрипта для инвалидации кэша рендера.
/// Хэш << полный рендер (нет аллокаций Line/Span, wrap, style), но точно
/// отражает контент — поэтому кэш не может «устареть» незаметно.
pub(crate) fn transcript_signature(
    transcript: &[String],
    width: u16,
    theme: Theme,
    lang: Language,
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    width.hash(&mut hasher);
    std::mem::discriminant(&theme).hash(&mut hasher);
    std::mem::discriminant(&lang).hash(&mut hasher);
    transcript.len().hash(&mut hasher);
    for line in transcript {
        line.hash(&mut hasher);
    }
    hasher.finish()
}

/// Хвост из лоадера (анимируется каждый кадр, поэтому НЕ кэшируется).
pub(crate) fn loader_tail_lines(app: &App, width: u16) -> Vec<Line<'static>> {
    if !app.running {
        return Vec::new();
    }
    let mut lines = vec![Line::from("")];
    lines.extend(loader_lines(app, width));
    lines
}

/// Окно отрисовки транскрипта: (start, end) в виртуальном теле длиной `total`.
/// scroll_offset=0 показывает низ (свежие строки), рост offset листает вверх до
/// начала. Без «минус одна строка» — влезающий чат виден целиком.
pub(crate) fn transcript_scroll_window(
    total: usize,
    area_height: u16,
    scroll_offset: usize,
) -> (usize, usize) {
    let visible = (area_height as usize).max(1);
    let max_offset = total.saturating_sub(visible);
    let offset = scroll_offset.min(max_offset);
    let start = max_offset - offset;
    let end = (start + visible).min(total);
    (start, end)
}

/// Рендерит видимый срез из виртуального тела `[separator] + cached + loader_tail`,
/// клонируя только строки в окне экрана (кэш экономит пересборку всех строк).
pub(crate) fn draw_transcript(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    width: u16,
    cached: &[Line<'static>],
    loader_tail: &[Line<'static>],
) {
    frame.render_widget(Clear, area);
    let separator = Line::styled(
        "─".repeat(width as usize),
        Style::default().fg(app.theme.accent_dim()),
    );
    let total = 1 + cached.len() + loader_tail.len();
    let (start, end) = transcript_scroll_window(total, area.height, app.scroll_offset);

    let mut slice = Vec::with_capacity(end.saturating_sub(start));
    for index in start..end {
        if index == 0 {
            slice.push(separator.clone());
        } else if index <= cached.len() {
            slice.push(cached[index - 1].clone());
        } else {
            slice.push(loader_tail[index - 1 - cached.len()].clone());
        }
    }

    let transcript = Paragraph::new(slice).wrap(Wrap { trim: false });
    frame.render_widget(transcript, area);
}

#[derive(Default)]
pub(crate) struct TranscriptRenderState {
    in_code_block: bool,
}

pub(crate) fn transcript_lines(
    transcript: &[String],
    lang: Language,
    width: u16,
    theme: Theme,
) -> Vec<Line<'static>> {
    let mut state = TranscriptRenderState::default();
    let mut lines = Vec::new();

    for line in transcript {
        lines.extend(transcript_entry_lines_with_state(
            line, lang, width, theme, &mut state,
        ));
    }

    lines
}

pub(crate) fn transcript_entry_lines_with_state(
    line: &str,
    lang: Language,
    width: u16,
    theme: Theme,
    state: &mut TranscriptRenderState,
) -> Vec<Line<'static>> {
    if let Some(message) = line.strip_prefix("◆ ") {
        state.in_code_block = false;
        return user_message_box(message, lang, width, theme);
    }

    if is_markdown_fence(line) {
        state.in_code_block = !state.in_code_block;
        return Vec::new();
    }

    if state.in_code_block {
        return code_block_lines(line, width, theme);
    }

    wrap_terminal_line(line, width)
        .into_iter()
        .map(|wrapped| style_transcript_line(&wrapped, lang, theme))
        .collect()
}

pub(crate) fn user_message_box(
    message: &str,
    lang: Language,
    width: u16,
    theme: Theme,
) -> Vec<Line<'static>> {
    let width = width as usize;
    if width < 12 {
        return vec![Line::styled(
            format!("{} {}", lang.choose("Ты", "You"), message),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )];
    }

    let label = format!(" {} ", lang.choose("Ты", "You"));
    let content_width = width.saturating_sub(4).max(8);
    let horizontal_width = content_width + 2;
    let mut lines = Vec::new();
    let top_tail = "─".repeat(horizontal_width.saturating_sub(label.chars().count()));
    lines.push(Line::styled(
        format!("╭{label}{top_tail}╮"),
        Style::default().fg(theme.accent()),
    ));

    for wrapped in wrap_chars(message, content_width) {
        let padding = content_width.saturating_sub(wrapped.chars().count());
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(theme.accent())),
            Span::styled(
                wrapped,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ".repeat(padding)),
            Span::styled(" │", Style::default().fg(theme.accent())),
        ]));
    }

    lines.push(Line::styled(
        format!("╰{}╯", "─".repeat(horizontal_width)),
        Style::default().fg(theme.accent()),
    ));
    lines
}

/// Разбивает строку на спаны, подсвечивая inline-код в обратных кавычках.
/// Незакрытые кавычки оставляются как есть (перенос строки мог разорвать пару).
fn inline_code_spans(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close_rel) = after.find('`') else {
            break;
        };
        if open > 0 {
            spans.push(Span::raw(rest[..open].to_string()));
        }
        spans.push(Span::styled(
            after[..close_rel].to_string(),
            Style::default().fg(Color::Indexed(180)),
        ));
        rest = &after[close_rel + 1..];
    }
    if !rest.is_empty() {
        spans.push(Span::raw(rest.to_string()));
    }
    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }
    spans
}

pub(crate) fn style_transcript_line(line: &str, lang: Language, theme: Theme) -> Line<'static> {
    if line.starts_with("◆ ") {
        Line::from(vec![
            Span::styled(
                lang.choose("Ты", "You"),
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(line.trim_start_matches("◆ ").to_string()),
        ])
    } else if let Some(command) = line.strip_prefix("❯ ") {
        Line::from(vec![
            Span::styled(
                "❯ ",
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(command.to_string()),
        ])
    } else if line.starts_with("Final brief: ") {
        Line::from(vec![
            Span::styled(
                "⏺ brief ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.trim_start_matches("Final brief: ").to_string()),
        ])
    } else if is_error_status_line(line) {
        Line::styled(line.to_string(), Style::default().fg(Color::Red))
    } else if line.starts_with("Drafting")
        || line.starts_with("Review")
        || line.starts_with("Revision")
    {
        Line::from(vec![
            Span::styled(
                "⏺ ",
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.to_string()),
        ])
    } else if line.starts_with("⎿ ") || line.trim_start().starts_with('⎿') {
        Line::styled(line.to_string(), Style::default().fg(Color::DarkGray))
    } else if let Some(rest) = line.strip_prefix("⏺ ") {
        Line::from(vec![
            Span::styled(
                "⏺ ",
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(rest.to_string()),
        ])
    } else if line.starts_with("✻ ") || line.starts_with("✦ ") {
        Line::styled(
            line.to_string(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )
    } else if line.starts_with("🅐 ") {
        // Заголовок шага исполнителя в тандеме — цветом акцента.
        Line::styled(
            line.to_string(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )
    } else if line.starts_with("🅒 ") {
        // Заголовок шага критика в тандеме — отдельным цветом (как режим Tandem).
        Line::styled(
            line.to_string(),
            Style::default()
                .fg(Color::Indexed(170))
                .add_modifier(Modifier::BOLD),
        )
    } else if let Some(heading) = line.strip_prefix("### ") {
        Line::styled(
            format!("  {heading}"),
            Style::default()
                .fg(theme.accent_soft())
                .add_modifier(Modifier::BOLD),
        )
    } else if let Some(heading) = line.strip_prefix("## ") {
        Line::styled(
            heading.to_string(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )
    } else if let Some(heading) = line.strip_prefix("# ") {
        Line::styled(
            heading.to_string(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
    } else if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        let mut spans = vec![Span::styled("• ", Style::default().fg(theme.accent()))];
        spans.extend(inline_code_spans(item));
        Line::from(spans)
    } else if let Some(quote) = line.strip_prefix("> ") {
        Line::styled(
            format!("▏ {quote}"),
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        )
    } else {
        Line::from(inline_code_spans(line))
    }
}

pub(crate) fn is_markdown_fence(line: &str) -> bool {
    let trimmed = line.trim_start();
    let without_status = trimmed
        .strip_prefix("⏺ ")
        .map(str::trim_start)
        .unwrap_or(trimmed);

    without_status.starts_with("```") || without_status.starts_with("~~~")
}

pub(crate) fn code_block_lines(line: &str, width: u16, theme: Theme) -> Vec<Line<'static>> {
    let content_width = width.saturating_sub(3).max(1);
    wrap_terminal_line(line, content_width)
        .into_iter()
        .map(|wrapped| {
            Line::from(vec![
                Span::styled("  ", Style::default().fg(theme.accent_dim())),
                Span::styled(wrapped, Style::default().fg(Color::Gray)),
            ])
        })
        .collect()
}

pub(crate) fn is_error_status_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    let lower = trimmed.to_ascii_lowercase();

    lower.starts_with("error:")
        || lower.starts_with("failed:")
        || lower.starts_with("failed ")
        || lower.starts_with("wait failed:")
        || lower.starts_with("engine missing")
        || lower.contains("returned an error")
        || lower.contains("failed to spawn")
        || lower.contains("завершился с кодом")
        || lower.contains("вернул ошибку")
        || (trimmed.starts_with("⎿ ")
            && (lower.contains("error")
                || lower.contains("failed")
                || lower.contains("read-only file system")))
}

pub(crate) fn centered_line(text: impl Into<String>, width: u16, style: Style) -> Line<'static> {
    let text = text.into();
    let left_pad = (width as usize).saturating_sub(text.chars().count()) / 2;
    Line::from(vec![
        Span::raw(" ".repeat(left_pad)),
        Span::styled(text, style),
    ])
}

pub(crate) fn separator_line(width: u16, theme: Theme) -> Line<'static> {
    Line::styled(
        "─".repeat(width as usize),
        Style::default().fg(theme.accent_dim()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn hides_markdown_code_fence_markers() {
        let transcript = vec![
            "⏺ Вот пример:".to_string(),
            "```text".to_string(),
            "Покажи текущее состояние проекта".to_string(),
            "```".to_string(),
            "Готово.".to_string(),
        ];
        let rendered = transcript_lines(&transcript, Language::Ru, 80, Theme::Purple)
            .iter()
            .map(plain)
            .collect::<Vec<_>>();

        assert!(!rendered.iter().any(|line| line.contains("```")));
        assert!(rendered
            .iter()
            .any(|line| line.contains("Покажи текущее состояние проекта")));
        assert!(rendered.iter().any(|line| line.contains("Готово.")));
    }

    #[test]
    fn does_not_treat_plain_error_words_as_status_errors() {
        assert!(!is_error_status_line(
            "- слово error внутри обычного ответа не должно красить строку"
        ));
        assert!(is_error_status_line("Failed to spawn codex"));
        assert!(is_error_status_line("⎿ Read-only file system"));
    }

    #[test]
    fn scroll_window_reaches_top_and_bottom() {
        // Влезает целиком — показываем всё, без скрытой строки.
        assert_eq!(transcript_scroll_window(10, 30, 0), (0, 10));
        // Длинный чат, offset=0 — низ (свежие строки).
        assert_eq!(transcript_scroll_window(100, 20, 0), (80, 100));
        // Большой скролл вверх — упираемся в начало, не в пустоту.
        assert_eq!(transcript_scroll_window(100, 20, 1000), (0, 20));
    }

    #[test]
    fn inline_code_splits_backticks() {
        let spans = inline_code_spans("use `cargo build` now");
        let joined: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(joined, "use cargo build now");
        assert!(spans.len() >= 2);
        // незакрытая кавычка не ломает рендер
        let one = inline_code_spans("broken ` tail");
        let joined: String = one.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(joined, "broken ` tail");
    }

    #[test]
    fn separator_line_follows_active_theme() {
        // Разделитель должен брать цвет из активной темы, а не из захардкоженной палитры.
        for theme in [
            Theme::Purple,
            Theme::Cyan,
            Theme::Rose,
            Theme::Amber,
            Theme::Mono,
        ] {
            assert_eq!(separator_line(12, theme).style.fg, Some(theme.accent_dim()));
        }
        // Регрессия на «вечно фиолетовый»: смена темы должна менять цвет разделителя.
        assert_ne!(
            separator_line(12, Theme::Cyan).style.fg,
            separator_line(12, Theme::Purple).style.fg,
        );
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use std::time::Instant;

    const WIDTH: u16 = 100;

    /// Реалистичный транскрипт: user-боксы, ответы, блоки кода, активность, пустые.
    fn big_transcript(n: usize) -> Vec<String> {
        let mut t = Vec::with_capacity(n);
        for i in 0..n {
            match i % 8 {
                0 => t.push(format!("◆ Запрос пользователя номер {i} с понятным текстом")),
                1 => t.push(format!(
                    "⏺ Ответ модели, строка {i}, достаточно длинная, чтобы сработал перенос по ширине терминала и была реалистичная нагрузка на рендер"
                )),
                2 => t.push("```rust".to_string()),
                3 => t.push(format!("    let value = compute({i}); // комментарий внутри блока кода")),
                4 => t.push("```".to_string()),
                5 => t.push(format!("⎿ Читаю src/module_{i}.rs")),
                6 => t.push(format!(
                    "обычная строка ответа {i} со словами и словами и ещё словами для переноса по ширине"
                )),
                _ => t.push(String::new()),
            }
        }
        t
    }

    /// Воспроизводит ДОфиксовую O(n²)-версию переноса (для сравнения с текущей O(n)).
    fn wrap_quadratic(text: &str, max_chars: usize) -> Vec<String> {
        let max_chars = max_chars.max(1);
        if text.is_empty() {
            return vec![String::new()];
        }
        let mut rows = Vec::new();
        let mut current = String::new();
        for ch in text.chars() {
            if ch == '\n' {
                rows.push(current);
                current = String::new();
                continue;
            }
            if current.chars().count() >= max_chars {
                rows.push(current);
                current = String::new();
            }
            current.push(ch);
        }
        rows.push(current);
        rows
    }

    #[test]
    #[ignore = "perf bench: cargo test --release bench:: -- --ignored --nocapture"]
    fn bench_transcript_render() {
        let transcript = big_transcript(500);
        let iters = 300u32;
        let start = Instant::now();
        let mut sink = 0usize;
        for _ in 0..iters {
            sink = sink.wrapping_add(
                transcript_lines(&transcript, Language::Ru, WIDTH, Theme::Purple).len(),
            );
        }
        let elapsed = start.elapsed();
        println!(
            "[render] {} строк × {} итер = {:?} → {:?}/кадр (sink={sink})",
            transcript.len(),
            iters,
            elapsed,
            elapsed / iters,
        );
    }

    #[test]
    #[ignore = "perf bench: cargo test --release bench:: -- --ignored --nocapture"]
    fn bench_cache_signature_vs_render() {
        let transcript = big_transcript(500);
        let iters = 2000u32;

        let start = Instant::now();
        let mut acc = 0u64;
        for _ in 0..iters {
            acc = acc.wrapping_add(transcript_signature(
                &transcript,
                WIDTH,
                Theme::Purple,
                Language::Ru,
            ));
        }
        let sig = start.elapsed();

        let start = Instant::now();
        let mut acc2 = 0usize;
        for _ in 0..iters {
            acc2 = acc2.wrapping_add(
                transcript_lines(&transcript, Language::Ru, WIDTH, Theme::Purple).len(),
            );
        }
        let render = start.elapsed();

        let ratio = render.as_nanos() as f64 / sig.as_nanos().max(1) as f64;
        println!(
            "[cache] signature {:?}/вызов vs render {:?}/вызов → когда транскрипт стабилен, кадр дешевле ~{ratio:.0}x (acc={acc},{acc2})",
            sig / iters,
            render / iters,
        );
    }

    #[test]
    #[ignore = "perf bench: cargo test --release bench:: -- --ignored --nocapture"]
    fn bench_wrap_linear_vs_quadratic() {
        let long = "слово ".repeat(2000);
        let chars = long.chars().count();
        let iters = 500u32;

        let start = Instant::now();
        let mut a = 0usize;
        for _ in 0..iters {
            a = a.wrapping_add(wrap_terminal_text_preserving_spaces(&long, 80).len());
        }
        let linear = start.elapsed();

        let start = Instant::now();
        let mut b = 0usize;
        for _ in 0..iters {
            b = b.wrapping_add(wrap_quadratic(&long, 80).len());
        }
        let quad = start.elapsed();

        let ratio = quad.as_nanos() as f64 / linear.as_nanos().max(1) as f64;
        println!(
            "[wrap] {chars} симв: O(n) {:?}/вызов vs O(n²) {:?}/вызов → ~{ratio:.0}x (a={a},b={b})",
            linear / iters,
            quad / iters,
        );
    }
}
