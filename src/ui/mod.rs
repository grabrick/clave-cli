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
