use log::error;
use unic_langid::LanguageIdentifier;

use super::Cyclable;

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

    pub fn done_text(self) -> &'static str {
        match self {
            Self::English => "DONE",
            Self::French => "TERMINÉ",
            Self::Spanish => "HECHO",
            Self::Mandarin => "完成",
            Self::Korean => "완료",
            Self::Italian => "FATTO",
            Self::German => "FERTIG",
            Self::Tagalog => "TAPOS",
            Self::Indonesian => "SELESAI",
            Self::Dutch => "KLAAR",
            Self::Japanese => "完了",
            Self::Malay => "SELESAI",
            Self::Portuguese => "CONCLUÍDO",
            Self::Thai => "เสร็จสิ้น",
            Self::Turkish => "TAMAM",
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
            Self::Indonesian => Self::Dutch,
            Self::Dutch => Self::Japanese,
            Self::Japanese => Self::Malay,
            Self::Malay => Self::Portuguese,
            Self::Portuguese => Self::Thai,
            Self::Thai => Self::Turkish,
            Self::Turkish => Self::English,
        }
    }
}
