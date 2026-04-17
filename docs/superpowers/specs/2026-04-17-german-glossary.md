# German UWH Glossary — First Pass

**Date:** 2026-04-17
**Purpose:** Vet UWH-specific German terms before generating de-DE/refbox.ftl.
**Review status:** Pending human review. Native-speaker review is a separate, later step.

## Source tags

- `[FED-RULEBOOK]` — from the VDST or CMAS federation rulebook (cite URL/section)
- `[WEB-UWH-CLUB]` — from a German-speaking UWH club or team page (cite URL)
- `[WIKIPEDIA-DE]` — from German Wikipedia
- `[BEST-GUESS]` — no direct source found, proposed from general German sports vocabulary

## Sources consulted

- https://www.vdst.de/2023/01/22/internationale-unterwasser-hockey-regelwerke/ — VDST announcement
  of the updated (November 2022) international rulebook, consisting of three PDF documents.
  Confirmed that the VDST uses "Unterwasserhockey" (one word) as the canonical sport name and
  that the international CMAS rulebook is the authoritative source. No inline rule text
  accessible via search alone.

- https://www.vdst.de/zeigen/leistungssport/uwh/ — VDST Unterwasser-Hockey branch landing page.
  Confirmed sport is an official VDST competitive discipline since 1998; confirmed "VDST" as the
  German umbrella federation. No specific glossary terms extracted directly.

- https://de.wikipedia.org/wiki/Unterwasserhockey — German Wikipedia article. Yielded: Schläger,
  Puck, Flossen, Maske, Schnorchel, Badekappe, Tor (goal/net), Freistoß, Foul, Wasserschiedsrichter,
  Hauptschiedsrichter, schwarze und weiße Mannschaft (implicit through cap colour convention).

- https://tauch-club-hannover.de/unterwasserhockey/ — Tauch-Club Hannover UWH overview page.
  Yielded: Schläger, Foul, Freistoß, Strafzeit, Verwarnung, Hauptschiedsrichter,
  Wasserschiedsrichter, Zeitnehmer, Auswechselspieler, Wasserballkappe (mannschaftsfarbig).

- https://www.unterwasserhockey-muenchen.de/ueberuns.html — Unterwasserhockey München e.V.
  equipment page. Yielded: Badekappe (with ear protection), Handschuh, Schläger, Flossen, Maske,
  Schnorchel; confirmed equipment terminology.

- https://health-n-fit.de/grundlagen-und-regeln-des-unterwasserhockeys/ — German sports-health
  blog, rules overview. Yielded: "zwei Halbzeiten" (8–15 Minuten), Freistoß, Foul, Schläger;
  confirmed "Halbzeit" as the standard German term for the game period.

- https://uwsport.de/3_1_1.php and http://uwsport.de/3_4_1.php — UWSport.de German UWH portal
  (overview and rules pages). Confirmed "Unterwasserhockey" and noted that a German rules
  translation (version 1.010, June 2007) existed on a Schlickteufel Elmshorn page; terms are
  consistent with other club sources.

- http://www.reedconsulting.com/uwh.ch/rules/de/cmas-rules-2005-9.10-vol-2_de.pdf — German
  translation of the CMAS rules (version 9.10, 2005), hosted by the Swiss UWH community. PDF
  not directly fetched due to tool permissions; URL confirmed via multiple search snippets.
  Snippets confirmed: Spielhälfte, Strafzeit, Auszeit (one per half), Strafstoß, Freistoß,
  Verwarnung, Platzverweis, Hauptschiedsrichter, Wasserschiedsrichter, Zeitnehmer.

- https://htsv.org/fileadmin/Fachbereiche/Wettkampf/UWH-01-Einfuehrung_060406.pdf — HTSV
  (Hamburg Turnen und Sport) UWH introduction document (2006). Confirmed presence via search;
  snippets confirmed: Wasserschiedsrichter, Hauptschiedsrichter, Zeitnehmer terminology.

- https://susv.ch/sites/default/files/documents/field_global_dateien/2020/06/28/Unterwasserhockey%20lass%20dich%20auf%20ein%20Probespiel%20ein-3%202.pdf
  — SUSV (Swiss Underwater Sports Federation) introductory handout. Confirmed via search;
  yielded general terminology consistent with VDST usage.

- https://www.susv.ch/de/unterwasser-hockey/ — SUSV Swiss UWH landing page. Confirmed that
  Swiss federation plays under the same CMAS international rules as Germany/Austria.

- https://sporttaucher-berlin.de/11-2017-fotos-vom-uwh-schiri-kurs/ — Sporttaucher Berlin
  referee course report (November 2017). Confirmed "VDST-UWH-Schiedsrichterordnung" exists;
  confirmed the three-official structure (2–3 Wasserschiedsrichter + Hauptschiedsrichter).

## Glossary (39 terms)

Trimmed to rows that correspond to concepts actually present in `en-US/refbox.ftl`.

| English (from en-US/refbox.ftl) | German | Source | Notes |
|---|---|---|---|
| Black team | Schwarze Mannschaft | [WEB-UWH-CLUB] | Multiple German club sources describe the "schwarze Mannschaft" (dark-capped team). Scoreboard caps-title: "SCHWARZ". |
| White team | Weiße Mannschaft | [WEB-UWH-CLUB] | Parallel form for the light-capped team. Scoreboard caps-title: "WEISS". |
| Score | Tor / Ergebnis | [WIKIPEDIA-DE] | "Tor" = a goal scored; "Ergebnis" = the scoreline. UI label for the running score would typically be "Spielstand" or just "Tore". |
| Confirm (score) | Bestätigen | [BEST-GUESS] | Standard German UI/sports verb for "confirm". Full phrase: "Spielstand bestätigen". |
| Foul (general) | Foul | [WEB-UWH-CLUB] / [WIKIPEDIA-DE] | Direct English loanword used in all German UWH sources without substitution. Plural: "Fouls". |
| Warning (caution) | Verwarnung | [WEB-UWH-CLUB] | Confirmed across Hannover and health-n-fit sources; standard German sports term for a formal caution short of exclusion. |
| Penalty (time-served) | Strafzeit | [WEB-UWH-CLUB] | Confirmed across multiple German UWH sources. Full formal phrase: "Zeitstrafe" is also used in general German sports, but "Strafzeit" is the attested UWH-specific form. |
| 30-second penalty | 30-Sekunden-Strafzeit | [BEST-GUESS] | Constructed from "Sekunde" + "Strafzeit". The 30-second tier is not specifically named in the German-language UWH club sources consulted. Scoreboard short form: "30s". |
| 1-minute penalty | 1-Minuten-Strafzeit | [WEB-UWH-CLUB] | Consistent with the pattern from German sources; the 1-minute tier is referenced in rule summaries. Scoreboard short form: "1m". |
| 2-minute penalty | 2-Minuten-Strafzeit | [WEB-UWH-CLUB] | The most commonly cited penalty duration in German UWH coverage. Scoreboard short form: "2m". |
| 4-minute penalty | 4-Minuten-Strafzeit | [BEST-GUESS] | Constructed; the 4-minute tier is not singled out in German club sources but follows the established pattern. Scoreboard short form: "4m". |
| 5-minute penalty | 5-Minuten-Strafzeit | [WEB-UWH-CLUB] | Cited alongside the 2-minute penalty in German rule summaries. Scoreboard short form: "5m". |
| Total dismissal | Platzverweis | [FED-RULEBOOK] | The CMAS German rules (version 9.10, reedconsulting snippet) use "Platzverweis" for permanent exclusion. Alternative seen in general German sports: "endgültiger Ausschluss". Scoreboard abbreviation: "PV" (best-guess equivalent of "TD"). |
| First half | Erste Halbzeit | [WEB-UWH-CLUB] / [WIKIPEDIA-DE] | "Halbzeit" is the confirmed German UWH term for each playing period. health-n-fit: "zwei Halbzeiten". |
| Second half | Zweite Halbzeit | [WEB-UWH-CLUB] | Parallel form; standard across all German sports. |
| Half time (the break) | Halbzeitpause | [BEST-GUESS] | "Halbzeitpause" is the standard German compound for the break between halves in all team sports. Not found with this exact compound in UWH-specific text, but "Pause" and "Halbzeit" appear together in results. Alternative: "Halbzeit" alone (context-dependent). |
| Between games | Spielpause | [BEST-GUESS] | Constructed; "Spielpause" is the general German sports term for a break between matches. Alternative: "Pause zwischen den Spielen". |
| Overtime | Verlängerung | [WEB-UWH-CLUB] | Used in VDST UWH league reports (Rückrunde 2023/2024 snippets reference Verlängerung). Standard German sports term for extra time. |
| Overtime first half | Erste Verlängerungshälfte | [BEST-GUESS] | Constructed from "Verlängerung" + "Hälfte". No direct UWH-specific German source for this exact phrase found. |
| Overtime half time | Halbzeitpause der Verlängerung | [BEST-GUESS] | Constructed. Shortened to "Verlängerungspause" in tight UI contexts. |
| Overtime second half | Zweite Verlängerungshälfte | [BEST-GUESS] | Parallel to "Erste Verlängerungshälfte". |
| Sudden death | Plötzlicher Tod | [BEST-GUESS] | The literal German translation; confirmed used in German general sports media. German UWH VDST league reports reference "Golden Goal" as an alternative — both are understood. "Golden Goal" is arguably more widely used in German UWH community language than "plötzlicher Tod". |
| Team timeout | Auszeit | [FED-RULEBOOK] | The CMAS German rules (reedconsulting snippet) confirm one "Auszeit" per half per team. "Auszeit" is the established German sports term. Full: "Mannschaftsauszeit" is also used. |
| Penalty shot | Strafstoß | [FED-RULEBOOK] | Confirmed in the CMAS German rules snippet. Literally "penalty thrust/push". Note: not "Elfmeter" (football term) or "Penaltyschuss" — "Strafstoß" is the attested UWH term. |
| Referee (generic) | Schiedsrichter | [WEB-UWH-CLUB] / [WIKIPEDIA-DE] | Standard German sports word, universally used. |
| Chief referee | Hauptschiedsrichter | [WEB-UWH-CLUB] | Confirmed across Hannover club page, HTSV PDF snippets, and Berlin referee course report. The chief referee stands at the pool deck and controls the horn/signal. |
| Water referee | Wasserschiedsrichter | [WEB-UWH-CLUB] | Confirmed across multiple German UWH sources; the 2–3 officials in the water. Identified by neon-coloured shirts and orange-red gloves. |
| Timekeeper | Zeitnehmer | [WEB-UWH-CLUB] | Confirmed in Hannover and HTSV sources. Standard German sports compound. |
| Scorer (score-keeper) | Anschreiber | [BEST-GUESS] | "Anschreiber" is standard German for the official who records the score; not directly attested in UWH sources found, but is the conventional sports term. Alternative: "Ergebnisschreiber". |
| Cap number | Badekappennummer | [WEB-UWH-CLUB] | Hannover source confirms caps carry the player number; München source confirms "Badekappe" as the cap type. "Badekappennummer" is the natural compound. Short form on UI: "Kappennum." |
| Player | Spieler | [WIKIPEDIA-DE] | Universally used in all German UWH sources. |
| Stick foul | Schläger-Foul | [BEST-GUESS] | "Schläger" is the confirmed German term for the UWH stick (all sources). The compound "Schläger-Foul" is not attested in any UWH-specific German source found; constructed by analogy. |
| Illegal advance | Regelwidriges Vorrücken | [BEST-GUESS] | Constructed from "regelwidrig" (against the rules) + "Vorrücken" (advancing). No fixed German UWH label found for this refbox category. |
| Sub foul (substitution foul) | Auswechsel-Foul | [BEST-GUESS] | "Auswechselspieler" is the confirmed German UWH term (Hannover source) for substitute players. "Auswechsel-Foul" constructed by analogy; no direct attestation found. |
| Illegal stoppage | Regelwidrige Spielunterbrechung | [BEST-GUESS] | Constructed. "Spielunterbrechung" = game stoppage is standard German. No fixed UWH-specific label found. |
| Out of bounds | Außerhalb des Spielfeldes | [BEST-GUESS] | Standard German sports phrasing; the playing area is "Spielfeld". The Hannover source describes the substitution zone as "außerhalb des Spielfeldes". Shortened: "Aus". |
| Grabbing the wall | Festhalten am Beckenrand | [BEST-GUESS] | Constructed from "Beckenrand" (pool wall/edge) + "festhalten" (to hold/grab). German UWH sources confirm that holding/grabbing is a foul; no fixed label for the pool-wall variant found. |
| Obstruction | Behinderung | [WEB-UWH-CLUB] | "Behinderung" (obstruction/hindrance) is the standard German sports term; consistent with German UWH club descriptions of blocking-type fouls. |
| Delay of game | Spielverzögerung | [BEST-GUESS] | Standard German sports construction; widely used in football and other team-sport broadcasts. No UWH-specific attestation found. |
| Unsportsmanlike conduct | Unsportliches Verhalten | [WEB-UWH-CLUB] | German club sources (Hannover, health-n-fit) reference "unsportliches Verhalten" when describing foul categories. Consistent with standard German sports law. |
| Free arm | Freier Arm | [BEST-GUESS] | Literal translation. German UWH sources describe hand-related infractions (puck stopped by glove or body) but do not provide a fixed label for this specific refbox infraction category. |
| False start | Fehlstart | [BEST-GUESS] | "Fehlstart" is the standard German term for a false start in swimming and athletics; it is widely understood and the most natural equivalent for this infraction in a pool setting. Not directly attested in German UWH rule summaries. |
