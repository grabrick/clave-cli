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

pub(crate) fn provider_display(provider: &str, lang: Language) -> &'static str {
    match provider {
        "codex" => "Codex",
        "claude" => "Claude",
        _ => lang.choose("Модель", "Model"),
    }
}

pub(crate) fn chat_prompt(message: &str, context: &str, lang: Language, mode: ChatMode) -> String {
    let language_hint = lang.choose(
        "Отвечай на русском, если пользователь не просит другой язык.",
        "Reply in English unless the user asks for another language.",
    );
    let mode_hint = mode.prompt_hint(lang);
    format!(
        "You are {APP_NAME}, an AI assistant inside a terminal UI.\n\
         {mode_hint}\n\
         Keep your final answer concise and useful. {language_hint}\n\n\
         Recent chat context:\n{context}\n\n\
         User message:\n{message}",
        mode_hint = mode_hint,
        language_hint = language_hint,
        context = if context.trim().is_empty() {
            "(empty)"
        } else {
            context
        },
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

/// Аргументы запуска `claude` для прямого чата. Вынесено отдельно ради теста:
/// `--strict-mcp-config` гарантирует, что доступны РОВНО инструменты из
/// `mode.claude_tools()` — без MCP-серверов из глобального конфига пользователя
/// (иначе `--tools ""` не отключает MCP, и в Discussion протекали бы внешние
/// инструменты, а `needs-auth`-сервер мог бы зависнуть в headless `-p`).
pub(crate) fn claude_chat_args<'a>(
    effort: &'a str,
    mode: ChatMode,
    prompt: &'a str,
) -> Vec<&'a str> {
    vec![
        "-p",
        "--effort",
        effort,
        "--no-session-persistence",
        "--strict-mcp-config",
        "--tools",
        mode.claude_tools(),
        "--permission-mode",
        mode.claude_permission(),
        "--max-turns",
        "20",
        "--output-format",
        "stream-json",
        "--verbose",
        prompt,
    ]
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_chat_provider(
    provider: &'static str,
    effort: &str,
    prompt: &str,
    work_dir: &Path,
    cancel_rx: Receiver<()>,
    tx: Sender<WorkerEvent>,
    lang: Language,
    mode: ChatMode,
) -> io::Result<ChatRunResult> {
    let codex_out_file = env::temp_dir().join(format!(
        "clave-codex-{}-{}.txt",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let mut command = if provider == "claude" {
        let program = env::var("CLAVE_CLAUDE")
            .or_else(|_| env::var("AI_ORCHESTRATOR_CLAUDE"))
            .unwrap_or_else(|_| "claude".to_string());
        let mut command = Command::new(program);
        command.args(claude_chat_args(effort, mode, prompt));
        command
    } else {
        let program = env::var("CLAVE_CODEX")
            .or_else(|_| env::var("AI_ORCHESTRATOR_CODEX"))
            .unwrap_or_else(|_| "codex".to_string());
        let mut command = Command::new(program);
        let codex_out = codex_out_file.to_string_lossy().into_owned();
        command.args([
            "exec",
            "--json",
            "-o",
            &codex_out,
            "-c",
            &format!("model_reasoning_effort=\"{}\"", effort),
            "--skip-git-repo-check",
            "--ephemeral",
            "--color",
            "never",
            "-s",
            mode.codex_sandbox(),
            prompt,
        ]);
        command
    };

    let mut child = command
        .current_dir(work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout_handle = child.stdout.take().map(|out| {
        if provider == "claude" {
            spawn_claude_activity_reader(out, tx.clone(), lang)
        } else {
            spawn_codex_activity_reader(out, tx.clone(), lang)
        }
    });
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

                let (text, usage, is_error) = if provider == "claude" {
                    let parsed = parse_claude_response(&stdout);
                    (parsed.text, parsed.usage, parsed.is_error)
                } else {
                    let text = fs::read_to_string(&codex_out_file).unwrap_or_default();
                    let usage = parse_codex_usage(&stdout);
                    let _ = fs::remove_file(&codex_out_file);
                    (text, usage, false)
                };

                let mut code = status.code().unwrap_or(1);
                if is_error && code == 0 {
                    code = 1;
                }
                return Ok(ChatRunResult::Completed(code, text, stderr, usage));
            }
            None => thread::sleep(Duration::from_millis(80)),
        }
    }
}

pub(crate) struct ChatResponse {
    pub(crate) text: String,
    pub(crate) usage: Option<RunUsage>,
    pub(crate) is_error: bool,
}

/// Разобрать ответ `claude -p --output-format json`. При невалидном JSON —
/// fallback: весь stdout как текст, usage отсутствует.
pub(crate) fn parse_claude_response(stdout: &str) -> ChatResponse {
    let trimmed = stdout.trim();
    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(value) => {
            let text = value
                .get("result")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let is_error = value
                .get("is_error")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let usage = value.get("usage").map(|u| RunUsage {
                input: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                output: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                cache_read: u
                    .get("cache_read_input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                cache_creation: u
                    .get("cache_creation_input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                cost_usd: value
                    .get("total_cost_usd")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
            });
            ChatResponse {
                text,
                usage,
                is_error,
            }
        }
        Err(_) => ChatResponse {
            text: trimmed.to_string(),
            usage: None,
            is_error: false,
        },
    }
}

/// Рекурсивно ищем объект с токенами (имена полей различаются между версиями codex).
fn find_token_usage(value: &serde_json::Value) -> Option<RunUsage> {
    let input = value
        .get("input_tokens")
        .or_else(|| value.get("prompt_tokens"))
        .and_then(|v| v.as_u64());
    let output = value
        .get("output_tokens")
        .or_else(|| value.get("completion_tokens"))
        .and_then(|v| v.as_u64());
    if let (Some(input), Some(output)) = (input, output) {
        let cache_read = value
            .get("cached_input_tokens")
            .or_else(|| value.get("cache_read_input_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        return Some(RunUsage {
            input,
            output,
            cache_read,
            cache_creation: 0,
            cost_usd: 0.0,
        });
    }
    match value {
        serde_json::Value::Object(map) => map.values().find_map(find_token_usage),
        serde_json::Value::Array(items) => items.iter().find_map(find_token_usage),
        _ => None,
    }
}

/// Разобрать JSONL событий `codex exec --json`, вернуть последний найденный usage.
/// codex не сообщает стоимость, поэтому cost_usd = 0.0.
pub(crate) fn parse_codex_usage(jsonl: &str) -> Option<RunUsage> {
    let mut last = None;
    for line in jsonl.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(usage) = find_token_usage(&value) {
                last = Some(usage);
            }
        }
    }
    last
}

fn codex_command_start(value: &serde_json::Value) -> Option<String> {
    if value.get("type")?.as_str()? != "item.started" {
        return None;
    }
    let item = value.get("item")?;
    if item.get("type")?.as_str()? != "command_execution" {
        return None;
    }
    item.get("command")?.as_str().map(String::from)
}

fn codex_path_token(command: &str) -> Option<String> {
    command
        .split_whitespace()
        .rev()
        .map(|token| token.trim_matches(|c| c == '"' || c == '\''))
        .find(|token| token.contains('/') || token.contains('.'))
        .map(String::from)
}

/// Превратить shell-команду codex в короткую человекочитаемую активность для лоадера.
pub(crate) fn summarize_codex_command(command: &str, lang: Language) -> String {
    let inner = command
        .split_once("-lc")
        .map(|(_, rest)| rest.trim().trim_matches('"').trim().to_string())
        .unwrap_or_else(|| command.to_string());
    let first = inner.split_whitespace().next().unwrap_or("").to_lowercase();

    if matches!(
        first.as_str(),
        "sed" | "cat" | "head" | "tail" | "less" | "bat" | "more"
    ) {
        return match codex_path_token(&inner) {
            Some(file) => format!("{} {}", lang.choose("Читаю", "Reading"), file),
            None => lang.choose("Читаю файл", "Reading file").to_string(),
        };
    }
    if matches!(first.as_str(), "grep" | "rg" | "ag" | "ack") {
        return lang.choose("Ищу по коду", "Searching code").to_string();
    }
    if matches!(first.as_str(), "ls" | "find" | "fd" | "tree") {
        return lang
            .choose("Просматриваю файлы", "Listing files")
            .to_string();
    }
    format!("⚙ {}", truncate_chars(&inner, 60))
}

/// Потоково читает JSONL codex: эмитит активность (command_execution) в лоадер
/// и возвращает весь stdout (для разбора usage в конце).
pub(crate) fn spawn_codex_activity_reader(
    reader: impl Read + Send + 'static,
    tx: Sender<WorkerEvent>,
    lang: Language,
) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        let mut full = String::new();
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(command) = codex_command_start(&value) {
                    let _ = tx.send(WorkerEvent::Activity(summarize_codex_command(
                        &command, lang,
                    )));
                }
            }
            full.push_str(&line);
            full.push('\n');
        }
        full
    })
}

fn short_path(path: &str) -> String {
    let tail: Vec<&str> = path.rsplit('/').take(2).collect();
    tail.into_iter().rev().collect::<Vec<_>>().join("/")
}

/// Превратить claude tool_use в короткую человекочитаемую активность для лоадера.
fn summarize_claude_tool(item: &serde_json::Value, lang: Language) -> Option<String> {
    let name = item.get("name")?.as_str()?;
    let input = item.get("input");
    let path = input
        .and_then(|i| i.get("file_path"))
        .and_then(|v| v.as_str())
        .map(short_path);
    let command = input
        .and_then(|i| i.get("command"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let summary = match name {
        "Read" | "NotebookRead" => {
            format!(
                "{} {}",
                lang.choose("Читаю", "Reading"),
                path.unwrap_or_default()
            )
        }
        "Edit" | "MultiEdit" | "NotebookEdit" => {
            format!(
                "{} {}",
                lang.choose("Правлю", "Editing"),
                path.unwrap_or_default()
            )
        }
        "Write" => format!(
            "{} {}",
            lang.choose("Создаю", "Writing"),
            path.unwrap_or_default()
        ),
        "Bash" => format!(
            "{} {}",
            lang.choose("Выполняю", "Running"),
            truncate_chars(command, 50)
        ),
        "Grep" => lang.choose("Ищу по коду", "Searching code").to_string(),
        "Glob" => lang
            .choose("Просматриваю файлы", "Listing files")
            .to_string(),
        other => format!("⚙ {other}"),
    };
    Some(summary)
}

/// Потоково читает claude stream-json: эмитит активность (tool_use) в лоадер
/// и возвращает финальное result-событие (для разбора текста и usage).
pub(crate) fn spawn_claude_activity_reader(
    reader: impl Read + Send + 'static,
    tx: Sender<WorkerEvent>,
    lang: Language,
) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        let mut result_line = String::new();
        for line in reader.lines().map_while(Result::ok) {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            match value.get("type").and_then(|v| v.as_str()) {
                Some("assistant") => {
                    if let Some(content) = value
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_array())
                    {
                        for item in content {
                            if item.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                                if let Some(activity) = summarize_claude_tool(item, lang) {
                                    let _ = tx.send(WorkerEvent::Activity(activity));
                                }
                            }
                        }
                    }
                }
                Some("result") => result_line = line.clone(),
                _ => {}
            }
        }
        result_line
    })
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
    if let Ok(path) = env::var("CLAVE_ENGINE") {
        if let Some(path) = existing_path(PathBuf::from(path)) {
            return Some(path);
        }
    }

    if let Ok(path) = env::var("DUEL_ENGINE") {
        if let Some(path) = existing_path(PathBuf::from(path)) {
            return Some(path);
        }
    }

    if let Ok(current_dir) = env::current_dir() {
        if let Some(path) = existing_path(current_dir.join(ENGINE_NAME)) {
            return Some(path);
        }
        if let Some(path) = existing_path(current_dir.join(LEGACY_ENGINE_NAME)) {
            return Some(path);
        }
    }

    if let Ok(exe) = env::current_exe() {
        for dir in exe.ancestors().skip(1).take(4) {
            if let Some(path) = existing_path(dir.join(ENGINE_NAME)) {
                return Some(path);
            }
            if let Some(path) = existing_path(dir.join(LEGACY_ENGINE_NAME)) {
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

pub(crate) fn launch_work_dir() -> PathBuf {
    env::var("CLAVE_LAUNCH_CWD")
        .or_else(|_| env::var("DUEL_LAUNCH_CWD"))
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .and_then(existing_path)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub(crate) fn resolve_work_dir(configured: &str, base_dir: &Path) -> PathBuf {
    let configured = configured.trim();
    if configured.is_empty() || configured == "." {
        return base_dir.to_path_buf();
    }

    let path = PathBuf::from(configured);
    let resolved = if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    };

    if resolved.is_dir() {
        resolved.canonicalize().unwrap_or(resolved)
    } else {
        base_dir.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_dot_to_launch_directory() {
        let base = env::current_dir().expect("test cwd exists");
        assert_eq!(resolve_work_dir(".", &base), base);
    }

    #[test]
    fn resolves_relative_directory_from_launch_directory() {
        let base = env::current_dir().expect("test cwd exists");
        let expected = base.join("src").canonicalize().expect("src dir exists");
        assert_eq!(resolve_work_dir("src", &base), expected);
    }

    #[test]
    fn parses_claude_json_with_usage() {
        let raw = r#"{"type":"result","is_error":false,"result":"Привет!","total_cost_usd":0.0123,"usage":{"input_tokens":120,"output_tokens":40,"cache_read_input_tokens":5,"cache_creation_input_tokens":9}}"#;
        let parsed = parse_claude_response(raw);
        assert_eq!(parsed.text, "Привет!");
        assert!(!parsed.is_error);
        let usage = parsed.usage.expect("usage present");
        assert_eq!(usage.input, 120);
        assert_eq!(usage.output, 40);
        assert_eq!(usage.cache_read, 5);
        assert_eq!(usage.cache_creation, 9);
        assert!((usage.cost_usd - 0.0123).abs() < 1e-9);
    }

    #[test]
    fn claude_parser_falls_back_on_non_json() {
        let parsed = parse_claude_response("просто текст без json");
        assert_eq!(parsed.text, "просто текст без json");
        assert!(parsed.usage.is_none());
    }

    #[test]
    fn parses_codex_usage_from_jsonl() {
        let jsonl = "{\"type\":\"item\",\"text\":\"hi\"}\n{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":200,\"output_tokens\":60,\"cached_input_tokens\":10}}\n";
        let usage = parse_codex_usage(jsonl).expect("usage found");
        assert_eq!(usage.input, 200);
        assert_eq!(usage.output, 60);
        assert_eq!(usage.cache_read, 10);
        assert_eq!(usage.cost_usd, 0.0);
    }

    #[test]
    fn codex_usage_none_when_absent() {
        let jsonl = "{\"type\":\"item\",\"text\":\"hi\"}\n";
        assert!(parse_codex_usage(jsonl).is_none());
    }

    #[test]
    fn summarizes_codex_read_command() {
        let cmd = "/bin/zsh -lc \"sed -n '1,240p' src/model/overlay.rs\"";
        assert_eq!(
            summarize_codex_command(cmd, Language::En),
            "Reading src/model/overlay.rs"
        );
        let grep = "/bin/zsh -lc \"grep -rn Overlay src\"";
        assert_eq!(
            summarize_codex_command(grep, Language::En),
            "Searching code"
        );
    }

    #[test]
    fn claude_chat_args_are_strict_and_mode_scoped() {
        // --strict-mcp-config обязателен во всех режимах: иначе MCP-инструменты
        // из глобального конфига протекают мимо --tools.
        for mode in [ChatMode::Discussion, ChatMode::Plan, ChatMode::FullAccess] {
            let args = claude_chat_args("high", mode, "hi");
            assert!(
                args.contains(&"--strict-mcp-config"),
                "strict-mcp-config missing for {mode:?}"
            );
        }

        let discussion = claude_chat_args("high", ChatMode::Discussion, "hi");
        let tools_idx = discussion
            .iter()
            .position(|a| *a == "--tools")
            .expect("--tools present");
        assert_eq!(
            discussion[tools_idx + 1],
            "",
            "Discussion must be tool-free"
        );

        let full = claude_chat_args("high", ChatMode::FullAccess, "hi");
        let full_tools = full
            .iter()
            .position(|a| *a == "--tools")
            .expect("--tools present");
        assert!(
            full[full_tools + 1].contains("Bash"),
            "Full Access must include Bash"
        );
    }

    #[test]
    fn summarizes_claude_tool_use() {
        let read = serde_json::json!({
            "type": "tool_use",
            "name": "Read",
            "input": {"file_path": "/Users/x/proj/src/model/overlay.rs"}
        });
        assert_eq!(
            summarize_claude_tool(&read, Language::En),
            Some("Reading model/overlay.rs".to_string())
        );

        let bash = serde_json::json!({
            "type": "tool_use",
            "name": "Bash",
            "input": {"command": "cargo build"}
        });
        assert_eq!(
            summarize_claude_tool(&bash, Language::En),
            Some("Running cargo build".to_string())
        );

        let write = serde_json::json!({
            "type": "tool_use",
            "name": "Write",
            "input": {"file_path": "/a/b/new.rs"}
        });
        assert_eq!(
            summarize_claude_tool(&write, Language::En),
            Some("Writing b/new.rs".to_string())
        );
    }
}
