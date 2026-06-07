use crate::prelude::*;
use crate::*;

pub(crate) mod chats;
pub(crate) mod command_palette;
pub(crate) mod effort;
pub(crate) mod footer;
pub(crate) mod helpers;
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
pub(crate) use loader::*;
pub(crate) use onboarding::*;
pub(crate) use plan_gate::*;
pub(crate) use prompt::*;
pub(crate) use search::*;
pub(crate) use settings::*;
pub(crate) use shortcuts::*;
pub(crate) use transcript::*;

// ── Живой нижний регион (фиксированная высота, перерисовка на месте) ─────────

/// Максимальная высота живого региона: вмещает самую большую панель
/// (`COMMAND_PALETTE_ROWS`) + композер + футер.
pub(crate) const COMMAND_PALETTE_ROWS: u16 = 12;
pub(crate) const LIVE_VIEWPORT_MAX: u16 = 16;

/// Высота живого региона для текущего размера окна. Фиксирована за сессию,
/// меняется ТОЛЬКО при ресайзе окна — поэтому открытие панелей ничего не двигает.
pub(crate) fn live_viewport_height(full_h: u16) -> u16 {
    full_h
        .saturating_sub(2)
        .clamp(4, LIVE_VIEWPORT_MAX)
        .min(full_h.max(1))
}

/// Рисует живой регион: хвост истории + (панель|loader поверх него) + композер +
/// футер. `tail` — уже отрендеренные строки хвоста (см. `flush_overflow`).
pub(crate) fn draw_viewport(frame: &mut Frame<'_>, app: &App, tail: &[Line<'static>]) {
    let area = frame.area();
    let composer = composer_height(app, area.width);
    let footer = 1u16;
    let body = area.height.saturating_sub(composer).saturating_sub(footer);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(body),
            Constraint::Length(composer),
            Constraint::Length(footer),
        ])
        .split(area);

    draw_body(frame, chunks[0], app, tail);
    draw_prompt_bar(frame, chunks[1], app);
    draw_footer(frame, chunks[2], app);
}

/// Тело: хвост истории сверху, активная панель ИЛИ loader — внизу, ПОВЕРХ хвоста
/// (не сдвигая историю — в этом весь смысл фиксированной высоты).
fn draw_body(frame: &mut Frame<'_>, area: Rect, app: &App, tail: &[Line<'static>]) {
    if area.height == 0 {
        return;
    }
    let bottom = body_bottom_height(app, area.width, area.height);
    let tail_h = area.height.saturating_sub(bottom);

    if tail_h > 0 {
        let tail_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: tail_h,
        };
        draw_tail(frame, tail_area, tail);
    }
    if bottom > 0 {
        let bottom_area = Rect {
            x: area.x,
            y: area.y + tail_h,
            width: area.width,
            height: bottom,
        };
        if panel_active(app) {
            draw_active_panel(frame, bottom_area, app);
        } else if app.running {
            frame.render_widget(Paragraph::new(loader_lines(app, area.width)), bottom_area);
        }
    }
}

/// Хвост истории, выровненный по низу области (свежее — у самого композера).
fn draw_tail(frame: &mut Frame<'_>, area: Rect, tail: &[Line<'static>]) {
    if area.height == 0 || tail.is_empty() {
        return;
    }
    let visible = (area.height as usize).min(tail.len());
    let shown = tail[tail.len() - visible..].to_vec();
    let used = visible as u16;
    let render_area = Rect {
        x: area.x,
        y: area.y + area.height - used,
        width: area.width,
        height: used,
    };
    frame.render_widget(Paragraph::new(shown), render_area);
}

/// Активна ли под-композерная панель (палитра/подсказки/поиск/гейт).
fn panel_active(app: &App) -> bool {
    normalized_command_query(&app.input).is_some()
        || app.overlay == Overlay::Shortcuts
        || app.overlay == Overlay::Search
        || app.plan_gate_active()
}

/// Высота нижнего элемента тела (панель или loader), обрезанная по доступному телу.
fn body_bottom_height(app: &App, width: u16, body: u16) -> u16 {
    let height = if normalized_command_query(&app.input).is_some() {
        COMMAND_PALETTE_ROWS
    } else if app.overlay == Overlay::Shortcuts {
        shortcuts_panel_height(app.lang, width)
    } else if app.overlay == Overlay::Search {
        search_panel_height()
    } else if app.plan_gate_active() {
        plan_gate_panel_height()
    } else if app.running {
        loader_lines(app, width).len() as u16
    } else {
        0
    };
    height.min(body)
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

/// Полноэкранная модалка (effort/settings/chats/onboarding) во временном alt-screen.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_viewport_height_fits_panel_and_caps() {
        // На обычном экране — максимум, и он вмещает палитру + композер(3) + футер.
        assert_eq!(live_viewport_height(24), LIVE_VIEWPORT_MAX);
        assert!(LIVE_VIEWPORT_MAX >= COMMAND_PALETTE_ROWS + 3 + 1);
        // На маленьком экране ужимается, но остаётся в пределах [часть экрана; экран].
        assert_eq!(live_viewport_height(10), 8);
        assert!(live_viewport_height(5) <= 5);
        assert!(live_viewport_height(40) <= LIVE_VIEWPORT_MAX);
    }
}
