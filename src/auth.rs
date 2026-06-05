use crate::prelude::*;
use crate::*;

pub(crate) fn auth_requirements_ready(mode: Mode, onboarding: &Onboarding) -> bool {
    (!mode.needs_codex() || onboarding.codex_authenticated)
        && (!mode.needs_claude() || onboarding.claude_authenticated)
}

pub(crate) fn provider_auth_ready(provider: Provider, onboarding: &Onboarding) -> bool {
    match provider {
        Provider::Codex => onboarding.codex_authenticated,
        Provider::Claude => onboarding.claude_authenticated,
    }
}

pub(crate) fn missing_provider_auth_text(
    provider: Provider,
    onboarding: &Onboarding,
    lang: Language,
) -> String {
    match provider {
        Provider::Codex if onboarding.codex_authenticated => {
            lang.choose("всё готово", "all ready").to_string()
        }
        Provider::Codex if onboarding.codex_installed => "Codex".to_string(),
        Provider::Codex => lang
            .choose("Codex CLI не найден", "Codex CLI missing")
            .to_string(),
        Provider::Claude if onboarding.claude_authenticated => {
            lang.choose("всё готово", "all ready").to_string()
        }
        Provider::Claude if onboarding.claude_installed => "Claude".to_string(),
        Provider::Claude => lang
            .choose("Claude CLI не найден", "Claude CLI missing")
            .to_string(),
    }
}

pub(crate) fn missing_auth_text(mode: Mode, onboarding: &Onboarding, lang: Language) -> String {
    let mut missing = Vec::new();
    if mode.needs_codex() && !onboarding.codex_authenticated {
        missing.push(if onboarding.codex_installed {
            "Codex"
        } else {
            lang.choose("Codex CLI не найден", "Codex CLI missing")
        });
    }
    if mode.needs_claude() && !onboarding.claude_authenticated {
        missing.push(if onboarding.claude_installed {
            "Claude"
        } else {
            lang.choose("Claude CLI не найден", "Claude CLI missing")
        });
    }

    if missing.is_empty() {
        lang.choose("всё готово", "all ready").to_string()
    } else {
        missing.join(" + ")
    }
}

pub(crate) fn codex_auth_probe() -> AuthProbe {
    match Command::new("codex").args(["login", "status"]).output() {
        Ok(output) => {
            let text = command_output_text(&output.stdout, &output.stderr);
            AuthProbe {
                installed: true,
                authenticated: auth_output_looks_ready(output.status.success(), &text),
                status: first_nonempty_line(&text)
                    .unwrap_or_else(|| "status unavailable".to_string()),
            }
        }
        Err(err) => AuthProbe {
            installed: false,
            authenticated: false,
            status: err.to_string(),
        },
    }
}

pub(crate) fn claude_auth_probe() -> AuthProbe {
    match Command::new("claude")
        .args(["auth", "status", "--text"])
        .output()
    {
        Ok(output) => {
            let text = command_output_text(&output.stdout, &output.stderr);
            AuthProbe {
                installed: true,
                authenticated: auth_output_looks_ready(output.status.success(), &text),
                status: first_nonempty_line(&text)
                    .unwrap_or_else(|| "status unavailable".to_string()),
            }
        }
        Err(err) => AuthProbe {
            installed: false,
            authenticated: false,
            status: err.to_string(),
        },
    }
}

pub(crate) fn auth_output_looks_ready(success: bool, text: &str) -> bool {
    if !success {
        return false;
    }

    let lower = text.to_lowercase();
    !lower.contains("not logged")
        && !lower.contains("not authenticated")
        && !lower.contains("not signed")
        && !lower.contains("login required")
        && !lower.contains("logged out")
        && !lower.contains("no credentials")
}

pub(crate) fn command_output_text(stdout: &[u8], stderr: &[u8]) -> String {
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(stdout));
    if !stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(stderr));
    }
    text
}

pub(crate) fn first_nonempty_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("WARNING:"))
        .map(ToString::to_string)
}

pub(crate) fn run_external_command(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    command: &ExternalCommand,
) -> AnyResult<i32> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!();
    println!(
        "Clave: running {} {}",
        command.program,
        command.args.join(" ")
    );
    println!();

    let result = Command::new(command.program).args(command.args).status();
    let code = match result {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            println!("Clave: failed to start command: {err}");
            1
        }
    };

    println!();
    println!("Clave: press Enter to return...");
    let mut wait = String::new();
    let _ = io::stdin().read_line(&mut wait);

    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.clear()?;

    Ok(code)
}
