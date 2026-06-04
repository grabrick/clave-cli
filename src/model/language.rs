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
