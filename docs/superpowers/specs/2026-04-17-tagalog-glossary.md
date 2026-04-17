# Tagalog/Filipino UWH Glossary — First Pass

**Date:** 2026-04-17
**Purpose:** Vet UWH-specific Tagalog/Filipino terms before regenerating tl-PH/refbox.ftl.
**Review status:** Pending human review. Native-speaker review is a separate, later step.

---

## Important notes before using this glossary

1. **Philippine UWH operates entirely in English.** The Philippine Underwater Hockey
   Confederation (PUHC/PUHC) uses English-language rules (CMAS international rules) and
   conducts tournaments in English. No Tagalog-language rulebook was found. This is the
   expected situation — the Philippines is the only country in this project's locale set where
   English is a co-official language and the sport's operational language.

2. **Keep English loanwords where the community uses them.** Filipinos routinely code-switch in
   sports contexts. Terms like "penalty," "foul," "referee," "timeout," and "overtime" are
   used in English by Filipino UWH players, exactly as they appear in English basketball,
   volleyball, and swimming coverage. Forcing a Tagalog coinage where none exists would produce
   text that sounds foreign to Filipino players. Where the community uses English, this glossary
   recommends keeping it.

3. **The existing tl-PH translation was consulted as a prior-art source.** The file at
   `refbox/translations/tl-PH/refbox.ftl` contains choices already made by whoever produced
   the original translation. Rows below note where the existing choice is confirmed good,
   needs adjustment, or is a concern.

---

## Source tags

- `[FED-RULEBOOK]` — federation rulebook (cite URL/section); no Tagalog-language rulebook exists
- `[WEB-UWH-CLUB]` — Philippine UWH club/team page (cite URL)
- `[WIKIPEDIA-TL]` — Tagalog Wikipedia (tl.wikipedia.org)
- `[TAGALOG-DICT]` — standard Tagalog dictionary or well-attested reference (cite URL)
- `[BEST-GUESS]` — no direct source; proposed from general Tagalog/Filipino sports vocabulary

---

## Sources consulted

- https://en.wikipedia.org/wiki/Philippine_Underwater_Hockey_Confederation — English Wikipedia
  article on the PUHC. Confirmed: the confederation uses English-language rules (CMAS), no
  Tagalog rulebook exists. Confirmed team/club names and history.

- https://pilipinasuwh.com — Official PUHC website (URL confirmed via search; direct fetch was
  blocked by permissions). The site is confirmed to exist but content was not directly
  extractable. Google snippets confirmed the site is in English.

- https://www.facebook.com/pilipinasuwh/ — PUHC Facebook page. Confirmed via search snippet:
  all posts are in English. No Tagalog UWH vocabulary found.

- https://swimtravelph.com/underwaterhockey/ — Philippine dive-travel site with an "Learn to
  play Underwater Hockey" page. Confirmed via search snippet: English-language rules summary.
  Confirmed use of "foul," "referee," "penalty," "goal," "half" in English.

- https://www.rappler.com/sports/by-sport/other-sports/136206-philippine-underwater-hockey-team-quietly-brings-pride
  — Rappler article on the Philippine UWH team. English throughout; confirmed that Filipino UWH
  players use English sports vocabulary in media coverage.

- https://tl.wikipedia.org — Tagalog Wikipedia searched for "underwater hockey" and related
  terms. No Tagalog Wikipedia article on underwater hockey was found. The sport does not appear
  to be documented there.

- https://www.tagalog.com — Tagalog dictionary reference. Used to confirm: "iskor" (score),
  "manlalaro" (player), "koponan" (team), "tagahatol" (referee/judge), "babala" (warning),
  "pahinga" (rest/break/timeout), "harang" (obstruction/block), "hadlang" (obstruction),
  "hawak" (hold/grip), "kapit" (grip/grab).

- https://www.tagaloglang.com — Secondary Tagalog dictionary reference. Confirmed: "babala"
  (warning), "hadlang" (obstacle/obstruction).

- https://www.wordhippo.com — Word lookup used for: "expulsion" → pagpapaalis/pagpapalabas;
  "false start" → maling simula; "pause/timeout" → tigil, pahinga, sandaling huminto.

- https://fluentfilipino.com/filipino-language-and-sports/ — General Filipino sports vocabulary
  reference. Confirmed: "iskor" (score), "manlalaro" (player), "koponan" (team), "tagahatol"
  (referee), "puntos" (points).

- https://tagalogjourney.com/vocabulary/tagalog-vocabulary-for-discussing-different-sports-tournaments/
  — Tagalog sports tournament vocabulary. Confirmed general sports terms.

- https://mymemory.translated.net/en/English/Tagalog/half-time — Translation reference.
  Confirmed "kalahati" as the translation of "half time."

- https://www.englishtotagalog.org/meaning/overtime-in-tagalog — Confirmed "overtime" is used
  as-is in Tagalog basketball contexts; no established Tagalog coinage.

- https://www.researchgate.net/publication/356580278_Tagalog-Flavored_Basketball — Academic
  paper on Tagalog basketball terminology. Confirmed "paglabag" (foul/violation) in basketball
  context.

- https://pinoysolohiker.blogspot.com/2018/08/pinoy-basketball-terminologies.html — Pinoy
  basketball terminology list. Confirmed: "paglabag" (foul), various slang terms.

- https://ccgit.crown.edu/cyber-reels/osc-sports-terms-a-tagalog-glossary-1767648840 — OSC
  Sports Terms Tagalog glossary (Oregon-based reference). Confirmed general sports terms in
  Tagalog but not UWH-specific terms.

- https://www.wordhippo.com/what-is/the/filipino-word-for-a8ab847aaaa844d4aa6891c5bb26547d719d89ab.html
  — "False start" in Filipino confirmed as "maling simula."

---

## Glossary (40 terms)

Rows cover every UWH-specific or UI concept in `en-US/refbox.ftl` that merits vetting.
The "Existing tl-PH" column shows what the current translation file already has (prior art).
The "Recommended" column is this glossary's vetted recommendation.

| English (from en-US/refbox.ftl) | Recommended Tagalog/Filipino | Existing tl-PH | Source | Notes |
|---|---|---|---|---|
| Black team | Itim na koponan | Itim / ITIM | [TAGALOG-DICT] | "Itim" = black in Tagalog; "koponan" = team (standard in sports, e.g., "Pambansang koponan"). Scoreboard short form: "ITIM". The existing file uses ITIM throughout — confirmed correct. |
| White team | Puting koponan | Puti / PUTI | [TAGALOG-DICT] | "Puti" = white. Scoreboard short form: "PUTI". Existing file confirmed correct. |
| Score | Puntos | PUNTOS | [TAGALOG-DICT] | Human reviewer selected "Puntos" (Spanish-derived, widely used in Filipino sports commentary — especially basketball) over "Iskor" as the more natural choice. |
| Confirm (score) | Kumpirmahin | KUMPIRMAHIN | [TAGALOG-DICT] | Standard Filipino loanword from "confirm." Existing file uses this correctly. |
| Foul (general) | Poul / foul | POUL / MGA POUL | [WEB-UWH-CLUB] | Philippine UWH operates in English; "foul" is used directly. The existing tl-PH file uses "poul" (Tagalog-ized spelling). Recommendation: keep "poul" as it is already Filipinized and consistent, but note that Filipino sports media also uses "paglabag" (violation) in basketball. For UWH context, "poul" is the safer loanword choice. |
| Warning (caution) | Babala | BABALA | [TAGALOG-DICT] | "Babala" = warning, caution. Well-attested Tagalog word. Existing file uses this consistently — confirmed correct. |
| Penalty (time-served) | Penalty / parusa | MGA PARUSA | [WEB-UWH-CLUB] | Philippine UWH uses English "penalty" in play. The existing file uses "parusa" (Tagalog for punishment/penalty). Both are defensible; "parusa" is more Tagalog but "penalty" would be more familiar to Filipino UWH players. Recommend keeping "parusa" for UI labels but flagging for native-speaker review. |
| 30-second penalty | 30s | 30s | [BEST-GUESS] | Abbreviated form used in all languages. No Tagalog equivalent needed for the scoreboard abbreviation. |
| 1-minute penalty | 1m | 1m | [BEST-GUESS] | Same as above — universal abbreviated form. |
| 2-minute penalty | 2m | 2m | [BEST-GUESS] | Same. |
| 4-minute penalty | 4m | 4m | [BEST-GUESS] | Same. |
| 5-minute penalty | 5m | 5m | [BEST-GUESS] | Same. |
| Total dismissal | TD | TD | [WEB-UWH-CLUB] | The existing file keeps "TD" — correct, as this is the universal UWH abbreviation. In narrative text, the existing file uses "Tinanggal" (removed/dismissed). "Pagpapaalis" or "pagpapalabas" are the closest Tagalog dictionary forms for "expulsion." Recommend keeping "TD" as the abbreviation and "Tinanggal" for narrative. |
| First half | Unang kalahati | UNANG KALAHATI | [TAGALOG-DICT] | "Una" = first; "kalahati" = half. Existing file confirmed correct. |
| Second half | Ikalawang kalahati | IKALAWANG KALAHATI | [TAGALOG-DICT] | "Ikalawa" = second; "kalahati" = half. Confirmed correct. |
| Half time (break between halves) | Pahinga | PAHINGA | [TAGALOG-DICT] | "Pahinga" = rest/break. The existing file uses this for half-time. Accurate and natural. Note: "pahinga" is also used for "timeout" in the existing file, which creates a naming collision — flagged below. |
| Timeout (team/referee pause) | Timeout / pahinga | PAHINGA | [WEB-UWH-CLUB] | **Concern:** The existing file uses "pahinga" for both "half time" and "timeout." This creates ambiguity. In Filipino sports, "timeout" is almost universally kept in English (basketball, volleyball). Recommendation: use "timeout" as the display term for team/referee timeouts, reserving "pahinga" for half-time breaks. Flag for native-speaker review. |
| Team timeout | Timeout ng koponan | PAHINGA NG KOPONAN | [WEB-UWH-CLUB] | See above — "timeout" is better than "pahinga" here to avoid confusion with half-time. |
| Referee timeout | Timeout ng referee | PAHINGA REF | [WEB-UWH-CLUB] | Same concern — keeping "timeout" in English is cleaner. |
| Between games (nominal break) | Sa pagitan ng mga laro | PAHINGA SA PAGITAN NG MGA LARO | [TAGALOG-DICT] | "Sa pagitan ng" = between; "mga laro" = games. Existing file is accurate. |
| Overtime | Overtime | OVERTIME | [WEB-UWH-CLUB] | No established Tagalog word for overtime. English "overtime" is used universally in Filipino sports (basketball, volleyball). Existing file correctly keeps "overtime." |
| Overtime first half | Unang kalahati ng OT | UNANG KALAHATI NG OT | [TAGALOG-DICT] | Existing file confirmed correct. |
| Overtime half time | Pahinga sa OT | PAHINGA SA OT | [TAGALOG-DICT] | Existing file confirmed correct (same word-collision caveat as "half time" above). |
| Overtime second half | Ikalawang kalahati ng OT | IKALAWANG KALAHATI NG OT | [TAGALOG-DICT] | Existing file confirmed correct. |
| Sudden death | Sudden death | SUDDEN DEATH | [WEB-UWH-CLUB] | No established Tagalog term. The literal translation "biglaang kamatayan" or "biglang pagkamatay" does not appear to be used in Filipino sports broadcasts. English "sudden death" is kept in the existing file — confirmed correct. |
| Penalty shot | Penalty shot | PENALTY SHOT | [WEB-UWH-CLUB] | Philippine UWH uses "penalty shot" in English. Existing file correctly keeps this in English. |
| Referee (generic) | Referee / tagahatol | REF | [TAGALOG-DICT] | "Tagahatol" is the formal Tagalog word for referee/judge. "Referee" is universally used in Filipino sports media. The existing file uses "REF" as an abbreviation — correct. For full labels, "referee" (English loanword) is appropriate given community usage. |
| Chief referee | Punong referee | Punong Ref | [TAGALOG-DICT] | "Punong" = chief/head (from "puno" = head). Used in Philippine government and sports contexts (e.g., "punong tagahatol"). Existing file uses "Punong Ref" — confirmed correct and natural. |
| Water referee | Water ref | Water Ref | [WEB-UWH-CLUB] | No Tagalog term found. Philippine UWH uses "water ref" in English. Existing file confirmed correct. |
| Timekeeper / Timer | Timer | Timer | [WEB-UWH-CLUB] | No Tagalog term found in Philippine sports. "Timer" is universal. Existing file confirmed correct. |
| Scorer / Score-keeper | Tagapanatili ng puntos | Tagapanatili ng Puntos | [TAGALOG-DICT] | "Tagapanatili" = keeper/maintainer; "puntos" = score (per human reviewer's decision above). |
| Cap number (player's cap/helmet number) | Numero ng gorro | NUMERO NG GORRO | [BEST-GUESS] | "Numero" = number (loanword, standard in Filipino); "gorro" = cap/hat (Spanish loanword, widely used in Filipino). The existing file uses "NUMERO NG GORRO NG MANANAKAY" (cap number of the scorer) — "mananakay" appears to be a translation error (it means "rider/jockey"), not a swimmer or hockey player. Recommend "numero ng gorro ng manlalaro" (player's cap number) or simply "numero ng gorro." Flag for native-speaker review. |
| Player | Manlalaro | MANLALARO | [TAGALOG-DICT] | Standard Tagalog sports word — confirmed across basketball, volleyball, and general sports. Existing file correct. |
| Stick foul | Poul sa stick | Poul sa Stick | [BEST-GUESS] | No Tagalog term for "stick foul" found in any Philippine UWH source. "Stick" is kept in English (the stick is called "stick" in Philippine UWH). "Poul sa stick" is a natural Tagalog-English mix consistent with how Filipino players discuss the game. Existing file confirmed. |
| Illegal advance | Iligal na pag-abante | Iligal na Pag-abante | [BEST-GUESS] | No Philippine UWH source uses a Tagalog term. "Iligal" = illegal (loanword); "pag-abante" = advancing/moving forward (from "abante"). Existing file confirmed. |
| Sub foul (substitution foul) | Poul sa pagpapalit | Poul sa Pagpapalit | [BEST-GUESS] | "Pagpapalit" = substitution/replacement. Natural construction. Existing file confirmed. |
| Illegal stoppage | Iligal na paghinto | Iligal na Paghinto | [BEST-GUESS] | "Paghinto" = stopping. Natural construction. Existing file confirmed. |
| Out of bounds | Labas ng hangganan | Labas ng Hangganan | [TAGALOG-DICT] | "Labas" = outside; "hangganan" = boundary/limit. Confirmed: "labas sa hangganan" and "labas ng hangganan" are both used in Filipino basketball for out-of-bounds. Existing file uses "Labas ng Hangganan" — confirmed correct. |
| Grabbing the wall | Pagkuha sa dingding | Pagkuha sa Dingding | [BEST-GUESS] | "Pagkuha" = grabbing/taking; "dingding" = wall. No UWH-specific source. "Pagkakamit ng pader" or "pagkapit sa dingding" (gripping the wall) would also work — "pagkapit" (grip/hold) is arguably more precise for the UWH infraction (holding on to the pool wall). Existing file uses "pagkuha" (taking/grabbing). Recommend considering "pagkapit sa dingding" as an alternative. Flag for native-speaker review. |
| Obstruction | Hadlang | Hadlang | [TAGALOG-DICT] | "Hadlang" = obstacle/obstruction. Well-attested Tagalog word. Synonym: "harang" (also used in basketball for "block"). Existing file uses "hadlang" — confirmed correct for obstruction (as distinct from a block shot). |
| Delay of game | Pagkaantala ng laro | Pagkaantala ng Laro | [TAGALOG-DICT] | "Pagkaantala" = delay/postponement; "laro" = game. Standard Tagalog construction. Existing file confirmed correct. |
| Unsportsmanlike conduct | Hindi sportsmanlike / Di-sportsmanlike | Hindi Sportsmanlike | [BEST-GUESS] | No established Tagalog phrase found. "Hindi sportsmanlike" (not sportsmanlike) is the most common approach in Filipino sports media code-switching. Alternatives: "laban sa sportsmanship" or "walang sportsmanship." Existing file uses "Hindi Sportsmanlike" — acceptable but flagged for native-speaker review since a more Tagalog construction may exist. |
| Free arm | Libreng braso | Libreng Braso | [BEST-GUESS] | "Libreng" = free; "braso" = arm (Spanish loanword, standard Filipino). No Philippine UWH source uses a specific Tagalog term. Existing file confirmed. |
| False start | Maling simula | Maling Simula | [TAGALOG-DICT] | Confirmed: "maling simula" is the standard Filipino translation of "false start" (attested for swimming and track). Existing file confirmed correct. |

---

## Terms used without change from English (confirmed community usage)

The following terms appear in the refbox UI and are best kept in English for the tl-PH locale,
because the Philippine UWH community uses them in English and no established Tagalog equivalent
exists:

| English term | Reason to keep in English |
|---|---|
| Penalty | Universal in Philippine sports; "penalty" appears in basketball, football, UWH |
| Foul | Used in English by Philippine UWH; "poul" is an acceptable Filipinized spelling |
| Referee / Ref | Used in English across all Philippine sports |
| Timeout | Used in English in basketball, volleyball, UWH; avoids collision with "pahinga" |
| Overtime | No Tagalog equivalent; used in English universally |
| Sudden death | No Tagalog sports equivalent in common use |
| Penalty shot | UWH-specific; used in English |
| OT (abbreviation) | Universal short form |
| TD (total dismissal abbreviation) | Universal UWH abbreviation |

---

## Flags for native-speaker review

These items in the existing tl-PH translation are potential concerns:

1. **"pahinga" used for both half-time and timeout** — creates ambiguity. Consider "timeout"
   for the player/referee pause, "pahinga" only for the between-halves break.

2. **"mananakay" in `track-cap-number-of-scorer`** — "mananakay" means rider/jockey (as in
   horse racing), not a hockey player. Likely auto-translation error. Should be "manlalaro"
   (player) or "mananakay" replaced entirely.

3. **"pagkuha sa dingding"** — "pagkuha" is more "taking/fetching." "Pagkapit sa dingding"
   (gripping the wall) may be more precise for the UWH infraction.

4. **"parusa" vs "penalty"** — worth asking a native Filipino UWH player which term they
   actually use when discussing the penalty box.

5. **"poul" spelling** — the Filipinized spelling of "foul." Some sources use "foul" directly
   in code-switching. Either is defensible; consistency matters more than the choice.
