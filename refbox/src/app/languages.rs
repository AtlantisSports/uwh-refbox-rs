use log::error;
use unic_langid::LanguageIdentifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    Dutch,
    Japanese,
    Malay,
    Portuguese,
    Thai,
    Turkish,
}

/// Which of the three bundled typefaces draws a language.
///
/// refbox chooses one font for the whole UI at startup, so this single grouping
/// settles three questions at once: which family iced is started with, which
/// font a widget is handed explicitly, and whether changing language needs a
/// restart -- only a change of *typeface* does.
///
/// This is the only place languages are grouped by typeface. It used to be
/// written out in six: `default_font_for` in `main.rs`, two copies of
/// `selected_font`, and three of `font_family_id`. One of those carried a
/// comment explaining that duplicating five lines beat widening a module's
/// visibility -- true of any one copy, and how six of them accumulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiFont {
    Latin,
    Cjk,
    Thai,
}

impl Language {
    /// The bundled typeface this language is drawn in.
    ///
    /// Deliberately exhaustive rather than falling back to Latin: a language
    /// added without a considered answer here should stop the compiler, not
    /// quietly render as blank boxes on the scoreboard the way an unlisted
    /// Asian language silently would.
    pub(crate) fn ui_font(self) -> UiFont {
        match self {
            Self::Korean | Self::Japanese | Self::Mandarin => UiFont::Cjk,
            Self::Thai => UiFont::Thai,
            Self::English
            | Self::French
            | Self::Spanish
            | Self::Italian
            | Self::German
            | Self::Tagalog
            | Self::Indonesian
            | Self::Dutch
            | Self::Malay
            | Self::Portuguese
            | Self::Turkish => UiFont::Latin,
        }
    }
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
            Self::Dutch => LanguageIdentifier::from_bytes(b"nl-NL").unwrap(),
            Self::Japanese => LanguageIdentifier::from_bytes(b"ja-JP").unwrap(),
            Self::Malay => LanguageIdentifier::from_bytes(b"ms-MY").unwrap(),
            Self::Portuguese => LanguageIdentifier::from_bytes(b"pt-PT").unwrap(),
            Self::Thai => LanguageIdentifier::from_bytes(b"th-TH").unwrap(),
            Self::Turkish => LanguageIdentifier::from_bytes(b"tr-TR").unwrap(),
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
        } else if lang_id.matches(&"nl".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Dutch
        } else if lang_id.matches(&"ja".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Japanese
        } else if lang_id.matches(&"ms".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Malay
        } else if lang_id.matches(&"pt".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Portuguese
        } else if lang_id.matches(&"th".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Thai
        } else if lang_id.matches(&"tr".parse::<LanguageIdentifier>().unwrap(), false, true) {
            Self::Turkish
        } else {
            error!("Unsupported language: {}", lang_id);
            Self::English // Default to English if unsupported
        }
    }

    pub fn cancel_text(self) -> &'static str {
        match self {
            Self::English => "CANCEL",
            Self::French => "ANNULER",
            Self::Spanish => "CANCELAR",
            Self::Mandarin => "取消",
            Self::Korean => "취소",
            Self::Italian => "ANNULLA",
            Self::German => "ABBRECHEN",
            Self::Tagalog => "KANSELAHIN",
            Self::Indonesian => "BATAL",
            Self::Dutch => "ANNULEREN",
            Self::Japanese => "キャンセル",
            Self::Malay => "BATAL",
            Self::Portuguese => "CANCELAR",
            Self::Thai => "ยกเลิก",
            Self::Turkish => "İPTAL",
        }
    }

    pub fn back_text(self) -> &'static str {
        match self {
            Self::English => "BACK",
            Self::French => "RETOUR",
            Self::Spanish => "ATRÁS",
            Self::Mandarin => "返回",
            Self::Korean => "뒤로",
            Self::Italian => "INDIETRO",
            Self::German => "ZURÜCK",
            Self::Tagalog => "BUMALIK",
            Self::Indonesian => "KEMBALI",
            Self::Dutch => "TERUG",
            Self::Japanese => "戻る",
            Self::Malay => "KEMBALI",
            Self::Portuguese => "VOLTAR",
            Self::Thai => "กลับ",
            Self::Turkish => "GERİ",
        }
    }

    pub fn apply_text(self) -> &'static str {
        match self {
            Self::English => "APPLY",
            Self::French => "APPLIQUER",
            Self::Spanish => "APLICAR",
            Self::Mandarin => "应用",
            Self::Korean => "적용",
            Self::Italian => "APPLICA",
            Self::German => "ANWENDEN",
            Self::Tagalog => "ILAPAT",
            Self::Indonesian => "TERAPKAN",
            Self::Dutch => "TOEPASSEN",
            Self::Japanese => "適用",
            Self::Malay => "GUNA",
            Self::Portuguese => "APLICAR",
            Self::Thai => "ใช้",
            Self::Turkish => "UYGULA",
        }
    }

    pub fn restart_text(self) -> &'static str {
        match self {
            Self::English => "RESTART TO APPLY",
            Self::French => "REDÉMARRER POUR APPLIQUER",
            Self::Spanish => "REINICIAR PARA APLICAR",
            Self::Mandarin => "重启以应用",
            Self::Korean => "재시작하여 적용",
            Self::Italian => "RIAVVIA PER APPLICARE",
            Self::German => "NEU STARTEN",
            Self::Tagalog => "I-RESTART UPANG ILAPAT",
            Self::Indonesian => "MULAI ULANG",
            Self::Dutch => "OPNIEUW STARTEN",
            Self::Japanese => "再起動して適用",
            Self::Malay => "MULAKAN SEMULA",
            Self::Portuguese => "REINICIAR PARA APLICAR",
            Self::Thai => "รีสตาร์ทเพื่อใช้งาน",
            Self::Turkish => "UYGULAMAK İÇİN YENİDEN BAŞLAT",
        }
    }
}

#[cfg(test)]
mod tests {
    /// Every language, with the typeface it was drawn in before the six copies
    /// of this decision were collapsed into one. This is what makes the
    /// collapse provable rather than asserted: moving any language between
    /// groups fails here, whatever the code that reads it looks like.
    #[test]
    fn every_language_keeps_the_typeface_it_shipped_with() {
        for (lang, expected) in [
            (Language::English, UiFont::Latin),
            (Language::French, UiFont::Latin),
            (Language::Spanish, UiFont::Latin),
            (Language::Mandarin, UiFont::Cjk),
            (Language::Korean, UiFont::Cjk),
            (Language::Italian, UiFont::Latin),
            (Language::German, UiFont::Latin),
            (Language::Tagalog, UiFont::Latin),
            (Language::Indonesian, UiFont::Latin),
            (Language::Dutch, UiFont::Latin),
            (Language::Japanese, UiFont::Cjk),
            (Language::Malay, UiFont::Latin),
            (Language::Portuguese, UiFont::Latin),
            (Language::Thai, UiFont::Thai),
            (Language::Turkish, UiFont::Latin),
        ] {
            assert_eq!(lang.ui_font(), expected, "{lang:?} changed typeface");
        }
    }

    /// A language change needs a restart only when the typeface changes, which
    /// is what the three deleted `font_family_id` copies were comparing. Both
    /// directions, so an implementation that answered the same way every time
    /// would fail.
    #[test]
    fn only_a_change_of_typeface_needs_a_restart() {
        assert_ne!(Language::English.ui_font(), Language::Japanese.ui_font());
        assert_ne!(Language::Japanese.ui_font(), Language::Thai.ui_font());
        assert_eq!(Language::Japanese.ui_font(), Language::Korean.ui_font());
        assert_eq!(Language::English.ui_font(), Language::Turkish.ui_font());
    }

    use super::*;

    #[test]
    fn back_text_matches_known_values() {
        assert_eq!(Language::English.back_text(), "BACK");
        assert_eq!(Language::French.back_text(), "RETOUR");
        assert_eq!(Language::German.back_text(), "ZURÜCK");
        assert_eq!(Language::Japanese.back_text(), "戻る");
        assert_eq!(Language::Mandarin.back_text(), "返回");
        // Back must read differently from Cancel in every language.
        for lang in [
            Language::English,
            Language::French,
            Language::Spanish,
            Language::Mandarin,
            Language::Korean,
            Language::Italian,
            Language::German,
            Language::Tagalog,
            Language::Indonesian,
            Language::Dutch,
            Language::Japanese,
            Language::Malay,
            Language::Portuguese,
            Language::Thai,
            Language::Turkish,
        ] {
            assert!(!lang.back_text().is_empty());
            assert_ne!(lang.back_text(), lang.cancel_text());
        }
    }
}
