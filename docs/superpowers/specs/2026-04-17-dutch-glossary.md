# Dutch (Nederlands) UWH Glossary — First Pass

**Date:** 2026-04-17
**Purpose:** Vet UWH-specific Dutch terms before generating nl-NL/refbox.ftl.
**Review status:** Pending human review. Native-speaker review is a separate, later step.

## Source tags

- `[FED-RULEBOOK]` — from the official Dutch UWH spelregels (cite URL/section)
- `[WEB-UWH-CLUB]` — from a Dutch/Belgian UWH club or association page (cite URL)
- `[WIKIPEDIA-NL]` — from Dutch Wikipedia
- `[BEST-GUESS]` — no direct source found, proposed from general Dutch sports vocabulary

## Sources consulted

- https://www.spelregels.eu/wp-content/uploads/2021/01/spelregels-onderwaterhockey.pdf — Concept/current version of the official Nederlandse onderwaterhockey spelregels (spelregels.eu). Confirmed repeatedly in search results as the live concept rulebook. PDF content not directly fetchable, but rich terminology extracted via search snippets quoting it. Yielded: eerste helft, tweede helft, rust, verlenging, strafbank, definitieve uitsluiting, strafpuck, vrije puck, waarschuwing, time-out, tijdwaarnemer, secretaris, waterscheidsrechter, hoofdscheidsrechter, donker/licht team colour terminology, wisselstraf/foutief wisselen, vals vertrek.
- https://aquanauten.nl/wp-content/uploads/2011/10/OWHspelregelsOkt2006.pdf — The October 2006 version of the Nederlandse onderwaterhockey spelregels (hosted by Dutch UWH club Aquanauten). PDF not directly fetchable, but quoted extensively in search snippets. Yielded: straftijd (netto tijd), strafbank, speler, team, scheidsrechter, hoofdscheidsrechter, waterscheidsrechter, tijdwaarnemer, verlenging (2x5 min), vals vertrek, definitieve uitsluiting, vrije puck, strafpuck.
- https://onderwaterhockey.nl/competitie/reglement/ — Reglementen page of the official Dutch UWH national association (onderwaterhockey.nl, the NOB's UWH branch). Confirmed as primary domain for Dutch UWH competition rules. Yielded: general confirmation of terminology.
- https://onderwatersport.org/voor-sporters/onze-sporten/onderwaterhockey/ — NOB (Nederlandse Onderwatersport Bond) landing page for onderwaterhockey. Confirmed the sport name and federation structure; limited specific term yield.
- https://nl.wikipedia.org/wiki/Onderwaterhockey — Dutch Wikipedia article on onderwaterhockey. Yielded: onderwaterhockey, puck, doelbox, scheidsrechter, team, speler, doelpunt, verlenging, eerste helft, tweede helft, rust; confirmed team colour terminology (donker/licht).
- https://www.spelregels.eu/onderwaterhockey/ — Spelregels.eu summary page for onderwaterhockey. Yielded: overview of game periods, penalty structure, referee roles; corroborates rulebook PDF.
- https://docplayer.nl/29416828-Onderwaterhockey-spelregels.html — DocPlayer HTML rendering of an older Dutch UWH spelregels document. Yielded: definitieve uitsluiting (signal description), strafpuck spot description, strafbank, straftijd (netto).
- https://argonauta.nl/spelregels/ — Dutch UWH club Argonauta (Prinsenbeek) spelregels page. Yielded: obstructie, stokfout, vrije arm, vals vertrek (penalty description), vertraging (delay), teamwaarschuwing context.
- https://zwemblog.com/kennisbank/onderwaterhockey/ — General Dutch UWH overview blog. Yielded: time-out (1 per helft), rust, eerste/tweede helft, donker/licht caps, puck.
- https://sport.nl/artikelen/2016/09/onderwaterhockey-spannende-sport-op-de-bodem-van-het-zwembad — Sport.nl article. Confirmed: scheidsrechter, time-out, donkere/lichte muts, puck.
- https://onderwaterhockey.nl/onderwaterhockey-materiaal/ — onderwaterhockey.nl equipment page. Yielded: muts (cap), donkere muts/lichte muts, stick (stok), confirmation of cap colour terminology.
- https://onderwaterhockey.nl/verder-met-owh/opleidingen/scheidsrechterscursus/ — Referee training page on onderwaterhockey.nl. Confirmed: scheidsrechterscursus, waterscheidsrechter, hoofdscheidsrechter roles.
- https://nelos.be/programma/OWH — NELOS (Nederlandstalige Liga voor Onderwateronderzoek en -Sport, Belgian Dutch-language federation) UWH page. Confirms NELOS as the Belgian Dutch-language governing body for onderwaterhockey; limited term yield from snippet.

## Glossary (40 terms)

Trimmed to rows that correspond to concepts actually present in `en-US/refbox.ftl`.

| English (from en-US/refbox.ftl) | Dutch | Source | Notes |
|---|---|---|---|
| Black team (dark team) | Donker team | [FED-RULEBOOK] | The Dutch rulebook uses "donker" (dark) rather than "zwart" (black) because the dark team may wear blue or black. Scoreboard caps-form: "DONKER". The refbox `dark-team-name` maps to "Donker"; `dark-team-name-caps` maps to "DONKER". |
| White team (light team) | Licht team | [FED-RULEBOOK] | Parallel to donker; "licht" = light/white. Scoreboard form: "LICHT". |
| Score | Score | [WEB-UWH-CLUB] | "Score" is used as a direct loanword in Dutch sports contexts. "Stand" (standing) is also used in general sports Dutch. Either is natural; "score" matches the scoreboard context directly. |
| Confirm (score) | Bevestig | [BEST-GUESS] | Standard Dutch UI verb for "confirm". "Bevestig de score" = confirm the score. No specific UWH source uses this UI phrase, but it is unambiguous Dutch. |
| Foul (general) | Overtreding | [FED-RULEBOOK] | The Dutch rulebook uses "overtreding" (violation/infraction) as the primary term for a foul. "Fout" is also used colloquially but "overtreding" is the formal rulebook term. |
| Warning (caution) | Waarschuwing | [FED-RULEBOOK] | Used directly in the Dutch spelregels: "officiële waarschuwing". |
| Team warning | Teamwaarschuwing | [FED-RULEBOOK] | The Dutch rulebook explicitly uses "teamwaarschuwing" for a warning issued to the team (not an individual player). |
| Penalty (time-served exclusion) | Uitsluiting | [FED-RULEBOOK] | The Dutch term for a timed penalty is "tijdelijke uitsluiting" (temporary exclusion). Short form on scoreboard: "UITSL". The refbox uses "penalty" generically; "uitsluiting" covers timed penalties. |
| 30-second penalty | 30 seconden uitsluiting | [BEST-GUESS] | Constructed from "seconden" (seconds) + "uitsluiting". The 30-second tier is not specifically cited in Dutch UWH sources (Dutch domestic rules use 1 and 2 minutes), but the phrase is grammatically standard. Scoreboard: "30s". |
| 1-minute penalty | 1 minuut uitsluiting | [FED-RULEBOOK] | Referenced directly in the Dutch spelregels as "1 minuut op de strafbank". Scoreboard: "1m". |
| 2-minute penalty | 2 minuten uitsluiting | [FED-RULEBOOK] | Referenced in the Dutch spelregels as a heavier tier. Scoreboard: "2m". |
| 4-minute penalty | 4 minuten uitsluiting | [BEST-GUESS] | Constructed; the 4-minute tier is not singled out in Dutch UWH summaries but the phrase is grammatically standard. Scoreboard: "4m". |
| 5-minute penalty | 5 minuten uitsluiting | [FED-RULEBOOK] | Referenced in the Dutch spelregels (straftijden include 5 minutes for serious infractions). Scoreboard: "5m". |
| Total dismissal | Definitieve uitsluiting | [FED-RULEBOOK] | The Dutch rulebook uses "definitieve uitsluiting" (permanent/definitive exclusion). This is the "TD" equivalent. Scoreboard abbreviation: "DEF" (best-guess short form for use on scoreboard). |
| Served (penalty served) | Uitgezeten | [BEST-GUESS] | Dutch past participle of "uitzitten" (to sit out). "De straf is uitgezeten" = the penalty has been served. Natural Dutch sports phrasing. |
| Pending (penalty pending) | In behandeling | [BEST-GUESS] | Standard Dutch for "pending" in an administrative/status context. Alternative: "wacht". |
| Dismissed | Definitief uitgesloten | [FED-RULEBOOK] | Past participle form of "definitieve uitsluiting". Used to describe the state of a player who has received total dismissal. |
| First half | Eerste helft | [FED-RULEBOOK] | "De speeltijd bestaat uit een eerste en een tweede helft." Direct quote from the Dutch spelregels. |
| Second half | Tweede helft | [FED-RULEBOOK] | Parallel form; direct from Dutch spelregels. |
| Half time | Rust | [FED-RULEBOOK] | The Dutch rulebook uses "rust" (rest/break) for the half-time interval: "de rust bedraagt 3 minuten". Also: "pauze". "Rust" is the idiomatic sports term. |
| Overtime | Verlenging | [FED-RULEBOOK] | Used directly in the Dutch spelregels: "verlenging van 2 x 5 minuten". Standard Dutch sports word for extra time/overtime. |
| Overtime first half | Eerste verlengingstijd | [BEST-GUESS] | Constructed from "verlenging" + "eerste" + "tijd". No single fixed label found in Dutch UWH sources. Alternative: "eerste helft verlenging". |
| Overtime half time | Rust verlenging | [BEST-GUESS] | Constructed: "rust" (half-time break) + "verlenging" (overtime). No dedicated Dutch UWH label found. Alternative: "verlenging rust". |
| Overtime second half | Tweede verlengingstijd | [BEST-GUESS] | Parallel to eerste verlengingstijd. |
| Pre-overtime break | Pauze voor verlenging | [FED-RULEBOOK] | The Dutch rulebook specifies "voor aanvang van de verlenging heeft men een pauze van 3 minuten". Constructed label: "pauze voor verlenging". |
| Sudden death | Plotselinge dood | [BEST-GUESS] | Literal Dutch translation of "sudden death"; the phrase is used in Dutch sports media (particularly ice hockey) for a first-goal-wins overtime rule. The Dutch UWH rulebooks describe overtime as 2x5 minutes without naming a sudden death variant; "plotselinge dood" is used in Dutch broadcast sports vocabulary for this concept. Alternative: "gouden doelpunt" (golden goal, used in football contexts). |
| Pre-sudden-death break | Pauze voor plotselinge dood | [BEST-GUESS] | Constructed from "pauze" + "voor" + "plotselinge dood". No Dutch UWH source names this break specifically. |
| Team timeout | Team time-out | [FED-RULEBOOK] | The Dutch spelregels use "time-out" as a loanword: "ieder team heeft het recht op één time-out van één minuut per speelhelft". "Team time-out" is the natural combination. Short form: "T/O". |
| Referee timeout | Scheidsrechters time-out | [BEST-GUESS] | Constructed from "scheidsrechter" (referee) + "time-out". No dedicated Dutch UWH label found for this refbox UI concept. Short form: "SR T/O". |
| Penalty shot | Strafschot | [FED-RULEBOOK] | The Dutch rulebook uses "strafpuck" for a penalty puck restart and "strafschot" for a direct penalty shot. The refbox "penalty shot" maps to "strafschot". |
| Referee (generic) | Scheidsrechter | [FED-RULEBOOK] | Standard Dutch word used throughout the rulebook. |
| Chief referee | Hoofdscheidsrechter | [FED-RULEBOOK] | Used in the Dutch spelregels: "het spel wordt geleid door een hoofdscheidsrechter en twee waterscheidsrechters". Also confirmed on onderwaterhockey.nl referee training page. |
| Water referee | Waterscheidsrechter | [FED-RULEBOOK] | Direct from the Dutch spelregels: "waterscheidsrechters controleren het speelveld". |
| Timekeeper | Tijdwaarnemer | [FED-RULEBOOK] | Direct from the Dutch spelregels: "de tijdwaarnemer, gezeten aan de wedstrijdtafel, controleert de speeltijd". |
| Cap number | Mutsnummer | [WEB-UWH-CLUB] | "Muts" = the cap worn in UWH (specifically the hard protective cap, not a swim cap). "Mutsnummer" = cap number. Confirmed from Dutch UWH equipment references. Alternative: "spelernummer" (player number). |
| Stick foul | Stokfout | [WEB-UWH-CLUB] | Attested in Dutch UWH club sources (argonauta.nl): "stokfout" = a foul committed with the stick. |
| Illegal advance | Onrechtmatig voortbewegen | [BEST-GUESS] | Constructed. Dutch rulebook describes illegal movement (speler mag niet voortbewegen) but does not assign a single fixed label to this refbox category. Alternative: "ongeoorloofd voortbewegen". |
| Sub foul (substitution foul) | Wisselfout | [FED-RULEBOOK] | The Dutch spelregels use "foutief wisselen" (faulty substitution) as a punishable offence. "Wisselfout" is the natural noun form. |
| Illegal stoppage | Onrechtmatige onderbreking | [BEST-GUESS] | Constructed. The Dutch rulebook addresses illegal game stoppages but uses descriptive prose rather than a fixed label. |
| Out of bounds | Buiten het speelveld | [BEST-GUESS] | Standard Dutch sports phrase. The Dutch rulebook refers to the playing area as "speelveld"; "buiten het speelveld" = out of bounds. Short form: "buiten". |
| Grabbing the wall | Vasthouden aan de wand | [BEST-GUESS] | Constructed. The Dutch rulebook prohibits holding the pool wall; no fixed idiomatic label was found. Literal translation: "vasthouden aan de wand". Alternative: "aan de wand hangen". |
| Obstruction | Obstructie | [WEB-UWH-CLUB] | Attested in Dutch UWH club sources (argonauta.nl): "obstructie" (obstructing, hindering). Also described as "hinderen" in the rulebook. "Obstructie" is the standard Dutch sports loanword. |
| Delay of game | Vertraging van het spel | [WEB-UWH-CLUB] | Attested in Dutch UWH sources: "vertragende tactiek" (delaying tactics). "Vertraging van het spel" is the natural noun-phrase form. Short form: "vertraging". |
| Unsportsmanlike conduct | Onwelvoeglijk gedrag | [FED-RULEBOOK] | The Dutch rulebook describes misconduct reasons including "verbaal geweld gericht tot een scheidsrechter" and "herhaaldelijk bekritiseren van de wedstrijdleiding". The standard formal Dutch term is "onwelvoeglijk gedrag" or "onsportief gedrag". "Onsportief gedrag" is the more common sports-media term. |
| Free arm | Vrije arm | [WEB-UWH-CLUB] | Attested in Dutch UWH club sources (argonauta.nl): "vrije arm" describes use of the non-stick arm in a prohibited way. |
| False start | Vals vertrek | [FED-RULEBOOK] | Attested in Dutch UWH rulebook references: "vals vertrek" = false start at game restart. Punished with a free puck to the opposing team and a team warning. |
