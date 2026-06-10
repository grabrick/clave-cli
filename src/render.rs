use crate::prelude::*;
use crate::*;

use crossterm::{
    cursor::{Hide, MoveDown, MoveRight, MoveTo, MoveToColumn, MoveUp, Show},
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
    /// Строки блока прошлого кадра — для дифф-перерисовки (правим только изменившиеся).
    prev_lines: Vec<Line<'static>>,
    /// Позиция курсора ввода прошлого кадра (строка, столбец) внутри блока.
    prev_cursor: (u16, u16),
}

impl LiveRenderer {
    pub(crate) fn new() -> Self {
        Self {
            started: false,
            prev_height: 0,
            cursor_above: 0,
            prev_lines: Vec::new(),
            prev_cursor: (0, 0),
        }
    }

    /// Заставляет следующий кадр перерисоваться полностью (после модалок/внешних команд).
    pub(crate) fn invalidate(&mut self) {
        self.prev_lines.clear();
    }

    /// Кадр: вытесняет новую историю в скроллбэк и обновляет живой блок.
    ///
    /// Полная перерисовка блока только при структурных изменениях (новая история,
    /// смена высоты, первый кадр). В остальных случаях — ДИФФ по строкам: правим
    /// лишь изменившиеся (цвет/текст), не трогая остальные → нет мерцания футера, а
    /// анимация появления палитры (меняется цвет) проигрывается.
    pub(crate) fn render(&mut self, app: &mut App, width: u16, full_h: u16) -> io::Result<()> {
        // Полная очистка терминала по запросу (/clear, /new, /resume): стираем
        // экран И нативный скроллбэк, иначе старая напечатанная история остаётся.
        if app.pending_clear_screen {
            app.pending_clear_screen = false;
            {
                let mut out = io::stdout().lock();
                queue!(
                    out,
                    Clear(ClearType::All),
                    Clear(ClearType::Purge),
                    MoveTo(0, 0)
                )?;
                out.flush()?;
            }
            self.started = false;
            self.prev_height = 0;
            self.cursor_above = 0;
            self.prev_lines.clear();
        }

        // Полная перерисовка после ресайза: терминал перелил историю под новую
        // ширину, а наш кэш позиций (prev_height/cursor_above) описывает старую
        // геометрию — относительные сдвиги курсора «съедут» и живой блок начнёт
        // дублироваться. Чистим экран И скроллбэк, сбрасываем счётчик истории и
        // состояние подсветки, чтобы структурный путь ниже перепечатал всё заново.
        if app.pending_full_redraw {
            app.pending_full_redraw = false;
            {
                let mut out = io::stdout().lock();
                queue!(
                    out,
                    Clear(ClearType::All),
                    Clear(ClearType::Purge),
                    MoveTo(0, 0)
                )?;
                out.flush()?;
            }
            self.started = false;
            self.prev_height = 0;
            self.cursor_above = 0;
            self.prev_lines.clear();
            app.scrollback_count = 0;
            app.flush_state = TranscriptRenderState::default();
        }

        let (lines, cur_row, cur_col) = build_dynamic(app, width, full_h);
        let has_new_history = app.scrollback_count < app.transcript.len();
        let structural = !self.started || has_new_history || lines.len() != self.prev_lines.len();

        if !structural && lines == self.prev_lines && (cur_row, cur_col) == self.prev_cursor {
            return Ok(()); // ничего не изменилось
        }

        let height = lines.len() as u16;
        let last = height.saturating_sub(1);
        let mut out = io::stdout().lock();
        queue!(out, Hide)?;

        if structural {
            // Полная перерисовка: стереть старый блок, вывести новую историю, блок.
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

            while app.scrollback_count < app.transcript.len() {
                let raw = app.transcript[app.scrollback_count].clone();
                let rows =
                    history_line_render(&raw, app.lang, width, app.theme, &mut app.flush_state);
                for row in &rows {
                    queue_line(&mut out, row)?;
                    queue!(out, Clear(ClearType::UntilNewLine), Print("\r\n"))?;
                }
                app.scrollback_count += 1;
            }

            for (index, line) in lines.iter().enumerate() {
                queue_line(&mut out, line)?;
                queue!(out, Clear(ClearType::UntilNewLine))?;
                if index + 1 < lines.len() {
                    queue!(out, Print("\r\n"))?;
                }
            }
        } else {
            // Дифф: встать на верх блока и перерисовать только изменившиеся строки.
            if self.cursor_above > 0 {
                queue!(out, MoveDown(self.cursor_above))?;
            }
            queue!(out, MoveToColumn(0))?;
            if last > 0 {
                queue!(out, MoveUp(last))?;
            }
            for (index, line) in lines.iter().enumerate() {
                queue!(out, MoveToColumn(0))?;
                if self.prev_lines.get(index) != Some(line) {
                    queue_line(&mut out, line)?;
                    queue!(out, Clear(ClearType::UntilNewLine))?;
                }
                if index + 1 < lines.len() {
                    queue!(out, MoveDown(1))?;
                }
            }
        }

        // Поставить курсор в поле ввода (он сейчас на последней строке блока).
        queue!(out, MoveToColumn(0))?;
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
        self.prev_lines = lines;
        self.prev_cursor = (cur_row, cur_col);
        self.started = true;
        Ok(())
    }

    /// Перед внешней командой: СТИРАЕТ живой блок целиком, оставляя на экране
    /// историю диалога. Вывод команды печатается на месте блока, а блок потом
    /// перерисуется (invalidate). Для выхода из приложения см. `clear_for_exit`.
    pub(crate) fn leave_below(&mut self) -> io::Result<()> {
        if !self.started {
            return Ok(());
        }
        let mut out = io::stdout().lock();
        // встать на нижнюю строку блока → на верх блока → стереть от курсора вниз
        if self.cursor_above > 0 {
            queue!(out, MoveDown(self.cursor_above))?;
        }
        queue!(out, MoveToColumn(0))?;
        if self.prev_height > 1 {
            queue!(out, MoveUp(self.prev_height - 1))?;
        }
        queue!(out, Clear(ClearType::FromCursorDown), Show)?;
        out.flush()?;
        self.started = false;
        self.prev_height = 0;
        self.cursor_above = 0;
        self.prev_lines.clear();
        Ok(())
    }

    /// При выходе из приложения: чистим ВЕСЬ видимый экран и уводим курсор в начало,
    /// чтобы оболочка получила пустой терминал, а не остатки беседы (в inline-режиме
    /// история живёт в нативном скроллбэке, и `leave_below` стёр бы только блок).
    /// Скроллбэк НЕ пуржим: это снесло бы и то, что было в терминале до запуска clave.
    /// Сама беседа не теряется — она сохранена в файле чата (вернуть через /chats).
    pub(crate) fn clear_for_exit(&mut self) -> io::Result<()> {
        let mut out = io::stdout().lock();
        queue!(out, MoveTo(0, 0), Clear(ClearType::All), Show)?;
        out.flush()?;
        self.started = false;
        self.prev_height = 0;
        self.cursor_above = 0;
        self.prev_lines.clear();
        Ok(())
    }
}

/// Рендерит живой блок в оффскрин-буфер (переиспользуя обычные виджеты ratatui,
/// включая рамки) и возвращает его строки + позицию курсора ввода в блоке.
fn build_dynamic(app: &App, width: u16, full_h: u16) -> (Vec<Line<'static>>, u16, u16) {
    let width = width.max(1);
    let composer = composer_height(app, width);
    // Футер прячется, когда открыта панель (палитра/подсказки/поиск/гейт): она сама
    // под композером, дублировать подсказки и отъедать строку незачем.
    let footer = if panel_active(app) { 0 } else { 1 };
    // «Воздух» только сверху блока: пустая строка между историей и блоком (работает и
    // под лоадером — он не липнет к тексту). Под инпутом отступ не нужен — футер идёт
    // сразу за нижней линейкой композера.
    let gap_top = 1u16;
    let reserved = gap_top + composer + footer;
    let room = full_h
        .saturating_sub(1) // оставить хотя бы строку под историю/скроллбэк
        .saturating_sub(reserved);
    // Верхний слот над вводом (область диалога): реплика пользователя текущего рана
    // (live_turn, ещё не в ленте) сверху, под ней «печать» ответа (reveal) или loader.
    let mut top: Vec<Line<'static>> = Vec::new();
    if let Some(turn) = &app.live_turn {
        let mut state = TranscriptRenderState::default();
        let mut turn_lines = history_line_render(turn, app.lang, width, app.theme, &mut state);
        // ведущую пустую строку из бокса убираем — воздух уже даёт gap_top
        if turn_lines.first().is_some_and(|line| line.width() == 0) {
            turn_lines.remove(0);
        }
        top.extend(turn_lines);
    }
    if let Some(reveal) = &app.reveal {
        let shown = reveal.shown_text();
        let mut state = TranscriptRenderState::default();
        top.extend(
            shown
                .split('\n')
                .flat_map(|line| history_line_render(line, app.lang, width, app.theme, &mut state)),
        );
    } else if app.running {
        // Живой токен-стрим ответа (claude): растёт по мере прихода, рисуется как
        // обычный ответ (⏺); лоадер со спиннером/активностью — под ним.
        // Прячем тело блока ```clave-ask` ещё в стриме: JSON выбора не должен
        // мелькать в ленте до того, как откроется панель (на ChatDone).
        let visible = live_answer_visible(&app.live_answer);
        if !visible.is_empty() {
            let shown = format!("⏺ {visible}");
            let mut state = TranscriptRenderState::default();
            top.extend(shown.split('\n').flat_map(|line| {
                history_line_render(line, app.lang, width, app.theme, &mut state)
            }));
        }
        // Отступ между ответом и лоадером — только когда ответ печатается. Сверху
        // блока пустую строку уже даёт gap_top, иначе отступ был бы двойным.
        if !visible.is_empty() {
            top.push(Line::from(""));
        }
        top.extend(loader_lines(app, width));
        // Воздух между лоадером и полем ввода: спиннер не липнет к инпуту.
        // Пустая строка — последняя в top, окно всегда держит хвост → она ровно
        // над композером (правка раскладки/курсора не нужна).
        top.push(Line::from(""));
    } else if let Some(d) = app.last_run_duration {
        // Ран завершён: «замороженный» лоадер остаётся над инпутом до следующего
        // ввода. Верхний воздух даёт gap_top, снизу — пустая строка над композером.
        top.push(idle_loader_line(app, d));
        top.push(Line::from(""));
    }
    let top_h = (top.len() as u16).min(room);
    // Если reveal длиннее окна — показываем хвост (низ), как стрим в терминале.
    let top_tail: Vec<Line<'static>> = top.split_off(top.len() - top_h as usize);
    let panel = panel_height(app, width, room.saturating_sub(top_h));
    let height = (gap_top + top_h + composer + footer + panel)
        .min(full_h.saturating_sub(1).max(1))
        .max(composer + footer);

    let mut terminal = match Terminal::new(TestBackend::new(width, height)) {
        Ok(terminal) => terminal,
        Err(_) => return (Vec::new(), 0, 0),
    };
    // Порядок сверху вниз: воздух → reveal|loader → поле ввода → футер → панель.
    let lines = terminal
        .draw(|frame| {
            let area = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(gap_top),
                    Constraint::Length(top_h),
                    Constraint::Length(composer),
                    Constraint::Length(footer),
                    Constraint::Length(panel),
                ])
                .split(area);
            if top_h > 0 {
                frame.render_widget(Paragraph::new(top_tail), chunks[1]);
            }
            draw_prompt_bar(frame, chunks[2], app);
            if footer > 0 {
                draw_footer(frame, chunks[3], app);
            }
            if panel > 0 {
                draw_active_panel(frame, chunks[4], app);
            }
        })
        .map(|completed| buffer_to_lines(completed.buffer))
        .unwrap_or_default();

    // Курсор ввода: композер идёт после воздуха и верхнего слота, +1 на линейку рамки.
    let (line_index, col) = input_cursor_position_wrapped(&app.input, app.cursor, width);
    let cur_row = (gap_top + top_h + 1 + line_index as u16).min(height.saturating_sub(1));
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

/// Убирает управляющие символы (ESC/CR/BEL/BS/…) из текста перед выводом в
/// терминал. Иначе ответ модели или содержимое прочитанного агентом файла могло бы
/// инжектить ANSI/OSC-последовательности (смена заголовка, OSC 52 → буфер обмена,
/// подмена UI). Цвет/стиль идут отдельно (`apply_style`), а не из контента, так что
/// собственный UI не страдает. Табы сохраняем; рамки/кириллица — не control, целы.
fn sanitize_terminal_text(text: &str) -> std::borrow::Cow<'_, str> {
    if text.chars().any(|ch| ch.is_control() && ch != '\t') {
        std::borrow::Cow::Owned(
            text.chars()
                .filter(|ch| !ch.is_control() || *ch == '\t')
                .collect(),
        )
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

fn queue_line(out: &mut impl Write, line: &Line<'static>) -> io::Result<()> {
    for span in &line.spans {
        apply_style(out, span.style)?;
        queue!(out, Print(sanitize_terminal_text(&span.content)))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_escape_and_control_keeps_text() {
        // ESC/OSC/цвет/CR/BEL — вырезаются (инъекция в терминал невозможна).
        let evil = "НАЧАЛО\u{1b}[31mКРАСНЫЙ\u{1b}]0;PWNED\u{7}\rКОНЕЦ";
        let clean = sanitize_terminal_text(evil);
        assert!(!clean.contains('\u{1b}'), "ESC должен быть убран");
        assert!(!clean.contains('\u{7}') && !clean.contains('\r'));
        assert_eq!(clean, "НАЧАЛО[31mКРАСНЫЙ]0;PWNEDКОНЕЦ");
        // Обычный текст, кириллица, рамки и табы — нетронуты (и без аллокации).
        let safe = "│ ответ\tкод ╭─╮ Ω";
        assert!(matches!(
            sanitize_terminal_text(safe),
            std::borrow::Cow::Borrowed(_)
        ));
        assert_eq!(sanitize_terminal_text(safe), safe);
    }
}
