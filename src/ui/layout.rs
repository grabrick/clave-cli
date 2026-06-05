use super::*;

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
    height += transcript_lines(&app.transcript, app.lang, width, app.theme).len();
    if app.running {
        height += 2;
    }
    height.min(u16::MAX as usize) as u16
}

pub(crate) fn command_palette_height(app: &App, screen_height: u16, composer_height: u16) -> u16 {
    const COMMAND_PALETTE_ROWS: u16 = 12;

    if normalized_command_query(&app.input).is_none() {
        return 0;
    }

    let available = screen_height
        .saturating_sub(composer_height)
        .saturating_sub(6)
        .max(3);
    COMMAND_PALETTE_ROWS.min(available)
}
