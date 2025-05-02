use log::error;
use unic_langid::LanguageIdentifier;

use super::Cyclable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    French,
    Spanish,
}

impl Language {
    pub fn as_lang_id(&self) -> LanguageIdentifier {
        match self {
            Self::English => LanguageIdentifier::from_bytes(b"en").unwrap(),
            Self::French => LanguageIdentifier::from_bytes(b"fr").unwrap(),
            Self::Spanish => LanguageIdentifier::from_bytes(b"es").unwrap(),
        }
    }

    pub fn from_lang_id(lang_id: &LanguageIdentifier) -> Self {
        if lang_id.matches(&"en".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::English
        } else if lang_id.matches(&"fr".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::French
        } else if lang_id.matches(&"es".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Spanish
        } else {
            error!("Unsupported language: {}", lang_id);
            Self::English // Default to English if unsupported
        }
    }
}

impl Cyclable for Language {
    fn next(&self) -> Self {
        match self {
            Self::English => Self::French,
            Self::French => Self::Spanish,
            Self::Spanish => Self::English,
        }
    }
}
