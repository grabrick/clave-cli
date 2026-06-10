use crate::prelude::*;
use crate::*;
use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const SERVE_HTML: &str = include_str!("../../assets/serve.html");
const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8765;
const MAX_BODY_BYTES: u64 = 64 * 1024;
const MAX_LOG_LINES: usize = 3_000;

#[derive(Clone)]
pub(crate) struct ServeOptions {
    host: String,
    port: u16,
}

#[derive(Clone)]
struct ServeRuntimeConfig {
    mode: Mode,
    direct_provider: Provider,
    lang: Language,
    work_dir: PathBuf,
    rounds: usize,
    effort_index: usize,
    codex_effort_index: usize,
    claude_effort_index: usize,
    linked_effort_split: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ServeRunMode {
    Discussion,
    Full,
    Tandem,
}

struct ServerState {
    running: bool,
    label: String,
    mode: String,
    started_at: Option<Instant>,
    run_id: u64,
    next_line_id: usize,
    log: VecDeque<(usize, String)>,
    cancel_tx: Option<Sender<()>>,
}

type SharedState = Arc<Mutex<ServerState>>;

pub(crate) fn run_server(args: &[String]) -> AnyResult<()> {
    let options = parse_serve_args(args)?;
    let token = serve_token();
    let config = ServeRuntimeConfig::load();
    let state = Arc::new(Mutex::new(ServerState::new()));
    let address = format!("{}:{}", options.host, options.port);
    let server = Server::http(&address).map_err(|err| {
        io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            format!("failed to bind {address}: {err}"),
        )
    })?;

    println!("{APP_NAME} remote is running");
    println!("URL: http://{address}/");
    println!("Token: {token}");
    println!("Working directory: {}", config.work_dir.display());
    if options.host != DEFAULT_HOST {
        eprintln!(
            "WARNING: remote control is bound to {}. Full and Tandem can execute code; use Tailscale or another trusted private network.",
            options.host
        );
    }

    for request in server.incoming_requests() {
        handle_request(request, &state, &token, &config);
    }

    Ok(())
}

fn parse_serve_args(args: &[String]) -> AnyResult<ServeOptions> {
    let mut host = DEFAULT_HOST.to_string();
    let mut port = DEFAULT_PORT;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => {
                println!(
                    "{APP_COMMAND} --serve\n\nUsage:\n  {APP_COMMAND} --serve [--host <ip>] [--port <port>]\n\nDefaults:\n  --host {DEFAULT_HOST}\n  --port {DEFAULT_PORT}\n"
                );
                std::process::exit(0);
            }
            "--host" => {
                index += 1;
                host = args
                    .get(index)
                    .ok_or_else(|| "--host requires a value".to_string())?
                    .to_string();
            }
            "--port" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--port requires a value".to_string())?;
                port = value
                    .parse::<u16>()
                    .map_err(|_| format!("invalid --port value: {value}"))?;
            }
            "--bind" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--bind requires host:port".to_string())?;
                let (bind_host, bind_port) = value
                    .rsplit_once(':')
                    .ok_or_else(|| "--bind requires host:port".to_string())?;
                host = bind_host.to_string();
                port = bind_port
                    .parse::<u16>()
                    .map_err(|_| format!("invalid --bind port: {bind_port}"))?;
            }
            other => return Err(format!("unknown --serve option: {other}").into()),
        }
        index += 1;
    }

    Ok(ServeOptions { host, port })
}

fn serve_token() -> String {
    env::var("CLAVE_SERVE_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(random_token)
}

/// Случайный токен из /dev/urandom (192 бита hex). Прежний `pid+время` был
/// предсказуем и перебираем — для токена, открывающего исполнение кода, это плохо.
fn random_token() -> String {
    let mut buf = [0u8; 24];
    if fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut buf))
        .is_ok()
    {
        return buf.iter().map(|byte| format!("{byte:02x}")).collect();
    }
    // Фолбэк (на поддерживаемых ОС /dev/urandom есть всегда) — чтобы не паниковать.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("clave-{}-{now:x}", std::process::id())
}

/// Сравнение токена за постоянное время: без раннего выхода на первом отличии
/// байтов (защита от тайминг-атаки). Длина токена не секрет — её сверяем сразу.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

impl ServeRuntimeConfig {
    fn load() -> Self {
        let mut config = load_config(&config_path());
        config.effort_index = normalize_common_effort_index(config.effort_index);
        config.codex_effort_index =
            normalize_provider_effort_index("codex", config.codex_effort_index);
        config.claude_effort_index =
            normalize_provider_effort_index("claude", config.claude_effort_index);
        let work_dir = resolve_work_dir(&config.work_dir, &launch_work_dir());

        Self {
            mode: config.mode,
            direct_provider: config.direct_provider,
            lang: config.lang,
            work_dir,
            rounds: config.rounds,
            effort_index: config.effort_index,
            codex_effort_index: config.codex_effort_index,
            claude_effort_index: config.claude_effort_index,
            linked_effort_split: config.linked_effort_split,
        }
    }

    fn provider_effort(&self, provider: &str) -> &'static str {
        if matches!(self.mode, Mode::ClaudeCodex | Mode::CodexClaude) && !self.linked_effort_split {
            return effort_label(self.effort_index);
        }

        match provider {
            "claude" => effort_label(self.claude_effort_index),
            "codex" => effort_label(self.codex_effort_index),
            _ => effort_label(self.effort_index),
        }
    }

    fn effort_summary(&self) -> String {
        match self.mode {
            Mode::CodexOnly => format!("codex {}", effort_label(self.codex_effort_index)),
            Mode::ClaudeOnly => format!("claude {}", effort_label(self.claude_effort_index)),
            Mode::ClaudeCodex | Mode::CodexClaude if self.linked_effort_split => format!(
                "claude {} · codex {}",
                effort_label(self.claude_effort_index),
                effort_label(self.codex_effort_index)
            ),
            Mode::ClaudeCodex | Mode::CodexClaude => {
                format!("shared {}", effort_label(self.effort_index))
            }
        }
    }
}

impl ServeRunMode {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "discussion" | "chat" => Some(Self::Discussion),
            "full" | "fullaccess" | "full-access" => Some(Self::Full),
            "tandem" => Some(Self::Tandem),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Discussion => "discussion",
            Self::Full => "full",
            Self::Tandem => "tandem",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Discussion => "Discussion",
            Self::Full => "Full",
            Self::Tandem => "Tandem",
        }
    }

    fn chat_mode(self) -> ChatMode {
        match self {
            Self::Discussion => ChatMode::Discussion,
            Self::Full => ChatMode::FullAccess,
            Self::Tandem => ChatMode::Tandem,
        }
    }
}

impl ServerState {
    fn new() -> Self {
        Self {
            running: false,
            label: String::new(),
            mode: "idle".to_string(),
            started_at: None,
            run_id: 0,
            next_line_id: 0,
            log: VecDeque::new(),
            cancel_tx: None,
        }
    }

    fn push_log(&mut self, line: impl Into<String>) {
        let id = self.next_line_id;
        self.next_line_id = self.next_line_id.saturating_add(1);
        self.log.push_back((id, line.into()));
        while self.log.len() > MAX_LOG_LINES {
            self.log.pop_front();
        }
    }

    fn begin_run(&mut self, mode: ServeRunMode, label: String, cancel_tx: Sender<()>) -> u64 {
        self.running = true;
        self.label = label;
        self.mode = mode.as_str().to_string();
        self.started_at = Some(Instant::now());
        self.run_id = self.run_id.saturating_add(1);
        self.next_line_id = 0;
        self.log.clear();
        self.cancel_tx = Some(cancel_tx);
        self.run_id
    }

    fn finish_run(&mut self) {
        self.running = false;
        self.started_at = None;
        self.cancel_tx = None;
    }
}

fn handle_request(request: Request, state: &SharedState, token: &str, config: &ServeRuntimeConfig) {
    let method = request.method().clone();
    let url = request.url().to_string();
    let (path, query) = split_url(&url);

    match (method, path.as_str()) {
        (Method::Get, "/") => respond_html(request),
        _ if !authorized(&request, query, token) => respond_json(
            request,
            401,
            serde_json::json!({"error":"unauthorized","message":"invalid token"}),
        ),
        (Method::Get, "/status") => respond_status(request, state, config),
        (Method::Get, "/log") => respond_log(request, state, query),
        (Method::Post, "/run") => respond_run(request, state, config),
        (Method::Post, "/cancel") => respond_cancel(request, state),
        _ => respond_json(
            request,
            404,
            serde_json::json!({"error":"not_found","message":"unknown endpoint"}),
        ),
    }
}

fn split_url(url: &str) -> (String, &str) {
    match url.split_once('?') {
        Some((path, query)) => (path.to_string(), query),
        None => (url.to_string(), ""),
    }
}

fn authorized(request: &Request, query: &str, token: &str) -> bool {
    if query_value(query, "token")
        .as_deref()
        .is_some_and(|value| constant_time_eq(value, token))
    {
        return true;
    }

    request.headers().iter().any(|header| {
        header.field.equiv("Authorization")
            && header
                .value
                .as_str()
                .strip_prefix("Bearer ")
                .is_some_and(|value| constant_time_eq(value, token))
    })
}

fn query_value(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|part| {
        let (name, value) = part.split_once('=')?;
        (name == key).then(|| value.to_string())
    })
}

fn respond_html(request: Request) {
    let _ = request.respond(
        Response::from_string(SERVE_HTML)
            .with_status_code(StatusCode(200))
            .with_header(header("Content-Type", "text/html; charset=utf-8")),
    );
}

fn respond_status(request: Request, state: &SharedState, config: &ServeRuntimeConfig) {
    let state = state.lock().expect("server state poisoned");
    let elapsed = state
        .started_at
        .map(|started| started.elapsed().as_secs())
        .unwrap_or(0);
    respond_json(
        request,
        200,
        serde_json::json!({
            "running": state.running,
            "mode": state.mode,
            "label": state.label,
            "elapsed": elapsed,
            "run_id": state.run_id,
            "line_count": state.next_line_id,
            "cwd": config.work_dir.display().to_string(),
            "pairing": config.mode.as_str(),
            "effort": config.effort_summary(),
        }),
    );
}

fn respond_log(request: Request, state: &SharedState, query: &str) {
    let since = query_value(query, "since")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let state = state.lock().expect("server state poisoned");
    let lines = state
        .log
        .iter()
        .filter(|(id, _)| *id >= since)
        .map(|(_, line)| line)
        .collect::<Vec<_>>();
    respond_json(
        request,
        200,
        serde_json::json!({
            "lines": lines,
            "next": state.next_line_id,
            "running": state.running,
            "run_id": state.run_id,
        }),
    );
}

fn respond_run(mut request: Request, state: &SharedState, config: &ServeRuntimeConfig) {
    let body = match read_body(&mut request) {
        Ok(body) => body,
        Err(message) => {
            respond_json(
                request,
                400,
                serde_json::json!({"error":"bad_request","message":message}),
            );
            return;
        }
    };
    let payload = match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(payload) => payload,
        Err(err) => {
            respond_json(
                request,
                400,
                serde_json::json!({"error":"bad_json","message":err.to_string()}),
            );
            return;
        }
    };
    let task = payload
        .get("task")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .unwrap_or("");
    if task.is_empty() {
        respond_json(
            request,
            400,
            serde_json::json!({"error":"empty_task","message":"task is required"}),
        );
        return;
    }
    let run_mode = payload
        .get("mode")
        .and_then(|value| value.as_str())
        .and_then(ServeRunMode::parse)
        .unwrap_or(ServeRunMode::Discussion);

    let (cancel_tx, cancel_rx) = mpsc::channel();
    let (worker_tx, worker_rx) = mpsc::channel();
    let run_id = {
        let mut state = state.lock().expect("server state poisoned");
        if state.running {
            respond_json(
                request,
                409,
                serde_json::json!({"error":"running","message":"run already active"}),
            );
            return;
        }
        let label = format!("{} · {}", run_mode.label(), config.direct_provider.title());
        let run_id = state.begin_run(run_mode, label, cancel_tx);
        state.push_log(format!("╭ Ты · {} ─", run_mode.label()));
        state.push_log(task.to_string());
        state.push_log("╰────────────────────".to_string());
        state.push_log(format!(
            "⏺ Запускаю {} · cwd {} · effort {}",
            run_mode.label(),
            config.work_dir.display(),
            config.effort_summary()
        ));
        run_id
    };

    spawn_log_drain(state.clone(), worker_rx, run_id);
    spawn_remote_run(
        state.clone(),
        worker_tx,
        cancel_rx,
        config.clone(),
        run_mode,
        task.to_string(),
    );

    respond_json(
        request,
        202,
        serde_json::json!({"ok":true,"run_id":run_id,"mode":run_mode.as_str()}),
    );
}

fn respond_cancel(request: Request, state: &SharedState) {
    let cancel = {
        let mut state = state.lock().expect("server state poisoned");
        if !state.running {
            None
        } else {
            state.push_log("⏹ Отмена запрошена с телефона.");
            state.cancel_tx.take()
        }
    };

    if let Some(cancel) = cancel {
        let _ = cancel.send(());
        respond_json(request, 202, serde_json::json!({"ok":true}));
    } else {
        respond_json(
            request,
            409,
            serde_json::json!({"error":"idle","message":"no active run"}),
        );
    }
}

fn read_body(request: &mut Request) -> Result<String, String> {
    let mut body = String::new();
    let mut reader = request.as_reader().take(MAX_BODY_BYTES + 1);
    reader
        .read_to_string(&mut body)
        .map_err(|err| err.to_string())?;
    if body.len() as u64 > MAX_BODY_BYTES {
        return Err("request body is too large".to_string());
    }
    Ok(body)
}

fn spawn_remote_run(
    state: SharedState,
    tx: Sender<WorkerEvent>,
    cancel_rx: Receiver<()>,
    config: ServeRuntimeConfig,
    mode: ServeRunMode,
    task: String,
) {
    thread::spawn(move || match mode {
        ServeRunMode::Discussion | ServeRunMode::Full => {
            run_remote_chat(tx, cancel_rx, config, mode, task)
        }
        ServeRunMode::Tandem => run_remote_tandem(tx, cancel_rx, config, task),
    });

    let _ = state;
}

fn run_remote_chat(
    tx: Sender<WorkerEvent>,
    cancel_rx: Receiver<()>,
    config: ServeRuntimeConfig,
    mode: ServeRunMode,
    task: String,
) {
    let provider = config.direct_provider.as_str();
    let effort = config.provider_effort(provider).to_string();
    let chat_mode = mode.chat_mode();
    let prompt = chat_prompt(&task, "", config.lang, chat_mode);
    let result = run_chat_provider(
        provider,
        &effort,
        &prompt,
        &config.work_dir,
        cancel_rx,
        tx.clone(),
        config.lang,
        RunAccess::Chat(chat_mode),
    );

    match result {
        Ok(ChatRunResult::Completed(code, stdout, stderr, usage)) => {
            if !stdout.trim().is_empty() {
                emit_chat_lines(&tx, stdout.trim());
            } else if code != 0 {
                emit_error_lines(&tx, stderr.trim());
            }
            let _ = tx.send(WorkerEvent::ChatDone(provider, code, usage));
        }
        Ok(ChatRunResult::Cancelled) => {
            let _ = tx.send(WorkerEvent::Cancelled);
        }
        Err(err) => {
            let _ = tx.send(WorkerEvent::Failed(format!(
                "{}: {}",
                provider_display(provider, config.lang),
                err
            )));
        }
    }
}

fn run_remote_tandem(
    tx: Sender<WorkerEvent>,
    cancel_rx: Receiver<()>,
    config: ServeRuntimeConfig,
    task: String,
) {
    let executor = config.mode.architect_provider().as_str();
    let critic = config.mode.reviewer_provider().as_str();
    let executor_effort = config.provider_effort(executor).to_string();
    let critic_effort = config.provider_effort(critic).to_string();
    let result = run_tandem(
        executor,
        critic,
        &executor_effort,
        &critic_effort,
        &task,
        config.rounds,
        &config.work_dir,
        cancel_rx,
        tx.clone(),
        config.lang,
    );

    match result {
        Ok(TandemResult::Completed(code, usage)) => {
            let _ = tx.send(WorkerEvent::ChatDone(executor, code, usage));
        }
        Ok(TandemResult::Cancelled) => {
            let _ = tx.send(WorkerEvent::Cancelled);
        }
        Err(err) => {
            let _ = tx.send(WorkerEvent::Failed(format!(
                "{}: {}",
                config.lang.choose("Тандем", "Tandem"),
                err
            )));
        }
    }
}

fn spawn_log_drain(state: SharedState, rx: Receiver<WorkerEvent>, run_id: u64) {
    thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            let mut state = state.lock().expect("server state poisoned");
            if state.run_id != run_id {
                continue;
            }

            match event {
                WorkerEvent::Line(line) | WorkerEvent::ChatLine(line) => state.push_log(line),
                // Токен-стрим веб-ремоут не показывает — финальный текст придёт ChatLine'ом.
                WorkerEvent::StreamDelta(_) => {}
                WorkerEvent::ReasoningDelta(_) => {}
                WorkerEvent::Activity(line) => state.push_log(format!("⎿ {line}")),
                WorkerEvent::Done(code) => {
                    state.push_log(format!("⏺ Clave завершился с кодом {code}."));
                    state.finish_run();
                    break;
                }
                WorkerEvent::ChatDone(provider, code, usage) => {
                    if let Some(usage) = usage {
                        state.push_log(format!("⏺ usage {}", format_usage(&usage)));
                    }
                    if code == 0 {
                        state.push_log(format!(
                            "⏺ Готово · {}",
                            provider_display(provider, Language::Ru)
                        ));
                    } else {
                        state.push_log(format!(
                            "⏺ {} завершился с кодом {code}.",
                            provider_display(provider, Language::Ru)
                        ));
                    }
                    state.finish_run();
                    break;
                }
                WorkerEvent::PlanReady(_, _, code, usage) => {
                    if let Some(usage) = usage {
                        state.push_log(format!("⏺ usage {}", format_usage(&usage)));
                    }
                    state.push_log(format!("⏺ План завершился с кодом {code}."));
                    state.finish_run();
                    break;
                }
                WorkerEvent::Cancelled => {
                    state.push_log("⏹ Выполнение остановлено.".to_string());
                    state.finish_run();
                    break;
                }
                WorkerEvent::Failed(message) => {
                    state.push_log(format!("⏺ Ошибка: {message}"));
                    state.finish_run();
                    break;
                }
                WorkerEvent::AuthMissing(provider) => {
                    state.push_log(format!(
                        "⏺ {} не залогинен.",
                        provider_display(provider, Language::Ru)
                    ));
                    state.finish_run();
                    break;
                }
            }
        }
    });
}

fn format_usage(usage: &RunUsage) -> String {
    let total = usage
        .input
        .saturating_add(usage.output)
        .saturating_add(usage.cache_read)
        .saturating_add(usage.cache_creation);
    format!(
        "{} tokens · in {} · out {} · ${:.4}",
        format_token_count(total as usize),
        format_token_count(usage.input as usize),
        format_token_count(usage.output as usize),
        usage.cost_usd
    )
}

fn respond_json(request: Request, status: u16, value: serde_json::Value) {
    let _ = request.respond(
        Response::from_string(value.to_string())
            .with_status_code(StatusCode(status))
            .with_header(header("Content-Type", "application/json; charset=utf-8"))
            .with_header(header("Cache-Control", "no-store")),
    );
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid static header")
}

#[allow(dead_code)]
fn response_from_string(status: u16, body: String) -> Response<Cursor<Vec<u8>>> {
    Response::from_string(body).with_status_code(StatusCode(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_matches_only_identical() {
        assert!(constant_time_eq("abc123", "abc123"));
        assert!(!constant_time_eq("abc123", "abc124"));
        assert!(!constant_time_eq("abc", "abc123")); // разная длина
        assert!(!constant_time_eq("", "x"));
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn random_token_is_long_and_unpredictable() {
        let a = random_token();
        let b = random_token();
        // 24 байта → 48 hex-символов; и два токена не совпадают (не из pid+время).
        assert_eq!(a.len(), 48, "ожидаем 192-битный hex-токен");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b, "токены должны быть случайными, а не предсказуемыми");
    }
}
