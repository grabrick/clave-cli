use super::*;

pub(crate) fn main_area_height(
    area: Rect,
    content_height: u16,
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

    content_height.max(1).min(max_height)
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
