use crate::prelude::*;
use crate::*;

pub(crate) fn spawn_reader<R>(reader: R, tx: Sender<WorkerEvent>)
where
    R: io::Read + Send + 'static,
{
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let _ = tx.send(WorkerEvent::Line(line));
                }
                Err(err) => {
                    let _ = tx.send(WorkerEvent::Line(format!("read error: {err}")));
                    break;
                }
            }
        }
    });
}

pub(crate) fn prefix_chars(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}

pub(crate) fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    let words = text.split_whitespace().count();
    ((chars / 4).max(words)).max(1)
}

pub(crate) fn format_token_count(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}m", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

pub(crate) fn chat_provider(mode: Mode) -> &'static str {
    match mode {
        Mode::CodexOnly => "codex",
        Mode::ClaudeOnly | Mode::ClaudeCodex => "claude",
    }
}

pub(crate) fn provider_display(provider: &str, lang: Language) -> &'static str {
    match provider {
        "codex" => "Codex",
        "claude" => "Claude",
        _ => lang.choose("Модель", "Model"),
    }
}

pub(crate) fn chat_prompt(message: &str, context: &str, lang: Language) -> String {
    let language_hint = lang.choose(
        "Отвечай на русском, если пользователь не просит другой язык.",
        "Reply in English unless the user asks for another language.",
    );
    format!(
        concat!(
            "You are Duel Code, a direct chat assistant inside a terminal UI.\n",
            "Answer the user's message directly. Do not create a spec, do not run a planning loop, and do not modify files.\n",
            "Keep the answer concise and useful. {language_hint}\n\n",
            "Recent chat context:\n{context}\n\n",
            "User message:\n{message}"
        ),
        language_hint = language_hint,
        context = if context.trim().is_empty() { "(empty)" } else { context },
        message = message
    )
}

pub(crate) fn recent_chat_context(transcript: &[String], max_lines: usize) -> String {
    transcript
        .iter()
        .rev()
        .filter(|line| !line.starts_with("⏺ Отправляю") && !line.starts_with("⏺ Sending"))
        .take(max_lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|line| truncate_chars(line, 240))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn run_chat_provider(
    provider: &'static str,
    effort: &str,
    prompt: &str,
    work_dir: &Path,
    cancel_rx: Receiver<()>,
) -> io::Result<ChatRunResult> {
    let mut command = if provider == "claude" {
        let program = env::var("AI_ORCHESTRATOR_CLAUDE").unwrap_or_else(|_| "claude".to_string());
        let mut command = Command::new(program);
        command.args([
            "-p",
            "--effort",
            effort,
            "--no-session-persistence",
            "--tools",
            "",
            "--max-turns",
            "3",
            "--output-format",
            "text",
            prompt,
        ]);
        command
    } else {
        let program = env::var("AI_ORCHESTRATOR_CODEX").unwrap_or_else(|_| "codex".to_string());
        let mut command = Command::new(program);
        command.args([
            "exec",
            "-c",
            &format!("model_reasoning_effort=\"{}\"", effort),
            "--skip-git-repo-check",
            "--ephemeral",
            "--color",
            "never",
            "-s",
            "read-only",
            prompt,
        ]);
        command
    };

    let mut child = command
        .current_dir(work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout_handle = child.stdout.take().map(spawn_capture_reader);
    let stderr_handle = child.stderr.take().map(spawn_capture_reader);

    loop {
        if cancel_rx.try_recv().is_ok() {
            let _ = child.kill();
            let _ = child.wait();
            if let Some(handle) = stdout_handle {
                let _ = handle.join();
            }
            if let Some(handle) = stderr_handle {
                let _ = handle.join();
            }
            return Ok(ChatRunResult::Cancelled);
        }

        match child.try_wait()? {
            Some(status) => {
                let stdout = stdout_handle
                    .map(|handle| handle.join().unwrap_or_default())
                    .unwrap_or_default();
                let stderr = stderr_handle
                    .map(|handle| handle.join().unwrap_or_default())
                    .unwrap_or_default();
                return Ok(ChatRunResult::Completed(
                    status.code().unwrap_or(1),
                    stdout,
                    stderr,
                ));
            }
            None => thread::sleep(Duration::from_millis(80)),
        }
    }
}

pub(crate) fn spawn_capture_reader<R>(reader: R) -> thread::JoinHandle<String>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut text = String::new();
        let _ = reader.read_to_string(&mut text);
        text
    })
}

pub(crate) fn emit_chat_lines(tx: &Sender<WorkerEvent>, text: &str) {
    let mut first_content = true;
    for line in text.lines() {
        let rendered = if first_content && !line.trim().is_empty() {
            first_content = false;
            format!("⏺ {}", line.trim_start())
        } else {
            line.to_string()
        };
        let _ = tx.send(WorkerEvent::ChatLine(rendered));
    }
}

pub(crate) fn emit_error_lines(tx: &Sender<WorkerEvent>, text: &str) {
    let mut emitted = 0;
    for line in text.lines().filter(|line| !line.trim().is_empty()).take(40) {
        let _ = tx.send(WorkerEvent::Line(format!("⎿ {}", line)));
        emitted += 1;
    }
    if emitted == 0 {
        let _ = tx.send(WorkerEvent::Line("⎿ no stderr output".to_string()));
    }
}

pub(crate) fn engine_path() -> Option<PathBuf> {
    if let Ok(path) = env::var("DUEL_ENGINE") {
        if let Some(path) = existing_path(PathBuf::from(path)) {
            return Some(path);
        }
    }

    if let Ok(current_dir) = env::current_dir() {
        if let Some(path) = existing_path(current_dir.join("spec-duel")) {
            return Some(path);
        }
    }

    if let Ok(exe) = env::current_exe() {
        for dir in exe.ancestors().skip(1).take(4) {
            if let Some(path) = existing_path(dir.join("spec-duel")) {
                return Some(path);
            }
        }
    }

    None
}

pub(crate) fn existing_path(path: PathBuf) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    Some(path.canonicalize().unwrap_or(path))
}

pub(crate) fn engine_work_dir(engine: &Path) -> PathBuf {
    engine
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}
