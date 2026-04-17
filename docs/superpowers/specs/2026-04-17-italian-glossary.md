# Italian UWH Glossary — First Pass

**Date:** 2026-04-17
**Purpose:** Vet UWH-specific Italian terms before generating it-IT/refbox.ftl.
**Review status:** Pending human review. Native-speaker review is a separate, later step.

> **Important caveat:** Several infraction labels in `en-US/refbox.ftl` (e.g. "Stick Foul",
> "Free Arm", "False Start") do not appear as fixed phrases in any Italian UWH source found.
> Those rows are tagged `[BEST-GUESS]` and should be confirmed against the full FIPSAS
> National Regulation PDF before the .ftl file is generated.
>
> Also notable: the FIPSAS National Regulation uses penalty durations of **1, 2, or 5 minutes**
> only. The refbox also supports 30-second and 4-minute penalties; these are not attested in
> Italian UWH sources and the translations below are constructed by analogy.

## Source tags

- `[FED-RULEBOOK]` — from the FIPSAS Regolamento Nazionale di Hockey Subacqueo (cite URL)
- `[WEB-UWH-CLUB]` — from an Italian UWH club/team page (cite URL)
- `[WIKIPEDIA-IT]` — from Italian Wikipedia
- `[BEST-GUESS]` — no direct source found, proposed from general Italian sports vocabulary

## Sources consulted

- https://www.fipsas.it/agonismo-subacqueo/attivita-subacquee/hockey-subacqueo/documenti-e-modulistica-hockey-sub/2781-regolamento-nazionale-di-hockey-subacqueo/file — The FIPSAS Regolamento Nazionale di Hockey Subacqueo (official federation rulebook PDF). Direct text extraction was not attempted (PDF served by Joomla behind redirects); content reached via Google-indexed excerpts and search engine snippets. Yielded: tempi (primo/secondo tempo), intervallo, tempo supplementare, morte improvvisa, arbitro principale, arbitro di vasca, penalità di 1/2/5 minuti, ammonizione, espulsione, espulsione definitiva, tiro di rigore, time-out, equal puck (palla a due), advantage puck (punizione), calottina, mazzetta, dischetto, riserva, sostituzione, capo arbitro, infrazioni minori/maggiori/gravi.
- https://www.fipsas.it/agonismo-subacqueo/attivita-subacquee/hockey-subacqueo — FIPSAS Hockey Subacqueo main landing page. Confirmed: "hockey subacqueo", "capo arbitro", team-colour system (nero/bianco), basic game structure.
- https://www.fipsas.it/gare-as-e-np/gare-hockey-subacqueo/5764-regolamento-particolare-campionato-italiano-assoluto-di-hockey-subacqueo-2025/file — 2025 Italian Championship particular regulation circular. Confirmed: time-out di squadra limited to the two regular halves only; tempo supplementare split 5+5 minutes with 1-minute interval; morte improvvisa procedure; mazzetta colour rule (nero o bianco).
- https://it.wikipedia.org/wiki/Hockey_subacqueo — Italian Wikipedia article on hockey subacqueo. Yielded: hockey subacqueo, dischetto, mazzetta, calottina, pinne, maschera, boccaglio, gol, arbitro, capo arbitro, primo tempo, secondo tempo, intervallo, tempo supplementare.
- https://www.altitudoroma.it/hockey-subacqueo/ — Altitudo Roma Hockey Sub club page (Rome). Confirmed: hockey subacqueo, gol, arbitro, fallo, penalità, time-out di squadra, squadra nera/bianca.
- https://www.eridaniasub.it/scuola-sub/area-sportiva/hockey-subacqueo/ — Eridania Sub club page (Turin/Po Valley). Confirmed: general game structure, penalty/foul vocabulary consistent with FIPSAS.
- https://www.fipsasbologna.it/index.php/subacquea-2/hockey-subacqueo/ — FIPSAS Bologna affiliate club page. Confirmed: standard FIPSAS terminology, capo arbitro, calottina numerata, riserve.
- https://www.circoloinzani.it/wp/wp-content/uploads/2020/02/LEONE-TAROZZI-UWH-FOR-DUMMIES.pdf — "Hockey Subacqueo for Dummies" (Circolo Inzani, Milan club). A player-facing introduction in Italian. Yielded: sostituzione, fallo, ammonizione, espulsione, ritardo di gioco concept (described in prose), comportamento antisportivo, calottina numerata, giocatore, riserva.
- https://ifg.uniurb.it/static/lavori-fine-corso-2004/gentili/regole.htm — Italian university journalism project summary of UWH rules in Italian. Yielded: primo tempo, secondo tempo, intervallo di 3 minuti, arbitro in acqua, ammonizione, espulsione, tiro di rigore, palla a due.
- https://www.divemania.it/post/l-hockey-subacqueo — Divemania.it Italian diving portal overview. Confirmed: hockey subacqueo, dischetto, mazzetta, capo arbitro, fallo.
- https://www.ilmessaggero.it/sport/altrisport/hockey_subacqueo_regole_campionati-8797450.html — Il Messaggero article (Altitudo Roma feature). Confirmed: hockey subacqueo, squadra, penalità, time-out, gol.
- https://sogese.com/wpsite/wp-content/uploads/Regole-Hockey-Subacqueo.pdf — Simplified rules card (PDF). Title confirmed via search; full PDF content not extracted but URL confirms "Regole Hockey Subacqueo" as standard Italian label for the rules document.

## Glossary (39 terms)

Trimmed to rows that correspond to concepts actually present in `en-US/refbox.ftl`.

| English (from en-US/refbox.ftl) | Italian | Source | Notes |
|---|---|---|---|
| Black team | Squadra nera | [FED-RULEBOOK] / [WEB-UWH-CLUB] | The FIPSAS rulebook and club pages consistently use "nero" for the dark team. Caps scoreboard form: "NERO". |
| White team | Squadra bianca | [FED-RULEBOOK] / [WEB-UWH-CLUB] | The light team is consistently "bianco/a". Caps scoreboard form: "BIANCO". |
| Score | Punteggio | [WIKIPEDIA-IT] / [WEB-UWH-CLUB] | "Punteggio" is standard Italian sports vocabulary for score/result. Alternative: "risultato". Scoreboard label: "PUNTEGGIO" or abbreviated "PUNT." |
| Confirm (score) | Conferma | [BEST-GUESS] | No single FIPSAS phrase for "confirm score" found. "Conferma" is the standard Italian UI/sports verb-noun; "conferma del punteggio" = confirm the score. |
| Foul (general) | Fallo | [FED-RULEBOOK] / [WIKIPEDIA-IT] | Universally used in FIPSAS documents and all Italian UWH sources. No alternative found. |
| Warning (caution) | Ammonizione | [FED-RULEBOOK] | Used directly in FIPSAS regulation excerpts for a formal caution short of expulsion: "ammonizione verbale" for minor/accidental infractions. Alternative: "avvertimento" (more informal). |
| Penalty (time-served) | Penalità | [FED-RULEBOOK] | Standard term throughout FIPSAS documents. The full phrase is "penalità di X minuti" (penalty of X minutes). Also used: "espulsione temporanea". |
| 30-second penalty | Penalità di 30 secondi | [BEST-GUESS] | The 30-second tier is not mentioned in any Italian UWH source (FIPSAS uses 1/2/5 min only). Phrase constructed from standard Italian. Scoreboard short form: "30s". |
| 1-minute penalty | Penalità di 1 minuto | [FED-RULEBOOK] | Directly attested in FIPSAS regulation excerpts: "penalità che può essere di 1, 2 o 5 minuti". Scoreboard short form: "1m". |
| 2-minute penalty | Penalità di 2 minuti | [FED-RULEBOOK] | Directly attested. Scoreboard short form: "2m". |
| 4-minute penalty | Penalità di 4 minuti | [BEST-GUESS] | Not attested in Italian UWH sources (FIPSAS uses 1/2/5 min only). Phrase constructed by analogy. Scoreboard short form: "4m". |
| 5-minute penalty | Penalità di 5 minuti | [FED-RULEBOOK] | Directly attested. Scoreboard short form: "5m". |
| Total dismissal | Espulsione definitiva | [FED-RULEBOOK] | FIPSAS excerpts use "espulsione definitiva" for a player dismissed for the remainder of the game. The referees must file a written report ("verbale") with tournament officials. Scoreboard abbreviation: "ES.DEF." (best-guess short form for Italian; no attested abbreviation found). |
| First half | Primo tempo | [FED-RULEBOOK] / [WIKIPEDIA-IT] | Universally used. "Tempo" = period/half in Italian sports (football, hockey); "primo tempo" is the standard first half. |
| Second half | Secondo tempo | [FED-RULEBOOK] / [WIKIPEDIA-IT] | Parallel form; universally attested. |
| Half time | Intervallo | [FED-RULEBOOK] / [WIKIPEDIA-IT] | "Intervallo" is the standard Italian term for the break between halves, directly used in FIPSAS sources. Alternative: "pausa tra i tempi". |
| Pre-overtime break | Pausa pre-supplementari | [BEST-GUESS] | FIPSAS describes the break before overtime but does not name it with a fixed label in the excerpts found. Constructed from "pausa" + "supplementari". |
| Overtime | Tempi supplementari | [FED-RULEBOOK] | FIPSAS regulation confirms "tempi supplementari" (two 5-minute halves). Singular: "tempo supplementare". |
| Overtime first half | Primo tempo supplementare | [FED-RULEBOOK] | Constructed from attested "primo tempo" and "supplementare". |
| Overtime half time | Intervallo supplementare | [BEST-GUESS] | FIPSAS confirms a 1-minute interval between overtime halves but does not name it explicitly. Constructed by analogy with "intervallo". |
| Overtime second half | Secondo tempo supplementare | [FED-RULEBOOK] | Constructed from attested "secondo tempo" and "supplementare". |
| Sudden death | Morte improvvisa | [FED-RULEBOOK] | Directly confirmed: the FIPSAS 2025 championship regulation describes "morte improvvisa" as the tiebreaker procedure ("si riprenderà il gioco senza interruzioni fino a quando una delle squadre segna un Goal"). This is the official Italian UWH federation term. |
| Pre-sudden-death break | Pausa pre-morte improvvisa | [BEST-GUESS] | Not named in sources; constructed from "pausa" + "morte improvvisa". |
| Team timeout | Time-out di squadra | [FED-RULEBOOK] | FIPSAS documents confirm "time-out" (anglicism used in Italian sports). The FIPSAS 2025 regulation specifically calls them "time-out di squadra" and restricts them to the two regular halves. Alternative Italian form: "sospensione di squadra". |
| Penalty shot | Tiro di rigore | [FED-RULEBOOK] / [WEB-UWH-CLUB] | Directly attested in multiple sources: FIPSAS excerpts, the ifg.uniurb.it rules summary, and club pages all use "tiro di rigore". |
| Referee (generic) | Arbitro | [FED-RULEBOOK] | Standard Italian sports term, universally used. |
| Chief referee | Capo arbitro | [FED-RULEBOOK] / [WIKIPEDIA-IT] | Directly attested: "capo arbitro è posizionato fuori dall'acqua" (the chief referee is positioned outside the water). No alternative form found. |
| Water referee | Arbitro di vasca | [FED-RULEBOOK] | FIPSAS regulation refers to the underwater officials as "arbitri di vasca" (pool referees) in the search-indexed excerpts. Also described as "arbitro in acqua" (referee in the water). |
| Timekeeper | Cronometrista | [FED-RULEBOOK] / [BEST-GUESS] | "Cronometrista" is standard Italian for timekeeper across all aquatic sports. Not found as an exact label in FIPSAS UWH excerpts, but consistent with FIPSAS aquatic sport regulations generally. Alternative: "addetto al cronometro". |
| Scorer (score-keeper) | Segnapunti | [BEST-GUESS] | Standard Italian sports vocabulary for the person keeping score. Not found as an explicit label in FIPSAS UWH excerpts. Alternative: "addetto al punteggio". |
| Cap number | Numero di calottina | [FED-RULEBOOK] | FIPSAS regulation confirms "calottina" as the cap (water-polo-style head cap) and requires numbers on it and on the player's arms ("numero della propria cuffia"). "Numero di calottina" is the natural combination. |
| Player | Giocatore | [FED-RULEBOOK] | Standard. FIPSAS uses "giocatore/i" throughout. |
| Stick foul | Fallo di mazzetta | [BEST-GUESS] | "Mazzetta" is the confirmed FIPSAS/Italian term for the UWH stick. No fixed Italian label for "stick foul" as a named infraction category was found. Constructed from "fallo di mazzetta". The FIPSAS rulebook lists infrazioni involving the mazzetta (e.g. guiding the puck with both hands on the stick) but does not use this compound as a heading. |
| Illegal advance | Avanzamento illegale | [BEST-GUESS] | No fixed Italian label found in UWH sources. Constructed from general Italian sports vocabulary. Alternative: "avanzamento non regolare". |
| Sub foul (substitution foul) | Fallo di sostituzione | [BEST-GUESS] | FIPSAS regulation describes substitution violations in detail (player entering before replaced player exits, headfirst entry, etc.) and imposes "espulsione temporanea diretta senza ammonizione" for these — but does not use "fallo di sostituzione" as a named heading. Phrase constructed by analogy. |
| Illegal stoppage | Arresto irregolare | [BEST-GUESS] | No fixed Italian UWH label found. Constructed from "arresto" (stoppage) + "irregolare" (irregular/illegal). Alternative: "interruzione illegale". |
| Out of bounds | Fuori dal campo | [BEST-GUESS] | Standard Italian sports phrase. UWH sources describe the boundary lines but do not use a fixed "fuori dal campo" label as an infraction heading. Alternative: "fuori area". |
| Grabbing the wall | Aggrapparsi alla parete | [BEST-GUESS] | No fixed Italian UWH phrase found. "Aggrapparsi" = to grab/cling; "parete" = wall (of the pool). Alternative: "aggancio alla parete della vasca". |
| Obstruction | Ostruzione | [FED-RULEBOOK] | "Ostruzione" is used in FIPSAS regulation excerpts as an infraction category ("grasping, holding, pushing" classified as "infrazioni maggiori"; ostruzione as a general concept is attested). Standard Italian sports term. |
| Delay of game | Perdita di tempo | [WEB-UWH-CLUB] | "Perdita di tempo" (literally "time wasting") is the standard Italian sports term for delay-of-game type infractions, used in football and other Italian sports contexts. No direct UWH-specific phrase found, but consistent with Italian sports vocabulary. |
| Unsportsmanlike conduct | Comportamento antisportivo | [FED-RULEBOOK] | Directly attested: FIPSAS regulation excerpts and the FIPSAS Tribunale Federale reference "Gioco pericoloso e/o comportamento antisportivo" as a named disciplinary infraction. |
| Free arm | Braccio libero | [BEST-GUESS] | No Italian UWH source uses "braccio libero" as a named infraction category. The literal translation from Italian sports vocabulary. The FIPSAS rulebook describes hand-use violations in prose. |
| False start | Falsa partenza | [BEST-GUESS] | No fixed Italian UWH label found. "Falsa partenza" is the standard Italian term for false start across sports (athletics, swimming). Consistent with Italian sports vocabulary; FIPSAS rules describe the concept in prose without naming it as a refbox-style category heading. |
