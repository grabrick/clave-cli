use super::Language;
use crate::prelude::*;

/// Куда открывать путь к файлу по Cmd+клику в транскрипте. Хранится в конфиге
/// (`path_link_target`), меняется в /settings. Клик по OSC 8-ссылке обрабатывает сам
/// терминал — clave лишь строит доверенный URL по выбранной цели (контент в URL не
/// попадает, см. безопасность в `render::queue_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PathTarget {
    VsCode,
    Cursor,
    /// Системное приложение macOS (file://), без номера строки.
    Default,
    /// Линковка выключена.
    Off,
}

impl PathTarget {
    pub(crate) fn as_config_str(self) -> &'static str {
        match self {
            PathTarget::VsCode => "vscode",
            PathTarget::Cursor => "cursor",
            PathTarget::Default => "default",
            PathTarget::Off => "off",
        }
    }

    /// Терпимый парсер конфига (легаси-алиасы в духе остального config-слоя).
    pub(crate) fn from_config_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "vscode" | "code" | "vs-code" => Some(PathTarget::VsCode),
            "cursor" => Some(PathTarget::Cursor),
            "default" | "file" | "finder" => Some(PathTarget::Default),
            "off" | "none" | "disabled" => Some(PathTarget::Off),
            _ => None,
        }
    }

    /// Метка для экрана настроек.
    pub(crate) fn label(self, lang: Language) -> &'static str {
        match self {
            PathTarget::VsCode => "VS Code",
            PathTarget::Cursor => "Cursor",
            PathTarget::Default => lang.choose("Системное приложение", "Default app"),
            PathTarget::Off => lang.choose("Выключено", "Off"),
        }
    }
}

/// Строит доверенный URL для OSC 8-ссылки. `abs` обязан быть абсолютным путём.
/// `None` означает «не линковать» (Off). Строку/колонку используют только редакторы.
pub(crate) fn open_url(
    target: PathTarget,
    abs: &Path,
    line: Option<u32>,
    col: Option<u32>,
) -> Option<String> {
    let path = abs.to_string_lossy();
    match target {
        PathTarget::Off => None,
        // abs начинается с '/', поэтому file:// + /Users → file:///Users (3 слэша).
        PathTarget::Default => Some(format!("file://{path}")),
        // Схема VS Code: vscode://file<abs>:<line>:<col>; ведущий '/' пути даёт слэш.
        PathTarget::VsCode => Some(format!(
            "vscode://file{path}:{}:{}",
            line.unwrap_or(1),
            col.unwrap_or(1)
        )),
        PathTarget::Cursor => Some(format!(
            "cursor://file{path}:{}:{}",
            line.unwrap_or(1),
            col.unwrap_or(1)
        )),
    }
}

/// Доступные цели на этой машине: установленные редакторы + всегда `Default` и `Off`.
/// `probe(target)` сообщает, установлен ли редактор (инъекция в тестах).
pub(crate) fn available_targets(probe: impl Fn(PathTarget) -> bool) -> Vec<PathTarget> {
    let mut out = Vec::new();
    for target in [PathTarget::VsCode, PathTarget::Cursor] {
        if probe(target) {
            out.push(target);
        }
    }
    out.push(PathTarget::Default);
    out.push(PathTarget::Off);
    out
}

/// Автодефолт при первом запуске: VS Code → Cursor → системное приложение.
pub(crate) fn auto_default(probe: impl Fn(PathTarget) -> bool) -> PathTarget {
    if probe(PathTarget::VsCode) {
        PathTarget::VsCode
    } else if probe(PathTarget::Cursor) {
        PathTarget::Cursor
    } else {
        PathTarget::Default
    }
}

/// Реальный пробник: редактор установлен, если есть его `.app` в /Applications или
/// бинарь в PATH. Для Default/Off всегда true.
pub(crate) fn editor_installed(target: PathTarget) -> bool {
    match target {
        PathTarget::VsCode => app_or_bin("Visual Studio Code", "code"),
        PathTarget::Cursor => app_or_bin("Cursor", "cursor"),
        PathTarget::Default | PathTarget::Off => true,
    }
}

fn app_or_bin(app: &str, bin: &str) -> bool {
    if Path::new(&format!("/Applications/{app}.app")).exists() {
        return true;
    }
    env::var_os("PATH")
        .map(|paths| env::split_paths(&paths).any(|dir| dir.join(bin).exists()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vscode_url_carries_file_line_col() {
        let p = PathBuf::from("/Users/k/src/app.rs");
        assert_eq!(
            open_url(PathTarget::VsCode, &p, Some(42), Some(7)).unwrap(),
            "vscode://file/Users/k/src/app.rs:42:7"
        );
    }

    #[test]
    fn vscode_url_defaults_line_and_col_to_one() {
        let p = PathBuf::from("/a/b.rs");
        assert_eq!(
            open_url(PathTarget::VsCode, &p, None, None).unwrap(),
            "vscode://file/a/b.rs:1:1"
        );
    }

    #[test]
    fn cursor_url_uses_cursor_scheme() {
        let p = PathBuf::from("/a/b.rs");
        assert_eq!(
            open_url(PathTarget::Cursor, &p, Some(3), None).unwrap(),
            "cursor://file/a/b.rs:3:1"
        );
    }

    #[test]
    fn default_url_is_file_scheme_without_line() {
        let p = PathBuf::from("/a/b.rs");
        // Три слэша: file:// + абсолютный путь. Номер строки file:// не несёт.
        assert_eq!(
            open_url(PathTarget::Default, &p, Some(3), Some(9)).unwrap(),
            "file:///a/b.rs"
        );
    }

    #[test]
    fn off_yields_no_url() {
        let p = PathBuf::from("/a/b.rs");
        assert_eq!(open_url(PathTarget::Off, &p, None, None), None);
    }

    #[test]
    fn config_roundtrip_and_aliases() {
        for target in [
            PathTarget::VsCode,
            PathTarget::Cursor,
            PathTarget::Default,
            PathTarget::Off,
        ] {
            assert_eq!(
                PathTarget::from_config_str(target.as_config_str()),
                Some(target)
            );
        }
        assert_eq!(
            PathTarget::from_config_str("CODE"),
            Some(PathTarget::VsCode)
        );
        assert_eq!(
            PathTarget::from_config_str("  none "),
            Some(PathTarget::Off)
        );
        assert_eq!(PathTarget::from_config_str("nonsense"), None);
    }

    #[test]
    fn auto_default_prefers_vscode_then_cursor_then_default() {
        assert_eq!(
            auto_default(|t| t == PathTarget::VsCode),
            PathTarget::VsCode
        );
        assert_eq!(
            auto_default(|t| t == PathTarget::Cursor),
            PathTarget::Cursor
        );
        assert_eq!(auto_default(|_| false), PathTarget::Default);
    }

    #[test]
    fn available_targets_always_offers_default_and_off() {
        assert_eq!(
            available_targets(|_| false),
            vec![PathTarget::Default, PathTarget::Off]
        );
        assert_eq!(
            available_targets(|_| true),
            vec![
                PathTarget::VsCode,
                PathTarget::Cursor,
                PathTarget::Default,
                PathTarget::Off
            ]
        );
    }
}
