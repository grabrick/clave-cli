use serde_json::Value;

/// Один вариант выбора: подпись + необязательная аннотация (пояснение).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AskOption {
    pub(crate) label: String,
    pub(crate) note: Option<String>,
}

/// Разобранный запрос выбора от модели (блок ```clave-ask).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AskPrompt {
    pub(crate) question: String,
    pub(crate) multi: bool,
    pub(crate) options: Vec<AskOption>,
}

struct AskBlock {
    /// Смещение начала строки с открывающим маркером (чтобы вырезать блок целиком).
    start: usize,
    json: String,
}

/// Находит первый блок ```clave-ask … ``` и парсит его JSON.
///
/// Возвращает `(текст_без_блока, Some(prompt))` при успехе. Если блока нет, JSON
/// невалиден или вариантов меньше двух — `(исходный_текст, None)`: блок остаётся
/// обычным текстом (мягкий фолбэк). Парсить нужно СЫРОЙ ответ модели, до того как
/// UI вешает префикс «⏺» и прячет ограждение ```.
pub(crate) fn parse_clave_ask(text: &str) -> (String, Option<AskPrompt>) {
    let Some(block) = find_ask_block(text) else {
        return (text.to_string(), None);
    };
    let Some(prompt) = parse_ask_json(&block.json) else {
        return (text.to_string(), None);
    };
    // Проза до блока сохраняется; хвост после блока в V0 отбрасываем.
    let prose = text[..block.start].trim_end().to_string();
    (prose, Some(prompt))
}

fn find_ask_block(text: &str) -> Option<AskBlock> {
    const MARKER: &str = "```clave-ask";
    let open = text.find(MARKER)?;
    let line_start = text[..open].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let after_marker = open + MARKER.len();
    // тело начинается со следующей строки после маркера
    let body_start = text[after_marker..]
        .find('\n')
        .map(|i| after_marker + i + 1)?;
    let close_rel = text[body_start..].find("```")?;
    let json = text[body_start..body_start + close_rel].trim().to_string();
    Some(AskBlock {
        start: line_start,
        json,
    })
}

/// Истинно ли значение `multi` (терпимо к bool / "true" / 1 / "yes" / "да").
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::String(s) => matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "true" | "1" | "yes" | "да"
        ),
        Value::Number(n) => n.as_i64() == Some(1) || n.as_f64() == Some(1.0),
        _ => false,
    }
}

fn parse_ask_json(json: &str) -> Option<AskPrompt> {
    let value: Value = serde_json::from_str(json).ok()?;
    let question = value.get("question")?.as_str()?.trim().to_string();
    if question.is_empty() {
        return None;
    }
    // Нестрого: модели часто отдают multi не булевым (строкой "true", числом 1) —
    // иначе строгий as_bool() молча даёт false и множественный выбор «ломается».
    let multi = value.get("multi").is_some_and(is_truthy);
    let options_val = value.get("options")?.as_array()?;

    let mut options = Vec::new();
    for opt in options_val {
        let Some(label) = opt
            .get("label")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let note = opt
            .get("note")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        options.push(AskOption { label, note });
    }

    // Селектор имеет смысл только при ≥2 вариантах — иначе фолбэк в обычный текст.
    if options.len() < 2 {
        return None;
    }
    Some(AskPrompt {
        question,
        multi,
        options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(json: &str) -> String {
        format!("```clave-ask\n{json}\n```")
    }

    #[test]
    fn parses_single_select_with_notes() {
        let text = block(
            r#"{"question":"Какой подход?","multi":false,
                "options":[{"label":"JWT","note":"stateless"},{"label":"Сессии"}]}"#,
        );
        let (prose, prompt) = parse_clave_ask(&text);
        let prompt = prompt.expect("валидный блок → Some");
        assert_eq!(prose, "", "блок без прозы → пустой текст");
        assert_eq!(prompt.question, "Какой подход?");
        assert!(!prompt.multi);
        assert_eq!(prompt.options.len(), 2);
        assert_eq!(prompt.options[0].label, "JWT");
        assert_eq!(prompt.options[0].note.as_deref(), Some("stateless"));
        assert_eq!(prompt.options[1].note, None, "note необязателен");
    }

    #[test]
    fn parses_multi_select() {
        let text = block(
            r#"{"question":"Что включить?","multi":true,
            "options":[{"label":"A"},{"label":"B"},{"label":"C"}]}"#,
        );
        let (_, prompt) = parse_clave_ask(&text);
        let prompt = prompt.expect("Some");
        assert!(prompt.multi);
        assert_eq!(prompt.options.len(), 3);
    }

    #[test]
    fn multi_is_parsed_leniently() {
        // Модель отдала multi строкой "true" — всё равно множественный выбор.
        for raw in [r#""true""#, "1", r#""yes""#] {
            let text = block(&format!(
                r#"{{"question":"q","multi":{raw},"options":[{{"label":"A"}},{{"label":"B"}}]}}"#
            ));
            let (_, prompt) = parse_clave_ask(&text);
            assert!(prompt.expect("Some").multi, "multi={raw} должно быть true");
        }
        // А вот ложные значения — одиночный.
        for raw in [r#""false""#, "0", "null"] {
            let text = block(&format!(
                r#"{{"question":"q","multi":{raw},"options":[{{"label":"A"}},{{"label":"B"}}]}}"#
            ));
            let (_, prompt) = parse_clave_ask(&text);
            assert!(
                !prompt.expect("Some").multi,
                "multi={raw} должно быть false"
            );
        }
    }

    #[test]
    fn keeps_prose_before_block_and_strips_block() {
        let text = format!(
            "Рассуждение по теме.\nЕщё строка.\n{}",
            block(r#"{"question":"Какую БД?","options":[{"label":"PG"},{"label":"SQLite"}]}"#,)
        );
        let (prose, prompt) = parse_clave_ask(&text);
        assert_eq!(prose, "Рассуждение по теме.\nЕщё строка.");
        assert!(prompt.is_some());
        assert!(!prose.contains("clave-ask"), "блок вырезан из прозы");
    }

    #[test]
    fn fewer_than_two_options_is_fallback_to_text() {
        let text = block(r#"{"question":"?","options":[{"label":"Один"}]}"#);
        let (prose, prompt) = parse_clave_ask(&text);
        assert!(prompt.is_none(), "<2 вариантов → не селектор");
        assert_eq!(prose, text, "текст не тронут (фолбэк)");
    }

    #[test]
    fn malformed_json_is_fallback_to_text() {
        let text = block(r#"{"question": "битый", "options": [ {"label": }"#);
        let (prose, prompt) = parse_clave_ask(&text);
        assert!(prompt.is_none());
        assert_eq!(prose, text);
    }

    #[test]
    fn plain_text_without_block_is_unchanged() {
        let text = "Обычный ответ без выбора.";
        let (prose, prompt) = parse_clave_ask(text);
        assert!(prompt.is_none());
        assert_eq!(prose, text);
    }

    #[test]
    fn empty_labels_are_skipped_then_validated() {
        // Два валидных варианта + один пустой label → пустой пропускаем, остаётся 2.
        let text =
            block(r#"{"question":"q","options":[{"label":"A"},{"label":"  "},{"label":"B"}]}"#);
        let (_, prompt) = parse_clave_ask(&text);
        let prompt = prompt.expect("Some");
        assert_eq!(prompt.options.len(), 2);
        assert_eq!(prompt.options[1].label, "B");
    }
}
