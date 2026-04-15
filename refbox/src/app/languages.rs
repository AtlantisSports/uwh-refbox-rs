use log::error;
use unic_langid::LanguageIdentifier;

use super::Cyclable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    French,
    Spanish,
    Mandarin,
    Korean,
    Italian,
    German,
    Tagalog,
    Indonesian,
}

impl Language {
    pub fn as_lang_id(&self) -> LanguageIdentifier {
        match self {
            Self::English => LanguageIdentifier::from_bytes(b"en").unwrap(),
            Self::French => LanguageIdentifier::from_bytes(b"fr").unwrap(),
            Self::Spanish => LanguageIdentifier::from_bytes(b"es").unwrap(),
            Self::Mandarin => LanguageIdentifier::from_bytes(b"zh-CN").unwrap(),
            Self::Korean => LanguageIdentifier::from_bytes(b"ko-KR").unwrap(),
            Self::Italian => LanguageIdentifier::from_bytes(b"it-IT").unwrap(),
            Self::German => LanguageIdentifier::from_bytes(b"de-DE").unwrap(),
            Self::Tagalog => LanguageIdentifier::from_bytes(b"tl-PH").unwrap(),
            Self::Indonesian => LanguageIdentifier::from_bytes(b"id-ID").unwrap(),
        }
    }

    pub fn from_lang_id(lang_id: &LanguageIdentifier) -> Self {
        if lang_id.matches(&"en".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::English
        } else if lang_id.matches(&"fr".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::French
        } else if lang_id.matches(&"es".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Spanish
        } else if lang_id.matches(&"zh".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Mandarin
        } else if lang_id.matches(&"ko".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Korean
        } else if lang_id.matches(&"it".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Italian
        } else if lang_id.matches(&"de".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::German
        } else if lang_id.matches(&"tl".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Tagalog
        } else if lang_id.matches(&"id".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Indonesian
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
            Self::Spanish => Self::Mandarin,
            Self::Mandarin => Self::Korean,
            Self::Korean => Self::Italian,
            Self::Italian => Self::German,
            Self::German => Self::Tagalog,
            Self::Tagalog => Self::Indonesian,
            Self::Indonesian => Self::English,
        }
    }
}
