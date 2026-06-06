use super::*;

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
