//! The overlay's fixed words in English, Spanish and French (the three CMAS languages).
//!
//! Plain tables rather than a translation library: the overlay has about 25 words, and a `match`
//! per word makes the compiler insist that every word has all three languages. Team names, player
//! names and anything else from the schedule or the portal are never translated.
//!
//! The Spanish and French start from the refbox's own translations (`refbox/translations/es` and
//! `fr`), shortened where the overlay has little room, and need a native speaker's check.

use serde::{Deserialize, Serialize};
use uwh_common::game_snapshot::{GamePeriod, Infraction};

/// A language the overlay can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Es,
    Fr,
}

/// The overlay's `language` setting: follow the refbox, or always use one language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LanguageSetting {
    /// Show the refbox's language (English if the refbox doesn't say, or uses another language).
    #[default]
    #[serde(rename = "follow-refbox")]
    FollowRefbox,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "fr")]
    French,
}

impl Lang {
    /// The language for a language code such as `"es"` or `"fr-CA"`, or `None` for one the
    /// overlay doesn't have.
    fn from_code(code: &str) -> Option<Self> {
        let primary = code.split(['-', '_']).next()?.to_ascii_lowercase();
        match primary.as_str() {
            "en" => Some(Self::En),
            "es" => Some(Self::Es),
            "fr" => Some(Self::Fr),
            _ => None,
        }
    }

    /// The language to show, from the setting and the language the refbox sent (if any).
    pub fn pick(setting: LanguageSetting, refbox_language: Option<&str>) -> Self {
        match setting {
            LanguageSetting::English => Self::En,
            LanguageSetting::Spanish => Self::Es,
            LanguageSetting::French => Self::Fr,
            LanguageSetting::FollowRefbox => refbox_language
                .and_then(Self::from_code)
                .unwrap_or_default(),
        }
    }

    /// Picks the English, Spanish or French text.
    const fn of<'a>(self, en: &'a str, es: &'a str, fr: &'a str) -> &'a str {
        match self {
            Self::En => en,
            Self::Es => es,
            Self::Fr => fr,
        }
    }

    pub const fn black(self) -> &'static str {
        self.of("BLACK", "NEGRO", "NOIR")
    }

    pub const fn white(self) -> &'static str {
        self.of("WHITE", "BLANCO", "BLANC")
    }

    /// "GAME", as in "GAME #12".
    pub const fn game(self) -> &'static str {
        self.of("GAME", "PARTIDO", "MATCH")
    }

    pub const fn next_game(self) -> &'static str {
        self.of("NEXT GAME", "PRÓXIMO PARTIDO", "MATCH SUIVANT")
    }

    pub const fn referees(self) -> &'static str {
        self.of("REFEREES", "ÁRBITROS", "ARBITRES")
    }

    /// The tab of a goal card.
    pub const fn goal(self) -> &'static str {
        self.of("GOAL", "GOL", "BUT")
    }

    /// The tab of a foul card.
    pub const fn foul(self) -> &'static str {
        self.of("FOUL", "FALTA", "FAUTE")
    }

    /// The name on a foul card for a foul against neither team.
    pub const fn both_teams(self) -> &'static str {
        self.of("Both teams", "Ambos equipos", "Les deux équipes")
    }

    /// The two lines of the referee timeout flag.
    pub const fn referee_timeout(self) -> (&'static str, &'static str) {
        match self {
            Self::En => ("REFEREE", "TIMEOUT"),
            Self::Es => ("TIEMPO MUERTO", "ÁRBITRO"),
            Self::Fr => ("TEMPS MORT", "ARBITRE"),
        }
    }

    /// The two lines of a team timeout flag (`black`: the Black team's).
    pub const fn team_timeout(self, black: bool) -> (&'static str, &'static str) {
        let team = if black { self.black() } else { self.white() };
        match self {
            Self::En => (team, "TIMEOUT"),
            // "TIEMPO MUERTO" only fits this flag in very small letters.
            Self::Es => ("T. MUERTO", team),
            Self::Fr => ("TEMPS MORT", team),
        }
    }

    /// The two lines of the penalty shot flag.
    pub const fn penalty_shot(self) -> (&'static str, &'static str) {
        match self {
            Self::En => ("PENALTY", "SHOT"),
            Self::Es => ("TIRO", "PENAL"),
            Self::Fr => ("TIR DE", "PÉNALITÉ"),
        }
    }

    /// The period's name, as shown under the game clock.
    pub const fn period(self, period: GamePeriod) -> &'static str {
        match period {
            GamePeriod::FirstHalf => self.of("1ST HALF", "1ª MITAD", "1re PÉRIODE"),
            GamePeriod::SecondHalf => self.of("2ND HALF", "2ª MITAD", "2e PÉRIODE"),
            GamePeriod::OvertimeFirstHalf => {
                self.of("OVERTIME 1ST HALF", "PRÓRROGA 1ª MITAD", "1re PROLONGATION")
            }
            GamePeriod::OvertimeSecondHalf => {
                self.of("OVERTIME 2ND HALF", "PRÓRROGA 2ª MITAD", "2e PROLONGATION")
            }
            GamePeriod::OvertimeHalfTime => {
                self.of("OVERTIME HALF TIME", "DESC. PRÓRROGA", "MI-TEMPS PROLONG.")
            }
            GamePeriod::PreOvertime => self.of("PRE OVERTIME", "ANTES PRÓRROGA", "AVANT PROLONG."),
            GamePeriod::SuddenDeath => self.of("SUDDEN DEATH", "MUERTE SÚBITA", "MORT SUBITE"),
            GamePeriod::PreSuddenDeath => {
                self.of("PRE SUDDEN DEATH", "ANTES M. SÚBITA", "AVANT MORT SUBITE")
            }
            GamePeriod::HalfTime | GamePeriod::BetweenGames => {
                self.of("HALF TIME", "DESCANSO", "MI-TEMPS")
            }
        }
    }

    /// The foul's name, as shown on the description bar (before it is put in capitals), or
    /// `None` when the foul isn't known. English uses the shared name from `uwh-common`.
    pub fn infraction(self, infraction: Infraction) -> Option<&'static str> {
        let (es, fr) = match infraction {
            Infraction::Unknown => return None,
            Infraction::StickInfringement => ("Infracción con el palo", "Faute de crosse"),
            Infraction::IllegalAdvancement => ("Avance ilegal", "Avancée irrégulière"),
            Infraction::IllegalSubstitution => ("Sustitución ilegal", "Changement incorrect"),
            Infraction::IllegallyStoppingThePuck => ("Parada ilegal", "Arrêt irrégulier"),
            Infraction::OutOfBounds => ("Pastilla fuera", "Palet sorti"),
            Infraction::GrabbingTheBarrier => ("Agarrarse a la barrera", "Agripper la barrière"),
            Infraction::Obstruction => ("Obstrucción", "Obstruction"),
            Infraction::DelayOfGame => ("Quemar tiempo", "Retarder le jeu"),
            Infraction::UnsportsmanlikeConduct => {
                ("Conducta antideportiva", "Conduite antisportive")
            }
            Infraction::FreeArm => (
                "Uso ilegal del brazo libre",
                "Usage irrégulier du bras libre",
            ),
            Infraction::FalseStart => ("Saque nulo", "Faux départ"),
        };
        Some(self.of(infraction.short_name(), es, fr))
    }

    /// The fallback team names when the schedule gives none (`network::BLACK_TEAM_NAME` and
    /// `WHITE_TEAM_NAME`), in this language, if `name` is one of them in any language.
    pub fn default_team_name(self, name: &str) -> Option<&'static str> {
        let all = [Self::En, Self::Es, Self::Fr];
        if all.iter().any(|l| l.black() == name) {
            Some(self.black())
        } else if all.iter().any(|l| l.white() == name) {
            Some(self.white())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN_FOULS: [Infraction; 11] = [
        Infraction::StickInfringement,
        Infraction::IllegalAdvancement,
        Infraction::IllegalSubstitution,
        Infraction::IllegallyStoppingThePuck,
        Infraction::OutOfBounds,
        Infraction::GrabbingTheBarrier,
        Infraction::Obstruction,
        Infraction::DelayOfGame,
        Infraction::UnsportsmanlikeConduct,
        Infraction::FreeArm,
        Infraction::FalseStart,
    ];

    #[test]
    fn following_the_refbox_uses_its_language_or_english() {
        let follow = LanguageSetting::FollowRefbox;
        assert_eq!(Lang::pick(follow, Some("es")), Lang::Es);
        assert_eq!(Lang::pick(follow, Some("fr")), Lang::Fr);
        assert_eq!(Lang::pick(follow, Some("fr-CA")), Lang::Fr);
        assert_eq!(Lang::pick(follow, Some("en")), Lang::En);
        assert_eq!(
            Lang::pick(follow, Some("de-DE")),
            Lang::En,
            "a language we don't have"
        );
        assert_eq!(Lang::pick(follow, Some("zh-CN")), Lang::En);
        assert_eq!(Lang::pick(follow, None), Lang::En, "an older refbox");
    }

    #[test]
    fn a_fixed_language_ignores_the_refbox() {
        assert_eq!(Lang::pick(LanguageSetting::French, Some("en")), Lang::Fr);
        assert_eq!(Lang::pick(LanguageSetting::Spanish, None), Lang::Es);
        assert_eq!(Lang::pick(LanguageSetting::English, Some("es")), Lang::En);
    }

    #[test]
    fn the_setting_reads_and_writes_as_short_codes() {
        #[derive(Serialize, Deserialize)]
        struct S {
            language: LanguageSetting,
        }
        for (setting, text) in [
            (LanguageSetting::FollowRefbox, "follow-refbox"),
            (LanguageSetting::English, "en"),
            (LanguageSetting::Spanish, "es"),
            (LanguageSetting::French, "fr"),
        ] {
            let line = serde_json::to_string(&S { language: setting }).expect("serialize");
            assert_eq!(line, format!("{{\"language\":\"{text}\"}}"));
            let back: S = serde_json::from_str(&line).expect("deserialize");
            assert_eq!(back.language, setting);
        }
    }

    #[test]
    fn every_known_foul_has_a_name_in_every_language() {
        for lang in [Lang::En, Lang::Es, Lang::Fr] {
            assert_eq!(lang.infraction(Infraction::Unknown), None);
            for infraction in KNOWN_FOULS {
                let name = lang
                    .infraction(infraction)
                    .expect("a known foul has a name");
                assert!(!name.is_empty(), "{infraction:?} in {lang:?}");
            }
        }
        assert_eq!(
            Lang::En.infraction(Infraction::Obstruction),
            Some("Obstruction")
        );
        assert_eq!(
            Lang::Es.infraction(Infraction::FalseStart),
            Some("Saque nulo")
        );
    }

    #[test]
    fn english_is_unchanged() {
        let en = Lang::En;
        assert_eq!(en.goal(), "GOAL");
        assert_eq!(en.referee_timeout(), ("REFEREE", "TIMEOUT"));
        assert_eq!(en.team_timeout(true), ("BLACK", "TIMEOUT"));
        assert_eq!(en.penalty_shot(), ("PENALTY", "SHOT"));
        assert_eq!(en.period(GamePeriod::FirstHalf), "1ST HALF");
        assert_eq!(en.period(GamePeriod::HalfTime), "HALF TIME");
        assert_eq!(en.period(GamePeriod::PreOvertime), "PRE OVERTIME");
        assert_eq!(en.next_game(), "NEXT GAME");
    }

    #[test]
    fn default_team_names_follow_the_language() {
        assert_eq!(Lang::Es.default_team_name("BLACK"), Some("NEGRO"));
        assert_eq!(Lang::En.default_team_name("BLANC"), Some("WHITE"));
        assert_eq!(Lang::Fr.default_team_name("Atlantis"), None);
    }
}
