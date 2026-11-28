//! The overlay's fixed words in English, Spanish and French (the three CMAS languages).
//!
//! Plain tables rather than a translation library: the overlay has about 25 words, and a `match`
//! per word makes the compiler insist that every word has all three languages. Team names, player
//! names and anything else from the schedule or the portal are never translated.
//!
//! The Spanish and French start from the refbox's own translations (`refbox/translations/es` and
//! `fr`). Words are written out in full: where one is longer than the English, its box is drawn
//! wider (see `pages::widen_for`) rather than the word being shortened or drawn smaller.

use log::warn;
use serde::{Deserialize, Deserializer, Serialize};
use uwh_common::game_snapshot::{GamePeriod, Infraction};

/// A language the overlay can show, and the overlay's own `language` setting (`"en"`, `"es"` or
/// `"fr"`; English by default). The setting belongs to the overlay alone: the refbox's language
/// never changes the broadcast. Written as those short codes; read leniently (see the
/// `Deserialize` impl).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum Lang {
    #[default]
    #[serde(rename = "en")]
    En,
    #[serde(rename = "es")]
    Es,
    #[serde(rename = "fr")]
    Fr,
}

/// Reads the setting from any text, so one mistyped value never stops the settings file loading:
/// case and surrounding spaces are ignored, a regional code such as `"es-ES"` means its language,
/// and anything else means English, with a warning naming it.
impl<'de> Deserialize<'de> for Lang {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let code = text.trim().to_lowercase();
        let primary = code.split(['-', '_']).next().unwrap_or_default();
        Ok(match primary {
            "en" => Self::En,
            "es" => Self::Es,
            "fr" => Self::Fr,
            _ => {
                warn!(
                    "Unknown overlay language setting {text:?}: using English. Use \"en\", \"es\" or \"fr\"."
                );
                Self::En
            }
        })
    }
}

impl Lang {
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

    /// The penalty card's tab for a total dismissal: the referees' own short form, "TD" in
    /// English and Spanish and "ED" (expulsion définitive) in French. The organiser ruled that
    /// these stay short.
    pub const fn total_dismissal(self) -> &'static str {
        self.of("TD", "TD", "ED")
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
            Self::Fr => ("TEMPS MORT", "ARBITRES"),
        }
    }

    /// The two lines of a team timeout flag (`black`: the Black team's).
    pub const fn team_timeout(self, black: bool) -> (&'static str, &'static str) {
        let team = if black { self.black() } else { self.white() };
        match self {
            Self::En => (team, "TIMEOUT"),
            Self::Es => ("TIEMPO MUERTO", team),
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
                self.of("OVERTIME 1ST HALF", "PRÓRROGA 1ª MITAD", "PROLONGATION 1")
            }
            GamePeriod::OvertimeSecondHalf => {
                self.of("OVERTIME 2ND HALF", "PRÓRROGA 2ª MITAD", "PROLONGATION 2")
            }
            GamePeriod::OvertimeHalfTime => self.of(
                "OVERTIME HALF TIME",
                "DESCANSO PRÓRROGA",
                "MI-TEMPS PROLONGATION",
            ),
            GamePeriod::PreOvertime => {
                self.of("PRE OVERTIME", "PRE PRÓRROGA", "AVANT PROLONGATION")
            }
            GamePeriod::SuddenDeath => self.of("SUDDEN DEATH", "MUERTE SÚBITA", "MORT SUBITE"),
            GamePeriod::PreSuddenDeath => {
                self.of("PRE SUDDEN DEATH", "PRE MUERTE SÚBITA", "AVANT MORT SUBITE")
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
            Infraction::IllegalAdvancement => ("Avance ilegal", "Avancée illégale du palet"),
            Infraction::IllegalSubstitution => ("Sustitución ilegal", "Changement incorrect"),
            Infraction::IllegallyStoppingThePuck => ("Parada ilegal", "Arrêt irrégulier du palet"),
            Infraction::OutOfBounds => ("Pastilla fuera", "Sortie du palet"),
            Infraction::GrabbingTheBarrier => {
                ("Agarrarse a la barrera", "S'agripper à la barrière")
            }
            Infraction::Obstruction => ("Obstrucción", "Obstruction"),
            Infraction::DelayOfGame => ("Demorar el juego", "Retarder le jeu"),
            Infraction::UnsportsmanlikeConduct => {
                ("Conducta antideportiva", "Conduite antisportive")
            }
            Infraction::FreeArm => (
                "Uso ilegal del brazo libre",
                "Usage irrégulier du bras libre",
            ),
            Infraction::FalseStart => ("Salida en falso", "Faux départ"),
        };
        Some(self.of(infraction.short_name(), es, fr))
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
    fn the_setting_reads_and_writes_as_short_codes() {
        #[derive(Serialize, Deserialize)]
        struct S {
            language: Lang,
        }
        for (setting, text) in [(Lang::En, "en"), (Lang::Es, "es"), (Lang::Fr, "fr")] {
            let line = serde_json::to_string(&S { language: setting }).expect("serialize");
            assert_eq!(line, format!("{{\"language\":\"{text}\"}}"));
            let back: S = serde_json::from_str(&line).expect("deserialize");
            assert_eq!(back.language, setting);
        }
    }

    #[test]
    fn the_language_setting_is_read_leniently() {
        #[derive(Deserialize)]
        struct S {
            language: Lang,
        }
        for (text, setting) in [
            ("ES", Lang::Es),
            ("es-ES", Lang::Es),
            ("Fr", Lang::Fr),
            (" es ", Lang::Es),
            ("fr_CA", Lang::Fr),
            ("follow-refbox", Lang::En),
            ("spanish", Lang::En),
        ] {
            let read: S = serde_json::from_str(&format!("{{\"language\":\"{text}\"}}"))
                .expect("any language text loads");
            assert_eq!(read.language, setting, "{text:?}");
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
            Some("Salida en falso")
        );
    }

    #[test]
    fn no_spanish_or_french_word_is_cut_short() {
        use GamePeriod::*;
        for lang in [Lang::Es, Lang::Fr] {
            let (referee_1, referee_2) = lang.referee_timeout();
            let (shot_1, shot_2) = lang.penalty_shot();
            let mut words = vec![
                lang.black(),
                lang.white(),
                lang.game(),
                lang.next_game(),
                lang.referees(),
                lang.goal(),
                lang.foul(),
                lang.total_dismissal(),
                referee_1,
                referee_2,
                shot_1,
                shot_2,
            ];
            for black in [true, false] {
                let (line_1, line_2) = lang.team_timeout(black);
                words.extend([line_1, line_2]);
            }
            for period in [
                FirstHalf,
                SecondHalf,
                HalfTime,
                BetweenGames,
                PreOvertime,
                OvertimeFirstHalf,
                OvertimeHalfTime,
                OvertimeSecondHalf,
                PreSuddenDeath,
                SuddenDeath,
            ] {
                words.push(lang.period(period));
            }
            for word in words {
                assert!(!word.contains('.'), "{word:?} in {lang:?} is cut short");
            }
        }
        assert_eq!(Lang::Es.team_timeout(true), ("TIEMPO MUERTO", "NEGRO"));
        assert_eq!(Lang::Fr.total_dismissal(), "ED");
        assert_eq!(Lang::Es.period(PreSuddenDeath), "PRE MUERTE SÚBITA");
        assert_eq!(Lang::Fr.period(OvertimeFirstHalf), "PROLONGATION 1");
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
}
