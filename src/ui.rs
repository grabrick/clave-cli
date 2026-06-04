use crate::prelude::*;
use crate::*;

pub(crate) fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    if app.onboarding.is_some() {
        draw_onboarding(frame, area, app);
        return;
    }

    if app.effort_picker {
        draw_effort_screen(frame, area, app);
        return;
    }

    let command_mode = app.input.starts_with('/');
    let composer_height = composer_height(app, area.width).min(area.height.saturating_sub(2));
    let palette_height = if command_mode {
        command_palette_height(app, area.height, composer_height)
    } else {
        0
    };
    let footer_height = if command_mode { 0 } else { 1 };
    let output_gap = if app.transcript.is_empty() && !app.running && app.last_run.is_none() {
        0
    } else {
        1
    };
    let palette_gap = if command_mode { 1 } else { 0 };
    let main_height = main_area_height(
        app,
        area,
        composer_height,
        palette_height,
        footer_height,
        output_gap,
        palette_gap,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(main_height),
            Constraint::Length(output_gap),
            Constraint::Length(composer_height),
            Constraint::Length(palette_gap),
            Constraint::Length(palette_height),
            Constraint::Length(footer_height),
            Constraint::Min(0),
        ])
        .split(area);

    draw_main_area(frame, chunks[0], app);
    draw_prompt_bar(frame, chunks[2], app);
    if command_mode {
        draw_command_screen(frame, chunks[4], app);
    } else {
        draw_footer(frame, chunks[5], app);
    }
}

pub(crate) fn draw_main_area(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if app.transcript.is_empty() && !app.running && app.last_run.is_none() {
        draw_welcome(frame, area, app);
    } else {
        draw_transcript(frame, area, app);
    }
}

pub(crate) fn draw_transcript(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let visible = area.height.saturating_sub(1) as usize;
    let mut lines = vec![Line::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(ACCENT_DIM),
    )];
    for line in &app.transcript {
        lines.extend(transcript_entry_lines(line, app.lang, area.width));
    }

    if app.running {
        lines.push(Line::from(""));
        lines.push(loader_line(app));
    }

    let start = lines.len().saturating_sub(visible);
    let lines = lines[start..].to_vec();

    let transcript = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(transcript, area);
}

pub(crate) fn transcript_entry_lines(line: &str, lang: Language, width: u16) -> Vec<Line<'static>> {
    if let Some(message) = line.strip_prefix("◆ ") {
        return user_message_box(message, lang, width);
    }

    wrap_terminal_line(line, width)
        .into_iter()
        .map(|wrapped| style_transcript_line(&wrapped, lang))
        .collect()
}

pub(crate) fn user_message_box(message: &str, lang: Language, width: u16) -> Vec<Line<'static>> {
    let width = width as usize;
    if width < 12 {
        return vec![Line::styled(
            format!("{} {}", lang.choose("Ты", "You"), message),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )];
    }

    let label = format!(" {} ", lang.choose("Ты", "You"));
    let content_width = width.saturating_sub(4).max(8);
    let horizontal_width = content_width + 2;
    let mut lines = Vec::new();
    let top_tail = "─".repeat(horizontal_width.saturating_sub(label.chars().count()));
    lines.push(Line::styled(
        format!("╭{label}{top_tail}╮"),
        Style::default().fg(ACCENT),
    ));

    for wrapped in wrap_chars(message, content_width) {
        let padding = content_width.saturating_sub(wrapped.chars().count());
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(ACCENT)),
            Span::styled(
                wrapped,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ".repeat(padding)),
            Span::styled(" │", Style::default().fg(ACCENT)),
        ]));
    }

    lines.push(Line::styled(
        format!("╰{}╯", "─".repeat(horizontal_width)),
        Style::default().fg(ACCENT),
    ));
    lines
}

pub(crate) fn style_transcript_line(line: &str, lang: Language) -> Line<'static> {
    if line.starts_with("◆ ") {
        Line::from(vec![
            Span::styled(
                lang.choose("Ты", "You"),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(line.trim_start_matches("◆ ").to_string()),
        ])
    } else if let Some(command) = line.strip_prefix("❯ ") {
        Line::from(vec![
            Span::styled(
                "❯ ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
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
    } else if line.contains("error") || line.contains("failed") || line.contains("Failed") {
        Line::styled(line.to_string(), Style::default().fg(Color::Red))
    } else if line.starts_with("Drafting")
        || line.starts_with("Review")
        || line.starts_with("Revision")
    {
        Line::from(vec![
            Span::styled(
                "⏺ ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.to_string()),
        ])
    } else if line.starts_with("⎿ ") || line.trim_start().starts_with('⎿') {
        Line::styled(line.to_string(), Style::default().fg(Color::DarkGray))
    } else if let Some(rest) = line.strip_prefix("⏺ ") {
        Line::from(vec![
            Span::styled(
                "⏺ ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(rest.to_string()),
        ])
    } else if line.starts_with("✻ ") || line.starts_with("✦ ") {
        Line::styled(
            line.to_string(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )
    } else {
        Line::from(line.to_string())
    }
}

pub(crate) fn centered_line(text: impl Into<String>, width: u16, style: Style) -> Line<'static> {
    let text = text.into();
    let left_pad = (width as usize).saturating_sub(text.chars().count()) / 2;
    Line::from(vec![
        Span::raw(" ".repeat(left_pad)),
        Span::styled(text, style),
    ])
}

pub(crate) fn separator_line(width: u16) -> Line<'static> {
    Line::styled("─".repeat(width as usize), Style::default().fg(ACCENT_DIM))
}

pub(crate) fn draw_welcome(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let card_height = area.height.min(12);
    let card = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: card_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled(
                " Duel Code ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("v0.1.0 ", Style::default().fg(MUTED)),
        ]))
        .border_style(Style::default().fg(ACCENT));

    frame.render_widget(block, card);

    if card.width < 30 || card.height < 5 {
        return;
    }

    let inner = Rect {
        x: card.x + 2,
        y: card.y + 1,
        width: card.width.saturating_sub(4),
        height: card.height.saturating_sub(2),
    };

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(41), Constraint::Percentage(59)])
        .split(inner);

    let user = env::var("USER").unwrap_or_else(|_| "friend".to_string());
    let left_width = columns[0].width;
    let left = vec![
        centered_line(
            app.lang.choose("С возвращением!", "Welcome back!"),
            left_width,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        centered_line(user, left_width, Style::default().fg(MUTED)),
        Line::from(""),
        centered_line("╭──╮", left_width, Style::default().fg(ACCENT)),
        centered_line(
            "›──◆──‹",
            left_width,
            Style::default()
                .fg(ACCENT_SOFT)
                .add_modifier(Modifier::BOLD),
        ),
        centered_line("╰──╯", left_width, Style::default().fg(ACCENT)),
        Line::from(""),
        centered_line(
            format!("{} · {}", app.mode.as_str(), app.compact_effort_summary()),
            left_width,
            Style::default().fg(Color::DarkGray),
        ),
    ];

    frame.render_widget(Paragraph::new(left).wrap(Wrap { trim: false }), columns[0]);

    let right_width = columns[1].width;
    let right = vec![
        Line::from(Span::styled(
            app.lang.choose("Быстрый старт", "Tips for getting started"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(app.lang.choose(
            "Введи сообщение и нажми Enter",
            "Type a message and press Enter",
        )),
        Line::from(app.lang.choose(
            "Для спеки используй /plan <задача>",
            "Use /plan <task> for spec-duel planning",
        )),
        separator_line(right_width),
        Line::from(Span::styled(
            app.lang.choose("Что нового", "What's new"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(app.lang.choose(
            "Обычный ввод отвечает напрямую моделью",
            "Plain input chats directly with the model",
        )),
        separator_line(right_width),
        Line::from(Span::styled(
            "Overview",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "{} {} · effort {}",
            app.lang.choose("Режим", "Mode"),
            app.mode.as_str(),
            app.effort_summary()
        )),
        Line::from(app.lang.choose(
            "/chats · /new · /resume · /effort",
            "/chats · /new · /resume · /effort",
        )),
    ];

    frame.render_widget(Paragraph::new(right).wrap(Wrap { trim: false }), columns[1]);
}

pub(crate) fn draw_onboarding(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let Some(onboarding) = app.onboarding.as_ref() else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(1)])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled(
                " Duel Setup ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("first run ", Style::default().fg(MUTED)),
        ]))
        .border_style(Style::default().fg(ACCENT));
    frame.render_widget(block, chunks[0]);

    let inner = Rect {
        x: chunks[0].x + 2,
        y: chunks[0].y + 1,
        width: chunks[0].width.saturating_sub(4),
        height: chunks[0].height.saturating_sub(2),
    };

    let lines = match onboarding.step {
        OnboardingStep::Provider => onboarding_provider_lines(app, onboarding),
        OnboardingStep::Auth => onboarding_auth_lines(app, onboarding),
        OnboardingStep::Settings => onboarding_settings_lines(app, onboarding),
    };

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    draw_footer(frame, chunks[1], app);
}

pub(crate) fn onboarding_provider_lines(app: &App, onboarding: &Onboarding) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::styled(
            app.lang
                .choose("Выбор связки моделей", "Choose model pairing"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(onboarding.message.clone()),
        Line::from(""),
    ];

    for index in 0..provider_count() {
        let selected = index == onboarding.provider_index;
        let mode = provider_mode(index);
        let style = if selected {
            Style::default()
                .fg(Color::White)
                .bg(ACCENT_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ACCENT_SOFT)
        };
        lines.push(Line::from(vec![
            Span::styled(
                if selected { "› " } else { "  " },
                Style::default().fg(ACCENT),
            ),
            Span::styled(format!("{:<14}", mode.as_str()), style),
            Span::raw(" "),
            Span::styled(
                provider_description(mode, app.lang),
                Style::default().fg(MUTED),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        app.lang.choose(
            "↑/↓ выбрать · Enter продолжить · Ctrl+C выйти",
            "↑/↓ choose · Enter continue · Ctrl+C exit",
        ),
        Style::default().fg(MUTED),
    ));
    lines
}

pub(crate) fn onboarding_auth_lines(app: &App, onboarding: &Onboarding) -> Vec<Line<'static>> {
    let codex_needed = app.mode.needs_codex();
    let claude_needed = app.mode.needs_claude();
    vec![
        Line::styled(
            app.lang.choose("Авторизация CLI", "CLI authentication"),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Line::from(onboarding.message.clone()),
        Line::from(""),
        auth_status_line(
            "Codex",
            codex_needed,
            onboarding.codex_installed,
            onboarding.codex_authenticated,
            &onboarding.codex_status,
            "codex login",
            "C",
            app.lang,
        ),
        auth_status_line(
            "Claude",
            claude_needed,
            onboarding.claude_installed,
            onboarding.claude_authenticated,
            &onboarding.claude_status,
            "claude auth login",
            "L",
            app.lang,
        ),
        Line::from(""),
        Line::styled(
            app.lang.choose(
                "C запустить Codex login · L запустить Claude auth login · Enter дальше · Esc назад",
                "C run Codex login · L run Claude auth login · Enter next · Esc back",
            ),
            Style::default().fg(MUTED),
        ),
    ]
}

pub(crate) fn onboarding_settings_lines(app: &App, onboarding: &Onboarding) -> Vec<Line<'static>> {
    let rows = [
        (
            app.lang.choose("Раунды ревью", "Review rounds").to_string(),
            app.rounds.to_string(),
        ),
        ("Effort".to_string(), app.effort_summary()),
        (
            app.lang.choose("Язык", "Language").to_string(),
            app.lang.as_str().to_string(),
        ),
    ];

    let mut lines = vec![
        Line::styled(
            app.lang.choose("Стартовые настройки", "Startup settings"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(onboarding.message.clone()),
        Line::from(""),
    ];

    for (index, (label, value)) in rows.into_iter().enumerate() {
        let selected = index == onboarding.setting_index;
        let style = if selected {
            Style::default()
                .fg(Color::White)
                .bg(ACCENT_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ACCENT_SOFT)
        };
        lines.push(Line::from(vec![
            Span::styled(
                if selected { "› " } else { "  " },
                Style::default().fg(ACCENT),
            ),
            Span::styled(format!("{label:<18}"), style),
            Span::raw(" "),
            Span::styled(value, Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            app.lang.choose("Режим ", "Mode "),
            Style::default().fg(MUTED),
        ),
        Span::styled(
            app.mode.as_str(),
            Style::default()
                .fg(ACCENT_SOFT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.lang.choose(" · Артефакты ", " · Artifacts "),
            Style::default().fg(MUTED),
        ),
        Span::styled(app.out_dir.clone(), Style::default().fg(ACCENT_SOFT)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::styled(
        app.lang.choose(
            "↑/↓ поле · ←/→ изменить · L язык · Enter сохранить · Esc назад",
            "↑/↓ field · ←/→ change · L language · Enter save · Esc back",
        ),
        Style::default().fg(MUTED),
    ));
    lines
}

pub(crate) fn auth_status_line(
    name: &'static str,
    needed: bool,
    installed: bool,
    authenticated: bool,
    status_text: &str,
    command: &'static str,
    key: &'static str,
    lang: Language,
) -> Line<'static> {
    let need_label = if needed {
        lang.choose("нужен", "needed")
    } else {
        lang.choose("опционально", "optional")
    };
    let status = if !installed {
        lang.choose("CLI не найден", "CLI missing").to_string()
    } else if authenticated {
        lang.choose("аккаунт готов", "account ready").to_string()
    } else {
        lang.choose("не авторизован", "not logged in").to_string()
    };
    let status_style = if installed && authenticated {
        Style::default()
            .fg(ACCENT_SOFT)
            .add_modifier(Modifier::BOLD)
    } else if installed {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    };
    let detail = truncate_chars(status_text, 36);

    Line::from(vec![
        Span::styled(
            format!("{name:<8}"),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{need_label:<12}"), Style::default().fg(MUTED)),
        Span::styled(status, status_style),
        Span::raw(" · "),
        Span::styled(format!("{key}: {command}"), Style::default().fg(MUTED)),
        Span::raw(" · "),
        Span::styled(detail, Style::default().fg(Color::DarkGray)),
    ])
}

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
                        .fg(ACCENT_SOFT)
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
        Span::styled(left, Style::default().fg(ACCENT_SOFT)),
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
    segments.push(format!("effort {}", app.compact_effort_summary()));
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
    let base_style = Style::default().fg(ACCENT_SOFT);
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
            Style::default().fg(footer_transition_color(elapsed_ms, false)),
        )
    } else {
        (
            app.footer_right_text.clone(),
            Style::default().fg(footer_transition_color(elapsed_ms - 360, true)),
        )
    }
}

pub(crate) fn footer_transition_color(elapsed_ms: u128, entering: bool) -> Color {
    let step = (elapsed_ms / 90).min(4) as usize;
    let palette: &[u8] = if entering {
        &[240, 243, 246, 183, 183]
    } else {
        &[183, 246, 243, 240, 240]
    };
    Color::Indexed(palette[step])
}

pub(crate) fn draw_effort_screen(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let tick = current_effort_tick();
    let scale_width = effort_scale_width(area.width);
    let scale_start = (area.width as usize).saturating_sub(scale_width) / 2;
    let mut lines = Vec::new();

    lines.push(Line::styled(
        "› /effort",
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    ));
    lines.push(separator_line(area.width));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        app.lang.choose("Усилие", "Effort"),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if matches!(app.mode, Mode::ClaudeCodex) {
        push_linked_effort_mode_line(&mut lines, area.width, app, tick);
    }
    push_effort_axis(&mut lines, area.width, scale_start, scale_width, app.lang);
    lines.push(Line::from(""));

    match app.mode {
        Mode::CodexOnly => push_effort_scale_block(
            &mut lines,
            EffortScaleBlock {
                width: area.width,
                scale_start,
                scale_width,
                title: "Codex",
                provider: "codex",
                allowed: CODEX_EFFORTS,
                selected_index: app.codex_effort_index,
                focused: true,
                tick,
                lang: app.lang,
            },
        ),
        Mode::ClaudeOnly => push_effort_scale_block(
            &mut lines,
            EffortScaleBlock {
                width: area.width,
                scale_start,
                scale_width,
                title: "Claude",
                provider: "claude",
                allowed: CLAUDE_EFFORTS,
                selected_index: app.claude_effort_index,
                focused: true,
                tick,
                lang: app.lang,
            },
        ),
        Mode::ClaudeCodex => {
            if app.linked_effort_split {
                push_effort_scale_block(
                    &mut lines,
                    EffortScaleBlock {
                        width: area.width,
                        scale_start,
                        scale_width,
                        title: "Claude",
                        provider: "claude",
                        allowed: CLAUDE_EFFORTS,
                        selected_index: app.claude_effort_index,
                        focused: app.effort_focus == 1,
                        tick: tick + 2,
                        lang: app.lang,
                    },
                );
                lines.push(Line::from(""));
                push_effort_scale_block(
                    &mut lines,
                    EffortScaleBlock {
                        width: area.width,
                        scale_start,
                        scale_width,
                        title: "Codex",
                        provider: "codex",
                        allowed: CODEX_EFFORTS,
                        selected_index: app.codex_effort_index,
                        focused: app.effort_focus == 2,
                        tick: tick + 4,
                        lang: app.lang,
                    },
                );
            } else {
                push_effort_scale_block(
                    &mut lines,
                    EffortScaleBlock {
                        width: area.width,
                        scale_start,
                        scale_width,
                        title: app.lang.choose("Общий", "Shared"),
                        provider: "shared",
                        allowed: COMMON_EFFORTS,
                        selected_index: app.effort_index,
                        focused: app.effort_focus == 1,
                        tick: tick + 2,
                        lang: app.lang,
                    },
                );
            }
        }
    }

    lines.push(Line::from(""));
    let hint = match app.mode {
        Mode::CodexOnly => app.lang.choose(
            "Codex: model_reasoning_effort, доступно low|medium|high|xhigh",
            "Codex: model_reasoning_effort, available low|medium|high|xhigh",
        ),
        Mode::ClaudeOnly => app.lang.choose(
            "Claude: --effort, доступно low|medium|high|max",
            "Claude: --effort, available low|medium|high|max",
        ),
        Mode::ClaudeCodex if app.linked_effort_split => app.lang.choose(
            "Раздельно: Claude и Codex настраиваются независимо",
            "Per-model: Claude and Codex are adjusted independently",
        ),
        Mode::ClaudeCodex => app.lang.choose(
            "Общий effort применяется к обеим моделям",
            "Shared effort is sent to both models",
        ),
    };
    lines.push(positioned_spans_line(
        area.width,
        vec![(
            scale_start,
            hint.chars().count(),
            vec![Span::styled(hint.to_string(), Style::default().fg(MUTED))],
        )],
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(app.lang.choose(
        "↑/↓ выбрать · ←/→ настроить · Enter подтвердить · Esc отменить",
        "↑/↓ select · ←/→ adjust · Enter confirm · Esc cancel",
    )));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

pub(crate) struct EffortScaleBlock<'a> {
    width: u16,
    scale_start: usize,
    scale_width: usize,
    title: &'a str,
    provider: &'a str,
    allowed: &'static [&'static str],
    selected_index: usize,
    focused: bool,
    tick: u64,
    lang: Language,
}

pub(crate) fn push_effort_axis(
    lines: &mut Vec<Line<'static>>,
    width: u16,
    scale_start: usize,
    scale_width: usize,
    lang: Language,
) {
    let faster = lang.choose("Быстрее", "Faster");
    let smarter = lang.choose("Умнее", "Smarter");
    let smarter_pos = scale_start + scale_width.saturating_sub(smarter.chars().count());
    lines.push(positioned_spans_line(
        width,
        vec![
            (
                scale_start,
                faster.chars().count(),
                vec![Span::styled(
                    faster.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )],
            ),
            (
                smarter_pos,
                smarter.chars().count(),
                vec![Span::styled(
                    smarter.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )],
            ),
        ],
    ));
}

pub(crate) fn push_linked_effort_mode_line(
    lines: &mut Vec<Line<'static>>,
    width: u16,
    app: &App,
    tick: u64,
) {
    let focused = app.effort_focus == 0;
    let split_label = app.lang.choose("раздельно", "per-model");
    let common_label = app.lang.choose("общий", "shared");
    let title = app.lang.choose("Режим", "Mode");
    let prefix = if focused { "› " } else { "  " };
    let mut spans = vec![
        Span::styled(prefix, Style::default().fg(ACCENT)),
        Span::styled(
            format!("{title:<10} "),
            Style::default().fg(Color::White).add_modifier(if focused {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
    ];

    if app.linked_effort_split {
        spans.push(Span::styled(
            common_label.to_string(),
            Style::default().fg(MUTED),
        ));
        spans.push(Span::styled("  |  ", Style::default().fg(MUTED)));
        spans.extend(shimmer_text_spans(split_label, "xhigh", focused, tick));
    } else {
        spans.extend(shimmer_text_spans(common_label, "xhigh", focused, tick));
        spans.push(Span::styled("  |  ", Style::default().fg(MUTED)));
        spans.push(Span::styled(
            split_label.to_string(),
            Style::default().fg(MUTED),
        ));
    }
    lines.push(positioned_spans_line(
        width,
        vec![(0, visible_width_from_spans(&spans), spans)],
    ));
    lines.push(Line::from(""));
}

pub(crate) fn push_effort_scale_block(lines: &mut Vec<Line<'static>>, block: EffortScaleBlock<'_>) {
    let selected_effort = effort_label(block.selected_index);
    let prefix = if block.focused { "› " } else { "  " };
    lines.push(Line::from(vec![
        Span::styled(prefix, Style::default().fg(ACCENT)),
        Span::styled(
            format!("{:<8}", block.title),
            Style::default()
                .fg(Color::White)
                .add_modifier(if block.focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        Span::styled(
            format!(
                " {}",
                effort_provider_hint(block.provider, block.lang, block.allowed)
            ),
            Style::default().fg(MUTED),
        ),
    ]));

    let tick_positions = effort_tick_positions(block.scale_width, block.allowed.len());
    lines.push(effort_scale_line(
        block.width,
        block.scale_start,
        block.scale_width,
        &tick_positions,
    ));

    let mut label_items = Vec::new();
    for (index, effort) in block.allowed.iter().enumerate() {
        let selected = *effort == selected_effort;
        let label = effort_scale_label(effort, selected);
        let label_width = label.chars().count();
        let position = block.scale_start
            + tick_positions[index]
                .saturating_sub(label_width / 2)
                .min(block.scale_width.saturating_sub(label_width));
        label_items.push((
            position,
            label_width,
            effort_label_spans(
                &label,
                effort,
                selected && block.focused,
                block.tick + index as u64,
            ),
        ));
    }
    lines.push(positioned_spans_line(block.width, label_items));

    let description = effort_description(selected_effort, block.lang);
    lines.push(positioned_spans_line(
        block.width,
        vec![(
            block.scale_start,
            description.chars().count(),
            vec![Span::styled(
                description.to_string(),
                Style::default().fg(MUTED),
            )],
        )],
    ));
    lines.push(Line::from(""));
}

pub(crate) fn effort_provider_hint(provider: &str, lang: Language, allowed: &[&str]) -> String {
    match provider {
        "codex" => format!("model_reasoning_effort {}", allowed.join("|")),
        "claude" => format!("--effort {}", allowed.join("|")),
        _ => format!(
            "{} {}",
            lang.choose("доступно", "available"),
            allowed.join("|")
        ),
    }
}

pub(crate) fn visible_width_from_spans(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|span| span.content.chars().count()).sum()
}

pub(crate) fn effort_scale_width(width: u16) -> usize {
    let available = (width as usize).saturating_sub(8);
    available.clamp(32, 74)
}

pub(crate) fn effort_tick_positions(scale_width: usize, count: usize) -> Vec<usize> {
    if count <= 1 {
        return vec![0];
    }

    let last = scale_width.saturating_sub(1);
    (0..count).map(|index| index * last / (count - 1)).collect()
}

pub(crate) fn effort_scale_line(
    width: u16,
    scale_start: usize,
    scale_width: usize,
    tick_positions: &[usize],
) -> Line<'static> {
    let width = width as usize;
    let mut cells = vec![' '; width];
    let end = (scale_start + scale_width).min(width);
    for cell in cells.iter_mut().take(end).skip(scale_start) {
        *cell = '─';
    }
    for tick in tick_positions {
        let position = scale_start + *tick;
        if position < width {
            cells[position] = '┬';
        }
    }

    Line::styled(
        cells.into_iter().collect::<String>(),
        Style::default().fg(ACCENT_DIM),
    )
}

pub(crate) fn positioned_spans_line(
    width: u16,
    mut items: Vec<(usize, usize, Vec<Span<'static>>)>,
) -> Line<'static> {
    let width = width as usize;
    items.sort_by_key(|(position, _, _)| *position);

    let mut spans = Vec::new();
    let mut cursor = 0usize;
    for (position, item_width, item_spans) in items {
        let position = position.min(width);
        if position < cursor {
            continue;
        }
        if position > cursor {
            spans.push(Span::raw(" ".repeat(position - cursor)));
        }
        spans.extend(item_spans);
        cursor = position.saturating_add(item_width);
    }

    Line::from(spans)
}

pub(crate) fn current_effort_tick() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| (duration.as_millis() / 120) as u64)
        .unwrap_or(0)
}

pub(crate) fn rotating_phase(seconds_per_phase: u64, phase_count: usize) -> usize {
    if phase_count == 0 {
        return 0;
    }

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| ((duration.as_secs() / seconds_per_phase) as usize) % phase_count)
        .unwrap_or(0)
}

pub(crate) fn effort_scale_label(effort: &str, selected: bool) -> String {
    let marker = if selected {
        match effort {
            "high" | "xhigh" | "max" => "✦",
            _ => "›",
        }
    } else {
        " "
    };
    format!("{marker} {effort:<6}")
}

pub(crate) fn effort_label_spans(
    label: &str,
    effort: &str,
    selected: bool,
    tick: u64,
) -> Vec<Span<'static>> {
    let animated = selected && matches!(effort, "high" | "xhigh" | "max");
    if !animated {
        return vec![Span::styled(
            label.to_string(),
            effort_style(effort, selected),
        )];
    }

    let mut spans = Vec::new();
    let chars = label.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().enumerate() {
        let color = shimmer_color(effort, index, tick);
        spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }
    spans
}

pub(crate) fn shimmer_text_spans(
    text: &str,
    effort: &str,
    active: bool,
    tick: u64,
) -> Vec<Span<'static>> {
    if !active {
        return vec![Span::styled(text.to_string(), effort_style(effort, true))];
    }

    text.chars()
        .enumerate()
        .map(|(index, ch)| {
            Span::styled(
                ch.to_string(),
                Style::default()
                    .fg(shimmer_color(effort, index, tick))
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect()
}

pub(crate) fn shimmer_color(effort: &str, index: usize, tick: u64) -> Color {
    let palette: &[u8] = match effort {
        "high" => &[136, 178, 220, 214, 220, 178, 136],
        "xhigh" => &[97, 141, 183, 219, 183, 141, 97],
        "max" => &[160, 198, 203, 205, 203, 198, 160],
        _ => &[250],
    };
    let phase = (tick as usize) % palette.len();
    let color_index = (index + palette.len() - phase) % palette.len();
    Color::Indexed(palette[color_index])
}

pub(crate) fn effort_style(effort: &str, selected: bool) -> Style {
    let color = match effort {
        "low" => Color::Indexed(114),
        "medium" => Color::Indexed(117),
        "high" => Color::Indexed(220),
        "xhigh" => ACCENT_SOFT,
        "max" => Color::Indexed(203),
        _ => MUTED,
    };

    let mut style = Style::default().fg(color);
    if selected {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

pub(crate) fn effort_description(effort: &str, lang: Language) -> &'static str {
    match effort {
        "low" => lang.choose("быстро и экономно", "fast and frugal"),
        "medium" => lang.choose("баланс скорости и качества", "balanced speed and quality"),
        "high" => lang.choose("глубже думает над задачей", "deeper task reasoning"),
        "xhigh" => lang.choose("максимум Codex reasoning", "maximum Codex reasoning"),
        "max" => lang.choose("максимум Claude effort", "maximum Claude effort"),
        _ => "",
    }
}

pub(crate) fn composer_height(app: &App, width: u16) -> u16 {
    let lines = input_lines_wrapped(&app.input, width).len() as u16;
    (lines + 2).clamp(3, 10)
}

pub(crate) fn initial_transcript(_lang: Language) -> Vec<String> {
    Vec::new()
}

pub(crate) fn provider_count() -> usize {
    3
}

pub(crate) fn provider_mode(index: usize) -> Mode {
    match index {
        0 => Mode::CodexOnly,
        1 => Mode::ClaudeCodex,
        2 => Mode::ClaudeOnly,
        _ => Mode::CodexOnly,
    }
}

pub(crate) fn provider_index(mode: Mode) -> usize {
    match mode {
        Mode::CodexOnly => 0,
        Mode::ClaudeCodex => 1,
        Mode::ClaudeOnly => 2,
    }
}

pub(crate) fn provider_description(mode: Mode, lang: Language) -> &'static str {
    match mode {
        Mode::CodexOnly => lang.choose("Codex пишет и ревьюит", "Codex drafts and reviews"),
        Mode::ClaudeCodex => lang.choose(
            "Claude пишет, Codex ревьюит",
            "Claude drafts, Codex reviews",
        ),
        Mode::ClaudeOnly => lang.choose("Claude пишет и ревьюит", "Claude drafts and reviews"),
    }
}

pub(crate) fn input_lines_wrapped(input: &str, width: u16) -> Vec<String> {
    let content_width = width.saturating_sub(2).max(1) as usize;
    if input.is_empty() {
        return vec![String::new()];
    }

    let mut rows = Vec::new();
    for line in input.split('\n') {
        rows.extend(wrap_terminal_text_preserving_spaces(line, content_width));
    }
    rows
}

pub(crate) fn input_cursor_position_wrapped(
    input: &str,
    cursor: usize,
    width: u16,
) -> (usize, usize) {
    let content_width = width.saturating_sub(2).max(1) as usize;
    let before = &input[..cursor];
    let parts = before.split('\n').collect::<Vec<_>>();
    let mut visual_line = 0usize;
    let mut visual_col = 0usize;

    for (index, line) in parts.iter().enumerate() {
        let len = line.chars().count();
        if index + 1 == parts.len() {
            visual_line += len / content_width;
            visual_col = len % content_width;
        } else {
            visual_line += (len / content_width) + 1;
        }
    }

    (visual_line, visual_col)
}

pub(crate) fn wrap_terminal_line(text: &str, width: u16) -> Vec<String> {
    let max_chars = width.saturating_sub(1).max(1) as usize;
    wrap_terminal_text_preserving_spaces(text, max_chars)
}

pub(crate) fn wrap_terminal_text_preserving_spaces(text: &str, max_chars: usize) -> Vec<String> {
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

pub(crate) fn wrap_chars(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let max_chars = max_chars.max(1);
    let mut rows = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let current_len = current.chars().count();
        let word_len = word.chars().count();
        let extra_space = usize::from(!current.is_empty());

        if current_len + extra_space + word_len > max_chars && !current.is_empty() {
            rows.push(current);
            current = String::new();
        }

        if word_len > max_chars {
            if !current.is_empty() {
                rows.push(current);
                current = String::new();
            }

            let mut chunk = String::new();
            for ch in word.chars() {
                if chunk.chars().count() >= max_chars {
                    rows.push(chunk);
                    chunk = String::new();
                }
                chunk.push(ch);
            }
            if !chunk.is_empty() {
                current = chunk;
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        rows.push(current);
    }
    rows
}

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

pub(crate) fn main_area_height(
    app: &App,
    area: Rect,
    composer_height: u16,
    palette_height: u16,
    footer_height: u16,
    output_gap: u16,
    palette_gap: u16,
) -> u16 {
    let max_height = area
        .height
        .saturating_sub(composer_height)
        .saturating_sub(palette_height)
        .saturating_sub(footer_height)
        .saturating_sub(output_gap)
        .saturating_sub(palette_gap)
        .max(1);

    let desired = if app.transcript.is_empty() && !app.running && app.last_run.is_none() {
        area.height.min(12).max(1)
    } else {
        transcript_content_height(app, area.width).max(1)
    };

    desired.min(max_height)
}

pub(crate) fn transcript_content_height(app: &App, width: u16) -> u16 {
    let mut height = 1usize;
    for line in &app.transcript {
        height += transcript_entry_lines(line, app.lang, width).len();
    }
    if app.running {
        height += 2;
    }
    height.min(u16::MAX as usize) as u16
}

pub(crate) fn command_palette_height(app: &App, screen_height: u16, composer_height: u16) -> u16 {
    let suggestions = app.suggestions();
    let command_count = if suggestions.is_empty() {
        COMMANDS.len()
    } else {
        suggestions.len()
    };
    let available = screen_height
        .saturating_sub(composer_height)
        .saturating_sub(6)
        .max(3);
    (command_count as u16).min(available).min(12)
}
