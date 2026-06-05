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

pub(crate) fn plan_prompt(task: &str, context: &str, lang: Language) -> String {
    let language_hint = lang.choose(
        "Отвечай на русском, если пользователь не просит другой язык.",
        "Reply in English unless the user asks for another language.",
    );
    format!(
        "You are {APP_NAME}, an AI assistant inside a terminal UI, in PLAN MODE.\n\
         Study the working directory (read files, search) and produce a concrete, \
         step-by-step implementation plan for the task. For each step name the files \
         to touch and what changes it makes; list risks or open questions at the end.\n\
         Do NOT modify any files and do NOT run shell commands — planning only.\n\
         {language_hint}\n\n\
         Recent chat context:\n{context}\n\n\
         Task:\n{task}",
        language_hint = language_hint,
        context = if context.trim().is_empty() {
            "(empty)"
        } else {
            context
        },
        task = task,
    )
}

pub(crate) fn execute_prompt(task: &str, plan: &str, context: &str, lang: Language) -> String {
    let language_hint = lang.choose(
        "Отвечай на русском, если пользователь не просит другой язык.",
        "Reply in English unless the user asks for another language.",
    );
    format!(
        "You are {APP_NAME}, an AI assistant inside a terminal UI, executing an APPROVED plan.\n\
         Implement the task fully: read, create and edit files and run commands in the \
         working directory as needed. Follow the plan; if reality differs, adapt but stay \
         within its intent. Keep your final answer concise and useful. {language_hint}\n\n\
         Recent chat context:\n{context}\n\n\
         Task:\n{task}\n\n\
         Approved plan:\n{plan}",
        language_hint = language_hint,
        context = if context.trim().is_empty() {
            "(empty)"
        } else {
            context
        },
        task = task,
        plan = plan,
    )
}

pub(crate) fn refine_prompt(
    task: &str,
    prev_plan: &str,
    feedback: &str,
    context: &str,
    lang: Language,
) -> String {
    let language_hint = lang.choose(
        "Отвечай на русском, если пользователь не просит другой язык.",
        "Reply in English unless the user asks for another language.",
    );
    format!(
        "You are {APP_NAME}, an AI assistant inside a terminal UI, in PLAN MODE.\n\
         Revise the previous plan to address the user's feedback. Same rules: read-only — \
         Do NOT modify files or run commands; numbered steps with files to touch and \
         risks at the end. {language_hint}\n\n\
         Recent chat context:\n{context}\n\n\
         Task:\n{task}\n\n\
         Previous plan:\n{prev_plan}\n\n\
         User feedback to address:\n{feedback}",
        language_hint = language_hint,
        context = if context.trim().is_empty() {
            "(empty)"
        } else {
            context
        },
        task = task,
        prev_plan = prev_plan,
        feedback = feedback,
    )
}

/// Последняя строка с `TANDEM:` определяет сигнал: CONSENSUS (и не CONTINUE) → true.
/// Дефолт false (= CONTINUE) — безопаснее продолжить, чем ложно согласиться (P1).
pub(crate) fn parse_tandem_signal(text: &str) -> bool {
    for line in text.lines().rev() {
        let up = line.to_uppercase();
        if up.contains("TANDEM:") {
            return up.contains("CONSENSUS") && !up.contains("CONTINUE");
        }
    }
    false
}

fn tandem_lang_hint(lang: Language) -> &'static str {
    lang.choose(
        "Отвечай на русском, если пользователь не просит другой язык.",
        "Reply in English unless the user asks for another language.",
    )
}

pub(crate) fn tandem_propose_prompt(task: &str, transcript: &str, lang: Language) -> String {
    format!(
        "You are {APP_NAME}, the EXECUTOR working in a pair with a CRITIC. PLAN MODE.\n\
         Study the working directory (read files, search). Propose a concrete approach to \
         the task: which files, what changes, and why. Address the critic's prior objections \
         if any. Do NOT modify files or run commands — this is discussion. {hint}\n\n\
         Task:\n{task}\n\n\
         Tandem transcript so far:\n{transcript}",
        hint = tandem_lang_hint(lang),
        task = task,
        transcript = if transcript.trim().is_empty() {
            "(empty)"
        } else {
            transcript
        },
    )
}

pub(crate) fn tandem_challenge_prompt(task: &str, transcript: &str, lang: Language) -> String {
    format!(
        "You are {APP_NAME}, the CRITIC working in a pair with an EXECUTOR. PLAN MODE.\n\
         Study the code (read-only) and STRICTLY evaluate the executor's proposed approach: \
         gaps, risks, what is missing, better alternatives. Do NOT agree out of politeness. \
         End with EXACTLY one line: `TANDEM: CONSENSUS` only if the approach is genuinely \
         correct and complete, otherwise `TANDEM: CONTINUE` followed by concrete objections. \
         {hint}\n\n\
         Task:\n{task}\n\n\
         Tandem transcript so far:\n{transcript}",
        hint = tandem_lang_hint(lang),
        task = task,
        transcript = if transcript.trim().is_empty() {
            "(empty)"
        } else {
            transcript
        },
    )
}

pub(crate) fn tandem_execute_prompt(task: &str, transcript: &str, lang: Language) -> String {
    format!(
        "You are {APP_NAME}, the EXECUTOR. The approach below was agreed with the critic. \
         Implement the task fully in the working directory: read, create and edit files and \
         run commands as needed. If reality differs from the plan, adapt within its intent. \
         Keep your final answer concise. {hint}\n\n\
         Task:\n{task}\n\n\
         Agreed approach / transcript:\n{transcript}",
        hint = tandem_lang_hint(lang),
        task = task,
        transcript = if transcript.trim().is_empty() {
            "(empty)"
        } else {
            transcript
        },
    )
}

pub(crate) fn tandem_review_prompt(task: &str, transcript: &str, lang: Language) -> String {
    format!(
        "You are {APP_NAME}, the CRITIC. The executor applied the approach. Inspect the REAL \
         result (read the changed files). Does it match what was agreed, is it correct, any \
         bugs or omissions? End with EXACTLY one line: `TANDEM: CONSENSUS` if the result is \
         good, otherwise `TANDEM: CONTINUE` followed by what to fix. {hint}\n\n\
         Task:\n{task}\n\n\
         Tandem transcript so far:\n{transcript}",
        hint = tandem_lang_hint(lang),
        task = task,
        transcript = if transcript.trim().is_empty() {
            "(empty)"
        } else {
            transcript
        },
    )
}

pub(crate) fn tandem_fix_prompt(
    task: &str,
    transcript: &str,
    review: &str,
    lang: Language,
) -> String {
    format!(
        "You are {APP_NAME}, the EXECUTOR. The critic raised issues with the result. Fix them \
         in the working directory. Keep your final answer concise. {hint}\n\n\
         Task:\n{task}\n\n\
         Critic's review to address:\n{review}\n\n\
         Tandem transcript so far:\n{transcript}",
        hint = tandem_lang_hint(lang),
        task = task,
        review = review,
        transcript = if transcript.trim().is_empty() {
            "(empty)"
        } else {
            transcript
        },
    )
}

pub(crate) fn tandem_confirm_prompt(task: &str, transcript: &str, lang: Language) -> String {
    format!(
        "You are {APP_NAME}, the CRITIC. The executor applied fixes. Briefly verify whether \
         your issues are resolved (read the changed files). End with EXACTLY one line: \
         `TANDEM: CONSENSUS` if resolved, otherwise `TANDEM: CONTINUE` with what remains. \
         {hint}\n\n\
         Task:\n{task}\n\n\
         Tandem transcript so far:\n{transcript}",
        hint = tandem_lang_hint(lang),
        task = task,
        transcript = if transcript.trim().is_empty() {
            "(empty)"
        } else {
            transcript
        },
    )
}

/// Аргументы запуска `claude` для прямого чата. Вынесено отдельно ради теста:
/// `--strict-mcp-config` гарантирует, что доступны РОВНО инструменты из
/// `access` — без MCP-серверов из глобального конфига пользователя (иначе
/// `--tools ""` не отключает MCP, и `needs-auth`-сервер может зависнуть в `-p`).
pub(crate) fn claude_chat_args<'a>(
    effort: &'a str,
    access: RunAccess,
    prompt: &'a str,
) -> Vec<&'a str> {
    vec![
        "-p",
        "--effort",
        effort,
        "--no-session-persistence",
        "--strict-mcp-config",
        "--tools",
        access.claude_tools(),
        "--permission-mode",
        access.claude_permission(),
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
    access: RunAccess,
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
        command.args(claude_chat_args(effort, access, prompt));
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
            access.codex_sandbox(),
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

pub(crate) struct TandemStep {
    pub(crate) text: String,
    pub(crate) code: i32,
    pub(crate) usage: Option<RunUsage>,
}

pub(crate) enum TandemResult {
    Completed(i32, Option<RunUsage>),
    Cancelled,
}

/// Лента тандема, передаётся целиком в каждый промпт (P6: усечение при росте).
struct TandemTranscript {
    entries: Vec<String>,
}

impl TandemTranscript {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn push(&mut self, who: &str, phase: &str, text: &str) {
        self.entries
            .push(format!("[{who} · {phase}]\n{}", text.trim()));
    }

    fn render(&self) -> String {
        let full = self.entries.join("\n\n");
        if full.len() <= 12_000 || self.entries.len() <= 4 {
            return full;
        }
        // P6: оставляем первую запись + хвост (последние 3)
        let head = &self.entries[0];
        let tail = &self.entries[self.entries.len() - 3..];
        format!(
            "{head}\n\n…[ранние раунды усечены]…\n\n{}",
            tail.join("\n\n")
        )
    }
}

/// Один вызов провайдера для тандема. `cancel_rx` по ссылке — чтобы переиспользовать
/// на серии шагов. None = отменён в процессе. Активность инструментов стримится в `tx`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_provider_once(
    provider: &'static str,
    effort: &str,
    prompt: &str,
    work_dir: &Path,
    access: RunAccess,
    lang: Language,
    tx: &Sender<WorkerEvent>,
    cancel_rx: &Receiver<()>,
) -> io::Result<Option<TandemStep>> {
    let codex_out_file = env::temp_dir().join(format!(
        "clave-tandem-{}-{}.txt",
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
        command.args(claude_chat_args(effort, access, prompt));
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
            access.codex_sandbox(),
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
            let _ = fs::remove_file(&codex_out_file);
            return Ok(None);
        }

        match child.try_wait()? {
            Some(status) => {
                let stdout = stdout_handle
                    .map(|handle| handle.join().unwrap_or_default())
                    .unwrap_or_default();
                let _ = stderr_handle.map(|handle| handle.join().unwrap_or_default());

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
                return Ok(Some(TandemStep { text, code, usage }));
            }
            None => thread::sleep(Duration::from_millis(80)),
        }
    }
}

fn tandem_accumulate(total: &mut RunUsage, usage: &Option<RunUsage>) {
    if let Some(u) = usage {
        total.input += u.input;
        total.output += u.output;
        total.cache_read += u.cache_read;
        total.cache_creation += u.cache_creation;
        total.cost_usd += u.cost_usd;
    }
}

fn emit_tandem_step(tx: &Sender<WorkerEvent>, marker: &str, who: &str, phase: &str, text: &str) {
    let _ = tx.send(WorkerEvent::ChatLine(format!("{marker} {who} · {phase}")));
    for line in text.trim().lines() {
        let _ = tx.send(WorkerEvent::ChatLine(line.to_string()));
    }
    let _ = tx.send(WorkerEvent::ChatLine(String::new()));
}

fn tandem_notice(tx: &Sender<WorkerEvent>, text: String) {
    let _ = tx.send(WorkerEvent::Line(text));
}

fn opt_usage(total: RunUsage) -> Option<RunUsage> {
    if total == RunUsage::default() {
        None
    } else {
        Some(total)
    }
}

/// Оркестратор тандема: дебаты до консенсуса → исполнение → ревью → правка →
/// подтверждение. Серия вызовов `run_provider_once`; стрим шагов в чат.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_tandem(
    executor: &'static str,
    critic: &'static str,
    executor_effort: &str,
    critic_effort: &str,
    task: &str,
    rounds: usize,
    work_dir: &Path,
    cancel_rx: Receiver<()>,
    tx: Sender<WorkerEvent>,
    lang: Language,
) -> io::Result<TandemResult> {
    let mut transcript = TandemTranscript::new();
    let mut total = RunUsage::default();
    let executor_name = provider_display(executor, lang);
    let critic_name = provider_display(critic, lang);
    let exec_role = lang.choose("Исполнитель", "Executor");
    let crit_role = lang.choose("Критик", "Critic");

    // P5: предупреждение о возможных изменённых файлах при прерывании после исполнения.
    let dirty_notice = |tx: &Sender<WorkerEvent>| {
        tandem_notice(
            tx,
            lang.choose(
                "⚠ Файлы были изменены до прерывания — проверь рабочую директорию.",
                "⚠ Files were modified before interruption — check the working directory.",
            )
            .to_string(),
        );
    };

    // ФАЗА ДЕБАТОВ
    let mut consensus = false;
    for round in 1..=rounds.max(1) {
        let propose = tandem_propose_prompt(task, &transcript.render(), lang);
        let step = match run_provider_once(
            executor,
            executor_effort,
            &propose,
            work_dir,
            RunAccess::PlanReadonly,
            lang,
            &tx,
            &cancel_rx,
        )? {
            Some(s) => s,
            None => return Ok(TandemResult::Cancelled),
        };
        tandem_accumulate(&mut total, &step.usage);
        if step.code != 0 {
            tandem_notice(
                &tx,
                format!(
                    "{} {}",
                    executor_name,
                    lang.choose("вернул ошибку", "returned an error")
                ),
            );
            return Ok(TandemResult::Completed(step.code, opt_usage(total)));
        }
        emit_tandem_step(
            &tx,
            "🅐",
            executor_name,
            &format!("{} {round} · {}", lang.choose("раунд", "round"), exec_role),
            &step.text,
        );
        transcript.push(
            exec_role,
            &format!(
                "{} {round}",
                lang.choose("предложение, раунд", "proposal, round")
            ),
            &step.text,
        );

        let challenge = tandem_challenge_prompt(task, &transcript.render(), lang);
        let step = match run_provider_once(
            critic,
            critic_effort,
            &challenge,
            work_dir,
            RunAccess::PlanReadonly,
            lang,
            &tx,
            &cancel_rx,
        )? {
            Some(s) => s,
            None => return Ok(TandemResult::Cancelled),
        };
        tandem_accumulate(&mut total, &step.usage);
        if step.code != 0 {
            tandem_notice(
                &tx,
                format!(
                    "{} {}",
                    critic_name,
                    lang.choose("вернул ошибку", "returned an error")
                ),
            );
            return Ok(TandemResult::Completed(step.code, opt_usage(total)));
        }
        emit_tandem_step(
            &tx,
            "🅒",
            critic_name,
            &format!("{} {round} · {}", lang.choose("раунд", "round"), crit_role),
            &step.text,
        );
        transcript.push(
            crit_role,
            &format!(
                "{} {round}",
                lang.choose("критика, раунд", "critique, round")
            ),
            &step.text,
        );

        if parse_tandem_signal(&step.text) {
            consensus = true;
            break;
        }
    }
    if !consensus {
        tandem_notice(
            &tx,
            lang.choose(
                "⚠ Консенсус не достигнут за раунды — исполняю последнюю версию.",
                "⚠ No consensus within the rounds — executing the latest proposal.",
            )
            .to_string(),
        );
    }

    // ФАЗА ИСПОЛНЕНИЯ
    if cancel_rx.try_recv().is_ok() {
        return Ok(TandemResult::Cancelled);
    }
    let execute = tandem_execute_prompt(task, &transcript.render(), lang);
    let step = match run_provider_once(
        executor,
        executor_effort,
        &execute,
        work_dir,
        RunAccess::PlanExecute,
        lang,
        &tx,
        &cancel_rx,
    )? {
        Some(s) => s,
        None => {
            dirty_notice(&tx);
            return Ok(TandemResult::Cancelled);
        }
    };
    tandem_accumulate(&mut total, &step.usage);
    if step.code != 0 {
        dirty_notice(&tx);
        tandem_notice(
            &tx,
            format!(
                "{} {}",
                executor_name,
                lang.choose("вернул ошибку", "returned an error")
            ),
        );
        return Ok(TandemResult::Completed(step.code, opt_usage(total)));
    }
    emit_tandem_step(
        &tx,
        "🅐",
        executor_name,
        &format!("{} · {}", lang.choose("исполнение", "execution"), exec_role),
        &step.text,
    );
    transcript.push(
        exec_role,
        lang.choose("исполнение", "execution"),
        &step.text,
    );

    // ФАЗА РЕВЬЮ
    let review = tandem_review_prompt(task, &transcript.render(), lang);
    let step = match run_provider_once(
        critic,
        critic_effort,
        &review,
        work_dir,
        RunAccess::PlanReadonly,
        lang,
        &tx,
        &cancel_rx,
    )? {
        Some(s) => s,
        None => {
            dirty_notice(&tx);
            return Ok(TandemResult::Cancelled);
        }
    };
    tandem_accumulate(&mut total, &step.usage);
    emit_tandem_step(
        &tx,
        "🅒",
        critic_name,
        &format!("{} · {}", lang.choose("ревью", "review"), crit_role),
        &step.text,
    );
    transcript.push(crit_role, lang.choose("ревью", "review"), &step.text);
    let review_ok = step.code == 0 && parse_tandem_signal(&step.text);

    // ФИНАЛЬНАЯ ПРАВКА + ПОДТВЕРЖДЕНИЕ (P4)
    if !review_ok {
        let review_text = step.text.clone();
        if cancel_rx.try_recv().is_ok() {
            dirty_notice(&tx);
            return Ok(TandemResult::Cancelled);
        }
        let fix = tandem_fix_prompt(task, &transcript.render(), &review_text, lang);
        let step = match run_provider_once(
            executor,
            executor_effort,
            &fix,
            work_dir,
            RunAccess::PlanExecute,
            lang,
            &tx,
            &cancel_rx,
        )? {
            Some(s) => s,
            None => {
                dirty_notice(&tx);
                return Ok(TandemResult::Cancelled);
            }
        };
        tandem_accumulate(&mut total, &step.usage);
        emit_tandem_step(
            &tx,
            "🅐",
            executor_name,
            &format!(
                "{} · {}",
                lang.choose("финальная правка", "final fix"),
                exec_role
            ),
            &step.text,
        );
        transcript.push(
            exec_role,
            lang.choose("финальная правка", "final fix"),
            &step.text,
        );

        let confirm = tandem_confirm_prompt(task, &transcript.render(), lang);
        let step = match run_provider_once(
            critic,
            critic_effort,
            &confirm,
            work_dir,
            RunAccess::PlanReadonly,
            lang,
            &tx,
            &cancel_rx,
        )? {
            Some(s) => s,
            None => {
                dirty_notice(&tx);
                return Ok(TandemResult::Cancelled);
            }
        };
        tandem_accumulate(&mut total, &step.usage);
        emit_tandem_step(
            &tx,
            "🅒",
            critic_name,
            &format!(
                "{} · {}",
                lang.choose("подтверждение", "confirmation"),
                crit_role
            ),
            &step.text,
        );
        if !parse_tandem_signal(&step.text) {
            tandem_notice(
                &tx,
                lang.choose(
                    "⚠ Остались замечания критика.",
                    "⚠ The critic still has unresolved issues.",
                )
                .to_string(),
            );
        }
    }

    Ok(TandemResult::Completed(0, opt_usage(total)))
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
        // --strict-mcp-config обязателен везде: иначе MCP-инструменты из
        // глобального конфига протекают мимо --tools.
        for access in [
            RunAccess::Chat(ChatMode::Discussion),
            RunAccess::Chat(ChatMode::Plan),
            RunAccess::PlanReadonly,
            RunAccess::PlanExecute,
        ] {
            let args = claude_chat_args("high", access, "hi");
            assert!(
                args.contains(&"--strict-mcp-config"),
                "strict-mcp-config missing for {access:?}"
            );
        }

        let discussion = claude_chat_args("high", RunAccess::Chat(ChatMode::Discussion), "hi");
        let tools_idx = discussion
            .iter()
            .position(|a| *a == "--tools")
            .expect("--tools present");
        assert_eq!(
            discussion[tools_idx + 1],
            "",
            "Discussion must be tool-free"
        );

        let readonly = claude_chat_args("high", RunAccess::PlanReadonly, "hi");
        let ro_tools = readonly
            .iter()
            .position(|a| *a == "--tools")
            .expect("--tools present");
        assert!(readonly[ro_tools + 1].contains("Read"));
        assert!(!readonly[ro_tools + 1].contains("Bash"));

        let execute = claude_chat_args("high", RunAccess::PlanExecute, "hi");
        let ex_tools = execute
            .iter()
            .position(|a| *a == "--tools")
            .expect("--tools present");
        assert!(execute[ex_tools + 1].contains("Bash"));
    }

    #[test]
    fn plan_prompt_forbids_file_changes() {
        let p = plan_prompt("add a feature", "", Language::En);
        assert!(p.contains("Do NOT modify"));
        assert!(p.contains("add a feature"));
    }

    #[test]
    fn execute_prompt_embeds_full_plan() {
        let p = execute_prompt(
            "add a feature",
            "1. first step\n2. second step",
            "",
            Language::En,
        );
        assert!(p.contains("Approved plan"));
        assert!(p.contains("first step"));
        assert!(p.contains("second step"));
    }

    #[test]
    fn refine_prompt_carries_feedback_and_prev_plan() {
        let p = refine_prompt(
            "add a feature",
            "1. old step",
            "make it simpler",
            "",
            Language::En,
        );
        assert!(p.contains("old step"));
        assert!(p.contains("make it simpler"));
        assert!(p.contains("Do NOT modify"));
    }

    #[test]
    fn tandem_signal_parses_last_marker() {
        assert!(parse_tandem_signal("bla bla\nTANDEM: CONSENSUS"));
        assert!(!parse_tandem_signal("TANDEM: CONTINUE\nmore text"));
        assert!(!parse_tandem_signal("no signal here"));
        // последний маркер решает
        assert!(!parse_tandem_signal(
            "TANDEM: CONSENSUS\n...\nTANDEM: CONTINUE"
        ));
    }

    #[test]
    fn tandem_prompts_carry_role_and_signal_rules() {
        let ch = tandem_challenge_prompt("do x", "", Language::En);
        assert!(ch.contains("CRITIC"));
        assert!(ch.contains("TANDEM: CONSENSUS"));
        assert!(ch.contains("Do NOT agree out of politeness"));

        let ex = tandem_execute_prompt("do x", "approach", Language::En);
        assert!(ex.contains("EXECUTOR"));
        assert!(ex.contains("edit files"));

        let fix = tandem_fix_prompt("do x", "", "fix the bug", Language::En);
        assert!(fix.contains("fix the bug"));
    }

    #[test]
    fn tandem_transcript_renders_and_truncates() {
        let mut t = TandemTranscript::new();
        t.push("Executor", "proposal 1", "short");
        assert!(t.render().contains("short"));
        for i in 0..60 {
            t.push("Critic", "round", &format!("entry {i} {}", "y".repeat(400)));
        }
        assert!(t.render().contains("усечены"));
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
