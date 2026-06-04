use crate::prelude::*;

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
