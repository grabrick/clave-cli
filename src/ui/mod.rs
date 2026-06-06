use crate::prelude::*;
use crate::*;

pub(crate) mod chats;
pub(crate) mod command_palette;
pub(crate) mod effort;
pub(crate) mod footer;
pub(crate) mod helpers;
pub(crate) mod layout;
pub(crate) mod loader;
pub(crate) mod onboarding;
pub(crate) mod plan_gate;
pub(crate) mod prompt;
pub(crate) mod search;
pub(crate) mod settings;
pub(crate) mod shortcuts;
pub(crate) mod transcript;
pub(crate) mod welcome;

pub(crate) use chats::*;
pub(crate) use command_palette::*;
pub(crate) use effort::*;
pub(crate) use footer::*;
pub(crate) use helpers::*;
pub(crate) use layout::*;
pub(crate) use loader::*;
pub(crate) use onboarding::*;
pub(crate) use plan_gate::*;
pub(crate) use prompt::*;
pub(crate) use search::*;
pub(crate) use settings::*;
pub(crate) use shortcuts::*;
pub(crate) use transcript::*;
pub(crate) use welcome::*;

pub(crate) fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    if app.onboarding.is_some() {
        draw_onboarding(frame, area, app);
        return;
    }

    match app.overlay {
        Overlay::Effort => {
            draw_effort_screen(frame, area, app);
            return;
        }
        Overlay::Settings => {
            draw_settings_screen(frame, area, app);
            return;
        }
        Overlay::Chats => {
            draw_chats_screen(frame, area, app);
            return;
        }
        Overlay::None | Overlay::Shortcuts | Overlay::Search => {}
    }

    let is_welcome = app.transcript.is_empty() && !app.running && app.last_run.is_none();
    let command_mode = normalized_command_query(&app.input).is_some();
    let shortcuts_mode = app.overlay == Overlay::Shortcuts;
    let search_mode = app.overlay == Overlay::Search;
    let gate_mode = app.plan_gate_active() && !command_mode && !shortcuts_mode && !search_mode;
    let composer_height = composer_height(app, area.width).min(area.height.saturating_sub(2));
    let palette_height = if command_mode {
        command_palette_height(app, area.height, composer_height)
    } else if shortcuts_mode {
        shortcuts_panel_height(app.lang, area.width)
    } else if search_mode {
        search_panel_height()
    } else if gate_mode {
        plan_gate_panel_height()
    } else {
        0
    };
    let footer_height = if command_mode { 0 } else { 1 };
    let output_gap = if is_welcome { 0 } else { 1 };
    let palette_gap = if command_mode || shortcuts_mode || search_mode || gate_mode {
        1
    } else {
        0
    };

    // Транскрипт берём из кэша (refresh_transcript_cache пересобирает его лишь при
    // изменении содержимого); loader-хвост анимируется, поэтому строится каждый кадр.
    let cached_transcript = app
        .transcript_cache
        .as_ref()
        .map(|(_, lines)| lines.as_slice())
        .unwrap_or(&[]);
    let loader_tail = loader_tail_lines(app, area.width);
    let content_height = if is_welcome {
        area.height.min(12).max(1)
    } else {
        ((1 + cached_transcript.len() + loader_tail.len()).min(u16::MAX as usize) as u16).max(1)
    };
    let main_height = main_area_height(
        area,
        content_height,
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

    if is_welcome {
        draw_welcome(frame, chunks[0], app);
    } else {
        draw_transcript(
            frame,
            chunks[0],
            app,
            area.width,
            cached_transcript,
            &loader_tail,
        );
    }
    draw_prompt_bar(frame, chunks[2], app);
    if command_mode {
        draw_command_screen(frame, chunks[4], app);
    } else {
        if shortcuts_mode {
            draw_shortcuts_panel(frame, chunks[4], app);
        } else if search_mode {
            draw_search_panel(frame, chunks[4], app);
        } else if gate_mode {
            draw_plan_gate_panel(frame, chunks[4], app);
        }
        draw_footer(frame, chunks[5], app);
    }
}

// ── Inline-рендеринг (живой viewport) ───────────────────────────────────────

/// Чистая арифметика высоты viewport (тестируется отдельно). footer = 1 строка.
pub(crate) fn viewport_height_parts(composer: u16, panel: u16, loader: u16) -> u16 {
    let gap = if panel > 0 { 1 } else { 0 };
    composer
        .saturating_add(gap)
        .saturating_add(panel)
        .saturating_add(loader)
        .saturating_add(1)
        .max(3)
}

/// Единственный источник истины высоты живого viewport (инвариант 5).
pub(crate) fn desired_viewport_height(app: &App, width: u16) -> u16 {
    let composer = composer_height(app, width);
    let panel = active_panel_height(app, width);
    let loader = if app.running {
        loader_lines(app, width).len() as u16
    } else {
        0
    };
    viewport_height_parts(composer, panel, loader)
}

/// Высота активной inline-панели (палитра/?/search/gate) или 0.
pub(crate) fn active_panel_height(app: &App, width: u16) -> u16 {
    if normalized_command_query(&app.input).is_some() {
        command_palette_height(app, 30, composer_height(app, width))
    } else if app.overlay == Overlay::Shortcuts {
        shortcuts_panel_height(app.lang, width)
    } else if app.overlay == Overlay::Search {
        search_panel_height()
    } else if app.plan_gate_active() {
        plan_gate_panel_height()
    } else {
        0
    }
}

/// Рисует ТОЛЬКО живой viewport (инвариант 2): loader + композер + inline-панель + футер.
pub(crate) fn draw_viewport(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let width = area.width;
    let composer = composer_height(app, width);
    let panel = active_panel_height(app, width);
    let loader = if app.running {
        loader_lines(app, width).len() as u16
    } else {
        0
    };
    let gap = if panel > 0 { 1 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(loader),
            Constraint::Length(composer),
            Constraint::Length(gap),
            Constraint::Length(panel),
            Constraint::Length(1),
        ])
        .split(area);

    if loader > 0 {
        frame.render_widget(Paragraph::new(loader_lines(app, width)), chunks[0]);
    }
    draw_prompt_bar(frame, chunks[1], app);
    draw_active_panel(frame, chunks[3], app);
    draw_footer(frame, chunks[4], app);
}

fn draw_active_panel(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if area.height == 0 {
        return;
    }
    if normalized_command_query(&app.input).is_some() {
        draw_command_screen(frame, area, app);
    } else if app.overlay == Overlay::Shortcuts {
        draw_shortcuts_panel(frame, area, app);
    } else if app.overlay == Overlay::Search {
        draw_search_panel(frame, area, app);
    } else if app.plan_gate_active() {
        draw_plan_gate_panel(frame, area, app);
    }
}

/// Полноэкранная модалка (effort/settings/chats/onboarding) — рисуется в временном
/// alt-screen (инвариант 4).
pub(crate) fn draw_modal(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    frame.render_widget(Clear, area);
    if app.onboarding.is_some() {
        draw_onboarding(frame, area, app);
        return;
    }
    match app.overlay {
        Overlay::Effort => draw_effort_screen(frame, area, app),
        Overlay::Settings => draw_settings_screen(frame, area, app),
        Overlay::Chats => draw_chats_screen(frame, area, app),
        _ => {}
    }
}
