use crate::prelude::*;
use crate::*;

pub(crate) mod command_palette;
pub(crate) mod effort;
pub(crate) mod footer;
pub(crate) mod helpers;
pub(crate) mod layout;
pub(crate) mod loader;
pub(crate) mod onboarding;
pub(crate) mod prompt;
pub(crate) mod transcript;
pub(crate) mod welcome;

pub(crate) use command_palette::*;
pub(crate) use effort::*;
pub(crate) use footer::*;
pub(crate) use helpers::*;
pub(crate) use layout::*;
pub(crate) use loader::*;
pub(crate) use onboarding::*;
pub(crate) use prompt::*;
pub(crate) use transcript::*;
pub(crate) use welcome::*;

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

    let command_mode = normalized_command_query(&app.input).is_some();
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
