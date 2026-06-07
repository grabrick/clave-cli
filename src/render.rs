use crate::prelude::*;
use crate::*;

use crossterm::{
    cursor::{Hide, MoveDown, MoveRight, MoveToColumn, MoveUp, Show},
    queue,
    style::{
        Attribute as CtAttr, Color as CtColor, Print, ResetColor, SetAttribute, SetBackgroundColor,
        SetForegroundColor,
    },
    terminal::{Clear, ClearType},
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

/// Живой нижний блок, перерисовываемый «на месте» (модель Ink / Claude Code).
///
/// История уходит в НАТИВНЫЙ скроллбэк терминала (печатается один раз), а блок
/// `[панель|loader][поле ввода][футер]` каждый кадр стирается и рисуется заново
/// прямо под историей. Высота блока меняется свободно — поэтому открытие меню
/// разворачивает блок «на месте» без сдвига истории и без накопления пустоты, а
/// закрытие чисто его схлопывает. Колесо/выделение работают (история = скроллбэк).
pub(crate) struct LiveRenderer {
    started: bool,
    /// Высота блока в прошлом кадре (строк на экране).
    prev_height: u16,
    /// На сколько строк выше нижней строки блока стоял курсор ввода.
    cursor_above: u16,
    /// Подпись блока прошлого кадра — чтобы не перерисовывать в покое (без мерцания).
    last_signature: String,
}

impl LiveRenderer {
    pub(crate) fn new() -> Self {
        Self {
            started: false,
            prev_height: 0,
            cursor_above: 0,
            last_signature: String::new(),
        }
    }

    /// Заставляет следующий кадр перерисоваться полностью (после модалок/внешних команд).
    pub(crate) fn invalidate(&mut self) {
        self.last_signature.clear();
    }

    /// Кадр: вытесняет новую историю в скроллбэк и перерисовывает живой блок.
    pub(crate) fn render(&mut self, app: &mut App, width: u16, full_h: u16) -> io::Result<()> {
        let (lines, cur_row, cur_col) = build_dynamic(app, width, full_h);
        let has_new_history = app.scrollback_count < app.transcript.len();
        let signature = dynamic_signature(&lines, cur_row, cur_col);
        if self.started && !has_new_history && signature == self.last_signature {
            return Ok(()); // ничего не изменилось — не трогаем экран
        }

        let mut out = io::stdout().lock();
        queue!(out, Hide)?;

        // 1) Встать в начало прошлого блока и стереть его (вместе с тем, что ниже).
        if self.started {
            if self.cursor_above > 0 {
                queue!(out, MoveDown(self.cursor_above))?;
            }
            queue!(out, MoveToColumn(0))?;
            if self.prev_height > 1 {
                queue!(out, MoveUp(self.prev_height - 1))?;
            }
        } else {
            queue!(out, MoveToColumn(0))?;
        }
        queue!(out, Clear(ClearType::FromCursorDown))?;

        // 2) Новая история — построчно в скроллбэк (печатается один раз).
        while app.scrollback_count < app.transcript.len() {
            let raw = app.transcript[app.scrollback_count].clone();
            let rows = history_line_render(&raw, app.lang, width, app.theme, &mut app.flush_state);
            for row in &rows {
                queue_line(&mut out, row)?;
                queue!(out, Clear(ClearType::UntilNewLine), Print("\r\n"))?;
            }
            app.scrollback_count += 1;
        }

        // 3) Живой блок.
        let height = lines.len() as u16;
        for (index, line) in lines.iter().enumerate() {
            queue_line(&mut out, line)?;
            queue!(out, Clear(ClearType::UntilNewLine))?;
            if index + 1 < lines.len() {
                queue!(out, Print("\r\n"))?;
            }
        }

        // 4) Поставить курсор в поле ввода.
        queue!(out, MoveToColumn(0))?;
        let last = height.saturating_sub(1);
        if last > cur_row {
            queue!(out, MoveUp(last - cur_row))?;
        }
        if cur_col > 0 {
            queue!(out, MoveRight(cur_col))?;
        }
        queue!(out, Show)?;
        out.flush()?;

        self.prev_height = height;
        self.cursor_above = last.saturating_sub(cur_row);
        self.last_signature = signature;
        self.started = true;
        Ok(())
    }

    /// Перед внешней командой / выходом: увести курсор под блок на чистую строку,
    /// чтобы дальнейший вывод не затирал живой блок.
    pub(crate) fn leave_below(&mut self) -> io::Result<()> {
        if !self.started {
            return Ok(());
        }
        let mut out = io::stdout().lock();
        if self.cursor_above > 0 {
            queue!(out, MoveDown(self.cursor_above))?;
        }
        queue!(out, MoveToColumn(0), Print("\r\n"))?;
        out.flush()?;
        self.started = false;
        self.prev_height = 0;
        self.cursor_above = 0;
        self.last_signature.clear();
        Ok(())
    }
}

/// Рендерит живой блок в оффскрин-буфер (переиспользуя обычные виджеты ratatui,
/// включая рамки) и возвращает его строки + позицию курсора ввода в блоке.
fn build_dynamic(app: &App, width: u16, full_h: u16) -> (Vec<Line<'static>>, u16, u16) {
    let width = width.max(1);
    let composer = composer_height(app, width);
    let footer = 1u16;
    let room = full_h
        .saturating_sub(1) // оставить хотя бы строку под историю/скроллбэк
        .saturating_sub(composer + footer);
    let bottom = body_bottom_height(app, width, room);
    let height = (bottom + composer + footer)
        .min(full_h.saturating_sub(1).max(1))
        .max(composer + footer);

    let mut terminal = match Terminal::new(TestBackend::new(width, height)) {
        Ok(terminal) => terminal,
        Err(_) => return (Vec::new(), 0, 0),
    };
    let lines = terminal
        .draw(|frame| {
            let area = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(bottom),
                    Constraint::Length(composer),
                    Constraint::Length(footer),
                ])
                .split(area);
            if bottom > 0 {
                if panel_active(app) {
                    draw_active_panel(frame, chunks[0], app);
                } else if app.running {
                    frame.render_widget(Paragraph::new(loader_lines(app, width)), chunks[0]);
                }
            }
            draw_prompt_bar(frame, chunks[1], app);
            draw_footer(frame, chunks[2], app);
        })
        .map(|completed| buffer_to_lines(completed.buffer))
        .unwrap_or_default();

    let (line_index, col) = input_cursor_position_wrapped(&app.input, app.cursor, width);
    let cur_row = (bottom + 1 + line_index as u16).min(height.saturating_sub(1));
    let cur_col = (2 + col as u16).min(width.saturating_sub(1));
    (lines, cur_row, cur_col)
}

/// Превращает строки оффскрин-буфера в `Line`, схлопывая одинаковые стили в спаны.
fn buffer_to_lines(buf: &Buffer) -> Vec<Line<'static>> {
    let area = buf.area;
    (0..area.height)
        .map(|y| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut text = String::new();
            let mut current: Option<Style> = None;
            for x in 0..area.width {
                let Some(cell) = buf.cell((area.x + x, area.y + y)) else {
                    continue;
                };
                let style = Style::default()
                    .fg(cell.fg)
                    .bg(cell.bg)
                    .add_modifier(cell.modifier);
                if current != Some(style) {
                    if !text.is_empty() {
                        spans.push(Span::styled(
                            std::mem::take(&mut text),
                            current.unwrap_or_default(),
                        ));
                    }
                    current = Some(style);
                }
                text.push_str(cell.symbol());
            }
            if !text.is_empty() {
                spans.push(Span::styled(text, current.unwrap_or_default()));
            }
            Line::from(spans)
        })
        .collect()
}

/// Подпись блока для определения «ничего не изменилось» (текст + позиция курсора).
fn dynamic_signature(lines: &[Line<'static>], cur_row: u16, cur_col: u16) -> String {
    let mut signature = format!("{cur_row}:{cur_col}\n");
    for line in lines {
        for span in &line.spans {
            signature.push_str(span.content.as_ref());
        }
        signature.push('\n');
    }
    signature
}

fn queue_line(out: &mut impl Write, line: &Line<'static>) -> io::Result<()> {
    for span in &line.spans {
        apply_style(out, span.style)?;
        queue!(out, Print(span.content.as_ref()))?;
        queue!(out, SetAttribute(CtAttr::Reset), ResetColor)?;
    }
    Ok(())
}

fn apply_style(out: &mut impl Write, style: Style) -> io::Result<()> {
    if let Some(fg) = style.fg {
        queue!(out, SetForegroundColor(to_crossterm_color(fg)))?;
    }
    if let Some(bg) = style.bg {
        queue!(out, SetBackgroundColor(to_crossterm_color(bg)))?;
    }
    let modifier = style.add_modifier;
    if modifier.contains(Modifier::BOLD) {
        queue!(out, SetAttribute(CtAttr::Bold))?;
    }
    if modifier.contains(Modifier::DIM) {
        queue!(out, SetAttribute(CtAttr::Dim))?;
    }
    if modifier.contains(Modifier::ITALIC) {
        queue!(out, SetAttribute(CtAttr::Italic))?;
    }
    if modifier.contains(Modifier::UNDERLINED) {
        queue!(out, SetAttribute(CtAttr::Underlined))?;
    }
    if modifier.contains(Modifier::REVERSED) {
        queue!(out, SetAttribute(CtAttr::Reverse))?;
    }
    if modifier.contains(Modifier::CROSSED_OUT) {
        queue!(out, SetAttribute(CtAttr::CrossedOut))?;
    }
    Ok(())
}

/// Точное соответствие маппингу ratatui-crossterm (чтобы цвета совпадали 1:1).
fn to_crossterm_color(color: Color) -> CtColor {
    match color {
        Color::Reset => CtColor::Reset,
        Color::Black => CtColor::Black,
        Color::Red => CtColor::DarkRed,
        Color::Green => CtColor::DarkGreen,
        Color::Yellow => CtColor::DarkYellow,
        Color::Blue => CtColor::DarkBlue,
        Color::Magenta => CtColor::DarkMagenta,
        Color::Cyan => CtColor::DarkCyan,
        Color::Gray => CtColor::Grey,
        Color::DarkGray => CtColor::DarkGrey,
        Color::LightRed => CtColor::Red,
        Color::LightGreen => CtColor::Green,
        Color::LightBlue => CtColor::Blue,
        Color::LightYellow => CtColor::Yellow,
        Color::LightMagenta => CtColor::Magenta,
        Color::LightCyan => CtColor::Cyan,
        Color::White => CtColor::White,
        Color::Indexed(i) => CtColor::AnsiValue(i),
        Color::Rgb(r, g, b) => CtColor::Rgb { r, g, b },
    }
}
