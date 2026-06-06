use super::*;

pub(crate) fn shortcuts_panel_height(lang: Language, width: u16) -> u16 {
    let rows = shortcut_rows(lang, width.saturating_sub(2)).len() as u16;
    (rows + 2).clamp(3, 8)
}

pub(crate) fn draw_shortcuts_panel(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(Span::styled(
            app.lang.choose(" Управление ", " Controls "),
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::default().fg(app.theme.accent_dim()));

    let rows: Vec<Line> = shortcut_rows(app.lang, area.width.saturating_sub(2))
        .into_iter()
        .map(Line::from)
        .collect();

    let paragraph = Paragraph::new(rows)
        .block(block)
        .style(Style::default().fg(MUTED))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

/// Складывает хоткеи из `SHORTCUTS` в строки по доступной ширине панели.
fn shortcut_rows(lang: Language, width: u16) -> Vec<String> {
    let width = width.max(10) as usize;
    let sep = " · ";
    let mut rows = Vec::new();
    let mut current = String::new();

    for spec in SHORTCUTS {
        let part = format!("{} {}", spec.keys, spec.describe(lang));
        if current.is_empty() {
            current = part;
        } else if current.chars().count() + sep.chars().count() + part.chars().count() <= width {
            current.push_str(sep);
            current.push_str(&part);
        } else {
            rows.push(std::mem::take(&mut current));
            current = part;
        }
    }
    if !current.is_empty() {
        rows.push(current);
    }
    rows
}
