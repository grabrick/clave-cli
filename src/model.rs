use crate::prelude::*;

pub(crate) type AnyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Copy)]
pub(crate) struct CommandSpec {
    pub(crate) usage: &'static str,
    pub(crate) insert: &'static str,
    pub(crate) description_en: &'static str,
    pub(crate) description_ru: &'static str,
}

pub(crate) const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        usage: "/brainstorming",
        insert: "/brainstorming",
        description_en: "(superpowers) Explore options before creative work",
        description_ru: "(superpowers) Исследовать варианты перед творческой работой",
    },
    CommandSpec {
        usage: "/writing-plans",
        insert: "/writing-plans",
        description_en: "(superpowers) Turn a spec into a multi-step plan",
        description_ru: "(superpowers) Превратить спеку в пошаговый план",
    },
    CommandSpec {
        usage: "/finishing-a-development-branch",
        insert: "/finishing-a-development-branch",
        description_en: "(superpowers) Decide how to complete and polish work",
        description_ru: "(superpowers) Довести ветку разработки до завершения",
    },
    CommandSpec {
        usage: "/subagent-driven-development",
        insert: "/subagent-driven-development",
        description_en: "(superpowers) Split implementation across agents",
        description_ru: "(superpowers) Разделить реализацию между агентами",
    },
    CommandSpec {
        usage: "/using-git-worktrees",
        insert: "/using-git-worktrees",
        description_en: "(superpowers) Use isolated workspaces for parallel work",
        description_ru: "(superpowers) Использовать worktree для параллельной работы",
    },
    CommandSpec {
        usage: "/add-dir",
        insert: "/add-dir ",
        description_en: "Add a new working directory",
        description_ru: "Добавить рабочую директорию",
    },
    CommandSpec {
        usage: "/advisor",
        insert: "/advisor",
        description_en: "Consult a stronger model for guidance",
        description_ru: "Попросить сильную модель о совете",
    },
    CommandSpec {
        usage: "/agents",
        insert: "/agents",
        description_en: "Manage agent configurations",
        description_ru: "Управлять конфигурациями агентов",
    },
    CommandSpec {
        usage: "/autofix-pr",
        insert: "/autofix-pr",
        description_en: "Monitor and autofix issues with the current PR",
        description_ru: "Отслеживать и исправлять проблемы в текущем PR",
    },
    CommandSpec {
        usage: "/background",
        insert: "/background",
        description_en: "Send this session to the background",
        description_ru: "Отправить сессию в фон",
    },
    CommandSpec {
        usage: "/branch",
        insert: "/branch",
        description_en: "Create a branch of the current conversation",
        description_ru: "Создать ветку текущего разговора",
    },
    CommandSpec {
        usage: "/btw",
        insert: "/btw ",
        description_en: "Ask a quick side question",
        description_ru: "Задать быстрый побочный вопрос",
    },
    CommandSpec {
        usage: "/plan <task>",
        insert: "/plan ",
        description_en: "Run the multi-agent spec-duel planning loop",
        description_ru: "Запустить multi-agent планирование spec-duel",
    },
    CommandSpec {
        usage: "/clear",
        insert: "/clear",
        description_en: "Start fresh with empty context",
        description_ru: "Очистить контекст",
    },
    CommandSpec {
        usage: "/new",
        insert: "/new",
        description_en: "Start a new saved chat",
        description_ru: "Начать новый сохранённый чат",
    },
    CommandSpec {
        usage: "/chats",
        insert: "/chats",
        description_en: "Show saved chats",
        description_ru: "Показать сохранённые чаты",
    },
    CommandSpec {
        usage: "/resume <id>",
        insert: "/resume ",
        description_en: "Resume a saved chat",
        description_ru: "Открыть сохранённый чат",
    },
    CommandSpec {
        usage: "/color",
        insert: "/color ",
        description_en: "Set the prompt bar color",
        description_ru: "Изменить цвет строки ввода",
    },
    CommandSpec {
        usage: "/effort",
        insert: "/effort",
        description_en: "Adjust model effort level",
        description_ru: "Настроить уровень усилия модели",
    },
    CommandSpec {
        usage: "/logout",
        insert: "/logout",
        description_en: "Return to CLI authentication",
        description_ru: "Вернуться к авторизации CLI",
    },
    CommandSpec {
        usage: "/help",
        insert: "/help",
        description_en: "Show available commands",
        description_ru: "Показать доступные команды",
    },
    CommandSpec {
        usage: "/lang ru|en",
        insert: "/lang ",
        description_en: "Switch interface language",
        description_ru: "Переключить язык интерфейса",
    },
    CommandSpec {
        usage: "/mode codex-only",
        insert: "/mode codex-only",
        description_en: "Use Codex for both architect and reviewer",
        description_ru: "Использовать Codex как архитектора и ревьюера",
    },
    CommandSpec {
        usage: "/mode claude-only",
        insert: "/mode claude-only",
        description_en: "Use Claude for both architect and reviewer",
        description_ru: "Использовать Claude как архитектора и ревьюера",
    },
    CommandSpec {
        usage: "/mode claude-codex",
        insert: "/mode claude-codex",
        description_en: "Use Claude as architect and Codex as reviewer",
        description_ru: "Использовать Claude как архитектора и Codex как ревьюера",
    },
    CommandSpec {
        usage: "/rounds <n>",
        insert: "/rounds ",
        description_en: "Set review loop limit",
        description_ru: "Задать лимит раундов ревью",
    },
    CommandSpec {
        usage: "/out <dir>",
        insert: "/out ",
        description_en: "Set artifact directory",
        description_ru: "Задать папку артефактов",
    },
    CommandSpec {
        usage: "/status",
        insert: "/status",
        description_en: "Print session state",
        description_ru: "Показать состояние сессии",
    },
    CommandSpec {
        usage: "/setup",
        insert: "/setup",
        description_en: "Open first-run setup again",
        description_ru: "Открыть стартовую настройку заново",
    },
    CommandSpec {
        usage: "/quit",
        insert: "/quit",
        description_en: "Exit Duel",
        description_ru: "Выйти из Duel",
    },
];

pub(crate) const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
pub(crate) const CODEX_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh"];
pub(crate) const CLAUDE_EFFORTS: &[&str] = &["low", "medium", "high", "max"];
pub(crate) const COMMON_EFFORTS: &[&str] = &["low", "medium", "high"];
pub(crate) const ACCENT: Color = Color::Indexed(141);
pub(crate) const ACCENT_SOFT: Color = Color::Indexed(183);
pub(crate) const ACCENT_DIM: Color = Color::Indexed(97);
pub(crate) const ACCENT_BG: Color = Color::Indexed(60);
pub(crate) const MUTED: Color = Color::Gray;
pub(crate) const MAX_TRANSCRIPT_LINES: usize = 700;
pub(crate) const MAX_HISTORY_LINES: usize = 200;
pub(crate) const CHAT_FILE_EXTENSION: &str = "duel";
pub(crate) const LOADER_PHRASES: &[&str] = &[
    "Spelunking",
    "Thinking",
    "Reading context",
    "Drafting",
    "Reviewing",
    "Polishing",
];
impl CommandSpec {
    pub(crate) fn description(self, lang: Language) -> &'static str {
        match lang {
            Language::En => self.description_en,
            Language::Ru => self.description_ru,
        }
    }
}

pub(crate) fn effort_label(index: usize) -> &'static str {
    EFFORTS.get(index).copied().unwrap_or("high")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    CodexOnly,
    ClaudeOnly,
    ClaudeCodex,
}

impl Mode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Mode::CodexOnly => "codex-only",
            Mode::ClaudeOnly => "claude-only",
            Mode::ClaudeCodex => "claude-codex",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "codex-only" => Some(Mode::CodexOnly),
            "claude-only" => Some(Mode::ClaudeOnly),
            "claude-codex" => Some(Mode::ClaudeCodex),
            _ => None,
        }
    }

    pub(crate) fn needs_codex(self) -> bool {
        matches!(self, Mode::CodexOnly | Mode::ClaudeCodex)
    }

    pub(crate) fn needs_claude(self) -> bool {
        matches!(self, Mode::ClaudeOnly | Mode::ClaudeCodex)
    }
}

pub(crate) fn provider_supports_effort(provider: &str, effort: &str) -> bool {
    match provider {
        "codex" => matches!(effort, "low" | "medium" | "high" | "xhigh"),
        "claude" => matches!(effort, "low" | "medium" | "high" | "max"),
        _ => false,
    }
}

pub(crate) fn provider_allowed_efforts(provider: &str) -> &'static [&'static str] {
    match provider {
        "codex" => CODEX_EFFORTS,
        "claude" => CLAUDE_EFFORTS,
        _ => COMMON_EFFORTS,
    }
}

pub(crate) fn effort_index_for(effort: &str) -> usize {
    EFFORTS
        .iter()
        .position(|value| *value == effort)
        .unwrap_or(2)
}

pub(crate) fn normalize_effort_index_for(allowed: &[&str], index: usize) -> usize {
    let effort = effort_label(index);
    if allowed.iter().any(|allowed| *allowed == effort) {
        index
    } else {
        effort_index_for("high")
    }
}

pub(crate) fn normalize_common_effort_index(index: usize) -> usize {
    normalize_effort_index_for(COMMON_EFFORTS, index)
}

pub(crate) fn normalize_provider_effort_index(provider: &str, index: usize) -> usize {
    normalize_effort_index_for(provider_allowed_efforts(provider), index)
}

pub(crate) fn move_effort_index_in(allowed: &[&str], index: usize, direction: isize) -> usize {
    let normalized = normalize_effort_index_for(allowed, index);
    let current = effort_label(normalized);
    let current_pos = allowed
        .iter()
        .position(|effort| *effort == current)
        .unwrap_or_else(|| {
            allowed
                .iter()
                .position(|effort| *effort == "high")
                .unwrap_or(0)
        });
    let next_pos = if direction < 0 {
        current_pos.saturating_sub(1)
    } else {
        (current_pos + 1).min(allowed.len().saturating_sub(1))
    };
    effort_index_for(allowed[next_pos])
}

pub(crate) fn move_common_effort_index(index: usize, direction: isize) -> usize {
    move_effort_index_in(COMMON_EFFORTS, index, direction)
}

pub(crate) fn move_provider_effort_index(provider: &str, index: usize, direction: isize) -> usize {
    move_effort_index_in(provider_allowed_efforts(provider), index, direction)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Language {
    Ru,
    En,
}

impl Language {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Language::Ru => "ru",
            Language::En => "en",
        }
    }

    pub(crate) fn choose(self, ru: &'static str, en: &'static str) -> &'static str {
        match self {
            Language::Ru => ru,
            Language::En => en,
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "ru" => Some(Language::Ru),
            "en" => Some(Language::En),
            _ => None,
        }
    }
}
