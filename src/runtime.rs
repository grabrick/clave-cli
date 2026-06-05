use crate::prelude::*;
use crate::*;

pub(crate) fn main_entry() -> AnyResult<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_usage();
        return Ok(());
    }

    if !args.is_empty() {
        return run_engine_direct(args);
    }

    run_tui()
}

pub(crate) fn print_usage() {
    println!(
        "duel\n\nUsage:\n  duel                 Open TUI\n  duel <task...>       Run task directly through spec-duel\n  duel --help          Show help\n"
    );
}

pub(crate) fn run_engine_direct(args: Vec<String>) -> AnyResult<()> {
    let engine = engine_path().ok_or("spec-duel engine not found")?;
    let work_dir = launch_work_dir();
    let status = Command::new(&engine)
        .current_dir(work_dir)
        .args(args)
        .status()?;
    std::process::exit(status.code().unwrap_or(1));
}

pub(crate) fn run_tui() -> AnyResult<()> {
    force_color_output(true);
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

pub(crate) fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> AnyResult<()> {
    let mut app = App::new();

    loop {
        app.drain_worker_events();
        app.advance_reveal();
        app.expire_footer_notice();
        app.refresh_command_palette_state();
        app.refresh_footer_right_state();
        terminal.draw(|frame| draw(frame, &app))?;

        if app.should_quit {
            return Ok(());
        }

        if event::poll(Duration::from_millis(80))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(&mut app, key),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if let Some(command) = app.pending_external.take() {
            let label = app
                .lang
                .choose(command.label_ru, command.label_en)
                .to_string();
            let result = run_external_command(terminal, &command);
            match result {
                Ok(code) => {
                    let mode = app.mode;
                    let lang = app.lang;
                    if let Some(onboarding) = app.onboarding.as_mut() {
                        onboarding.refresh_auth();
                        let ready = auth_requirements_ready(mode, onboarding);
                        onboarding.message = if ready {
                            onboarding.step = OnboardingStep::Settings;
                            lang.choose(
                                "Авторизация готова. Проверь стартовые настройки и нажми Enter.",
                                "Authentication is ready. Review startup settings and press Enter.",
                            )
                            .to_string()
                        } else if code == 0 {
                            lang.choose(
                                "Логин завершился. Статус обновлен, но нужные аккаунты еще не все готовы.",
                                "Login finished. Status updated, but not every required account is ready yet.",
                            ).to_string()
                        } else {
                            lang.choose(
                                "Команда логина завершилась с ошибкой. Проверь текст выше и повтори.",
                                "Login command failed. Check the text above and try again.",
                            ).to_string()
                        };
                    }
                    app.push_system(format!("{label}: exit {code}"));
                }
                Err(err) => app.push_system(format!("{label}: {err}")),
            }
        }
    }
}

pub(crate) fn handle_key(app: &mut App, key: KeyEvent) {
    if app.onboarding.is_some() {
        handle_onboarding_key(app, key);
        return;
    }

    if app.effort_picker {
        handle_effort_key(app, key);
        return;
    }

    if app.settings_open {
        handle_settings_key(app, key);
        return;
    }

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    if ctrl {
        match key.code {
            KeyCode::Char('c') => app.handle_ctrl_c(),
            KeyCode::Char('j') => app.insert_newline(),
            KeyCode::Char('m') => app.submit_input(),
            KeyCode::Char('a') => app.move_line_start(),
            KeyCode::Char('e') => app.move_line_end(),
            KeyCode::Char('b') => app.move_left(),
            KeyCode::Char('f') => app.move_right(),
            KeyCode::Char('p') => app.history_prev(),
            KeyCode::Char('n') => app.history_next(),
            KeyCode::Char('u') => app.kill_before_cursor(),
            KeyCode::Char('k') => app.kill_after_cursor(),
            KeyCode::Char('w') => app.delete_word_back(),
            KeyCode::Char('d') => app.delete(),
            KeyCode::Left => app.move_word_left(),
            KeyCode::Right => app.move_word_right(),
            KeyCode::Backspace => app.delete_word_back(),
            KeyCode::Delete => app.delete_word_forward(),
            KeyCode::Home => app.cursor = 0,
            KeyCode::End => app.cursor = app.input.len(),
            _ => {}
        }
        return;
    }

    if alt {
        match key.code {
            KeyCode::Left | KeyCode::Char('b') => app.move_word_left(),
            KeyCode::Right | KeyCode::Char('f') => app.move_word_right(),
            KeyCode::Backspace => app.delete_word_back(),
            KeyCode::Delete | KeyCode::Char('d') => app.delete_word_forward(),
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Enter => app.submit_input(),
        KeyCode::Tab => app.complete_command(),
        KeyCode::Backspace => app.backspace(),
        KeyCode::Delete => app.delete(),
        KeyCode::Left => app.move_left(),
        KeyCode::Right => app.move_right(),
        KeyCode::Up => app.history_prev(),
        KeyCode::Down => app.history_next(),
        KeyCode::Home => app.move_line_start(),
        KeyCode::End => app.move_line_end(),
        KeyCode::Esc => {
            app.input.clear();
            app.cursor = 0;
            app.history_index = None;
            app.selected_suggestion = 0;
        }
        KeyCode::Char(ch) if !ch.is_control() => app.insert_char(ch),
        _ => {}
    }
}

pub(crate) fn handle_onboarding_key(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        app.handle_ctrl_c();
        return;
    }

    let Some(step) = app.onboarding.as_ref().map(|onboarding| onboarding.step) else {
        return;
    };

    match step {
        OnboardingStep::Provider => handle_onboarding_provider_key(app, key),
        OnboardingStep::Auth => handle_onboarding_auth_key(app, key),
        OnboardingStep::Settings => handle_onboarding_settings_key(app, key),
    }
}

pub(crate) fn handle_onboarding_provider_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            let index = {
                let onboarding = app.onboarding.as_mut().expect("onboarding exists");
                onboarding.provider_index = onboarding.provider_index.saturating_sub(1);
                onboarding.provider_index
            };
            app.set_mode(provider_mode(index));
        }
        KeyCode::Down => {
            let index = {
                let onboarding = app.onboarding.as_mut().expect("onboarding exists");
                onboarding.provider_index =
                    (onboarding.provider_index + 1).min(provider_count() - 1);
                onboarding.provider_index
            };
            app.set_mode(provider_mode(index));
        }
        KeyCode::Enter => {
            let provider_index = app
                .onboarding
                .as_ref()
                .map(|onboarding| onboarding.provider_index);
            if let Some(provider_index) = provider_index {
                app.set_mode(provider_mode(provider_index));
            }
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.step = OnboardingStep::Auth;
                onboarding.message = app
                    .lang
                    .choose(
                        "Проверь авторизацию CLI. Можно запустить логин прямо отсюда.",
                        "Check CLI authentication. You can run login from here.",
                    )
                    .to_string();
            }
        }
        _ => {}
    }
}

pub(crate) fn handle_onboarding_auth_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.pending_external = Some(ExternalCommand {
                program: "codex",
                args: &["login"],
                label_ru: "Codex login",
                label_en: "Codex login",
            });
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            app.pending_external = Some(ExternalCommand {
                program: "claude",
                args: &["auth", "login"],
                label_ru: "Claude auth login",
                label_en: "Claude auth login",
            });
        }
        KeyCode::Enter => {
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.step = OnboardingStep::Settings;
                onboarding.message = app
                    .lang
                    .choose(
                        "Выставь стартовые настройки. Enter сохранит конфиг.",
                        "Choose startup defaults. Enter saves the config.",
                    )
                    .to_string();
            }
        }
        KeyCode::Backspace | KeyCode::Esc => {
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.step = OnboardingStep::Provider;
            }
        }
        _ => {}
    }
}

pub(crate) fn handle_onboarding_settings_key(app: &mut App, key: KeyEvent) {
    let setting_index = app
        .onboarding
        .as_ref()
        .map(|onboarding| onboarding.setting_index)
        .unwrap_or(0);

    match key.code {
        KeyCode::Up => {
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.setting_index = onboarding.setting_index.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.setting_index = (onboarding.setting_index + 1).min(2);
            }
        }
        KeyCode::Left => adjust_onboarding_setting(app, setting_index, -1),
        KeyCode::Right => adjust_onboarding_setting(app, setting_index, 1),
        KeyCode::Char('l') | KeyCode::Char('L') => {
            app.lang = if app.lang == Language::Ru {
                Language::En
            } else {
                Language::Ru
            };
        }
        KeyCode::Enter => {
            app.onboarding = None;
            app.status = app.lang.choose("готов", "ready").to_string();
            app.save_current_config(true);
        }
        KeyCode::Backspace | KeyCode::Esc => {
            if let Some(onboarding) = app.onboarding.as_mut() {
                onboarding.step = OnboardingStep::Auth;
            }
        }
        _ => {}
    }
}

pub(crate) fn adjust_onboarding_setting(app: &mut App, setting_index: usize, direction: isize) {
    match setting_index {
        0 => {
            if direction < 0 {
                app.rounds = app.rounds.saturating_sub(1).max(1);
            } else {
                app.rounds = (app.rounds + 1).min(9);
            }
        }
        1 => {
            app.adjust_startup_effort(direction);
        }
        2 => {
            app.lang = if app.lang == Language::Ru {
                Language::En
            } else {
                Language::Ru
            };
        }
        _ => {}
    }
}

pub(crate) fn handle_effort_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => app.effort_focus = app.effort_focus.saturating_sub(1),
        KeyCode::Down => {
            app.effort_focus = (app.effort_focus + 1).min(app.effort_picker_rows() - 1);
        }
        KeyCode::Left => app.adjust_effort_focus(-1),
        KeyCode::Right => app.adjust_effort_focus(1),
        KeyCode::Enter => {
            app.effort_picker = false;
            app.effort_original = None;
            app.status = app.lang.choose("готов", "ready").to_string();
            app.save_current_config(true);
            app.push_command_result(format!("Set to {}", app.effort_summary()));
        }
        KeyCode::Esc => {
            if let Some(snapshot) = app.effort_original.take() {
                app.restore_effort_snapshot(snapshot);
            }
            app.effort_picker = false;
            app.status = app.lang.choose("готов", "ready").to_string();
            app.push_command_result("Cancelled");
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.handle_ctrl_c();
        }
        _ => {}
    }
}

pub(crate) fn handle_settings_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => app.adjust_settings_focus(-1),
        KeyCode::Down => app.adjust_settings_focus(1),
        KeyCode::Left => app.adjust_settings_value(-1),
        KeyCode::Right => app.adjust_settings_value(1),
        KeyCode::Enter => {
            app.settings_open = false;
            app.settings_original = None;
            app.status = app.lang.choose("готов", "ready").to_string();
            app.save_current_config(true);
            app.push_command_result(format!("Saved {}", app.settings_summary()));
        }
        KeyCode::Esc => {
            if let Some(snapshot) = app.settings_original.take() {
                app.restore_settings_snapshot(snapshot);
            }
            app.settings_open = false;
            app.status = app.lang.choose("готов", "ready").to_string();
            app.push_command_result("Cancelled");
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.handle_ctrl_c();
        }
        _ => {}
    }
}
