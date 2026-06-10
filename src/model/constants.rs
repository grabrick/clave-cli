use crate::prelude::*;

pub(crate) const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
pub(crate) const CODEX_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh"];
pub(crate) const CLAUDE_EFFORTS: &[&str] = &["low", "medium", "high", "max"];
pub(crate) const COMMON_EFFORTS: &[&str] = &["low", "medium", "high"];
pub(crate) const APP_NAME: &str = "Clave";
pub(crate) const APP_COMMAND: &str = "clave";
pub(crate) const ENGINE_NAME: &str = "spec-clave";
pub(crate) const DEFAULT_ARTIFACT_DIR: &str = ".clave";
pub(crate) const STATE_DIR_NAME: &str = ".clave";
pub(crate) const MUTED: Color = Color::Gray;
pub(crate) const MAX_TRANSCRIPT_LINES: usize = 700;
pub(crate) const MAX_HISTORY_LINES: usize = 200;
pub(crate) const CHAT_FILE_EXTENSION: &str = "clave";
/// Фразы лоадера крутятся по таймеру и задают «настроение» думающего инструмента.
/// Разнообразные и с характером — чтобы ощущался живой ИИ, а не статичный спиннер.
pub(crate) const LOADER_PHRASES_RU: &[&str] = &[
    "Думаю",
    "Читаю контекст",
    "Соображаю",
    "Прикидываю",
    "Сверяюсь",
    "Взвешиваю",
    "Кручу шестерёнки",
    "Копаюсь в деталях",
    "Раскладываю по полочкам",
    "Собираю мысли",
];
pub(crate) const LOADER_PHRASES_EN: &[&str] = &[
    "Thinking",
    "Reading context",
    "Pondering",
    "Mulling it over",
    "Cross-checking",
    "Weighing options",
    "Cranking gears",
    "Digging into details",
    "Connecting the dots",
    "Gathering thoughts",
];
