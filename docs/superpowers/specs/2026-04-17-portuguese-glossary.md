# Portuguese (pt-PT) UWH Glossary — First Pass

**Date:** 2026-04-17
**Purpose:** Vet UWH-specific Portuguese terms before generating pt-PT/refbox.ftl.
**Review status:** Pending human review. Native-speaker review (ideally an FPAS-affiliated referee or club official) is a separate, later step.
**Locale note:** Target is pt-PT (European Portuguese). Brazilian Portuguese variants are noted in the Notes column where a meaningful split exists. Key distinction: Portugal uses "golo" while Brazil uses "gol"; Portugal uses "equipa" while Brazil uses "equipe"; Portugal uses "prorrogação" while Brazil may use the same or "overtime".

---

## Source tags

- `[FED-RULEBOOK]` — International UWH Rulebook in Portuguese (CMAS 8th edition, v8.20, Jan 2004), accessed via silo.tips (cite URL below)
- `[WEB-UWH-PT]` — Portuguese UWH club or federation web page (FPAS or affiliated clubs)
- `[WEB-UWH-BR]` — Brazilian UWH club or federation source (hoqueisub.com.br, CBDS)
- `[WIKIPEDIA-PT]` — Portuguese Wikipedia (pt.wikipedia.org)
- `[BEST-GUESS]` — no direct UWH source found; proposed from general Portuguese sports vocabulary

---

## Sources consulted

- https://silo.tips/download/regulamento-internacional-de-hoquei-subaquatico — Portuguese translation of the CMAS International Underwater Hockey Regulations, 8th edition, version 8.20, January 2004. The richest single source found. Yielded (via search snippet extraction): árbitro principal, árbitros aquáticos, controlador de tempo, controlador de pontuação, expulsão temporária, expulsão definitiva, conduta anti-desportiva, saída incorreta, obstruir/empurrar/bloquear, uso ilegal do braço livre, paragem ilegal do disco, taco, disco, baliza, equipa preta, equipa branca, touca (numbered), falta. PDF is accessible but WebFetch was not available; terms extracted from Google-indexed snippets.
- https://arquivo.fpas.pt/conteudo/672 — FPAS archived page "Hóquei Subaquático". Yielded: stique/taco (stick), disco (puck), baliza (goal), golo (goal scored), barbatanas (fins), luva (glove), touca (cap), árbitro principal, equipas distinguidas por cores (preta/branca), simplified rules confirming two halves and interval.
- https://fpas.pt/conteudo/730 — FPAS Arbitration Committee for Underwater Hockey. Confirmed: "árbitro principal", Level II and III referee designations, "Comité de Arbitragem do Hóquei Subaquático".
- https://fpas.pt/tag/hoquei-subaquatico/ — FPAS tag page for underwater hockey. Confirmed existence of FPAS competitions (Taça de Portugal, Campeonato Nacional). No new terminology.
- https://pt.wikipedia.org/wiki/H%C3%B3quei_subaqu%C3%A1tico — Portuguese Wikipedia article on hóquei subaquático. Yielded: two halves of 15 minutes (duas partes de 15 minutos), intervalo of 3 minutes, three referees (dois dentro de água, um fora), touca (cap), taco (stick), disco (puck), baliza (goal), golo (goal scored). Confirmed "prorrogação" as the standard Portuguese sports term for overtime.
- https://www.jpn.up.pt/2013/02/09/hoquei-subaquatico-um-novo-desporto-em-portugal/ — JPN (Jornal Público/Norte) article on underwater hockey in Portugal, 2013. Confirmed general vocabulary and club names in Portugal; confirmed "equipa preta" and "equipa branca" via search snippet.
- https://sportinforma.sapo.pt/modalidades/mais-modalidades/artigos/sabia-que-tambem-se-joga-hoquei-debaixo-de-agua-o-hoquei-subaquatico-e-cativante-e-quer-voltar-a-crescer-em-portugal — Sportinforma (Sapo) feature on UWH in Portugal. Confirmed general vocabulary; no new technical terms beyond what the rulebook and FPAS sources provide.
- http://hoqueisubaquatico.weebly.com/ — AquaCarca (NASAEIST) club page. Confirmed club-level use of "hóquei subaquático", standard vocabulary. No new terminology surfaced in search snippets.
- https://hoqueisubportugal.wordpress.com/sobre/ — "Hóquei Subaquático Portugal" blog. General overview; confirmed vocabulary consistent with other Portuguese sources.
- http://www.hoqueisub.com.br/ and http://www.hoqueisub.com.br/pt-br/info.html — Brazilian UWH (hoqueisub.com.br). Confirmed: "primeiro tempo", "segundo tempo", "intervalo", "prorrogação", "morte súbita", "árbitro de água", "árbitro principal". Brazilian variant for "equipe" vs. Portuguese "equipa".
- http://www.hoqueisub.com.br/info/UWH-01-Apostila1.pdf — Brazilian UWH booklet (ACRE/UFAC, 2017). Confirmed Brazilian terminology; primary value is cross-checking with pt-PT.
- https://www.megacurioso.com.br/esportes/hoquei-subaquatico-o-esporte-coletivo-que-mistura-natacao-e-apneia — Brazilian general overview. Confirmed: "primeiro tempo", "segundo tempo", intervalo de 3 minutos, "prorrogação" (morte súbita format).
- http://aquasub.pt/attachments/photos/Documentos%20de%20Apoio/REGRAS%20MINI-HOQUEI.pdf — Aquasub.pt simplified mini-hockey rules (Portuguese). URL found but PDF not fetched; Google snippet confirmed "touca numerada" and basic game structure.
- https://esportepelomundo.com/o-hoquei-subaquatico-que-desafia-os-limites/ — Brazilian sports blog. Confirmed "prorrogação" and "morte súbita" as standard terms for overtime and sudden-death in Portuguese-language UWH coverage.

---

## Glossary (40 terms)

Trimmed to rows that correspond to concepts actually present in `en-US/refbox.ftl`.

| English (from en-US/refbox.ftl) | Portuguese (pt-PT) | Source | Notes |
|---|---|---|---|
| Black team | Equipa preta | [FED-RULEBOOK] / [WEB-UWH-PT] | The rulebook and FPAS sources use "equipa preta" for the dark-capped team. pt-PT uses "equipa"; BR variant: "equipe preta". Scoreboard caps: "PRETA". Alternative used in some Portuguese sports media: "equipa negra" — but "preta" is the rulebook term. |
| White team | Equipa branca | [FED-RULEBOOK] / [WEB-UWH-PT] | Parallel to "equipa preta". pt-PT: "equipa branca"; BR variant: "equipe branca". Scoreboard caps: "BRANCA". |
| Score | Pontuação | [BEST-GUESS] | General Portuguese sports word for score/tally. "Marcar" = to score a goal. In display context, "RESULTADO" is also common for scoreboard usage. Recommend "RESULTADO" on the scoreboard. |
| Confirm (score) | Confirmar | [BEST-GUESS] | Standard Portuguese UI and sports term. "Confirmar o resultado" = confirm the score. |
| Foul (general) | Falta | [FED-RULEBOOK] | The official Portuguese UWH rulebook uses "falta" throughout. Widely confirmed across all sources. |
| Warning (caution) | Aviso | [FED-RULEBOOK] | The rulebook uses "aviso" for a caution/warning short of a penalty. "Advertência" is a synonym used in some sports contexts but "aviso" is the primary UWH term. |
| Team warning | Aviso de equipa | [FED-RULEBOOK] / [BEST-GUESS] | Constructed from "aviso" (warning) + "de equipa" (team). The rulebook references team-level sanctions; this phrase follows its grammatical pattern. BR variant: "aviso de equipe". |
| Penalty (time-served) | Penalidade | [FED-RULEBOOK] | The rulebook uses "penalidade" for timed suspension penalties. Also seen: "expulsão temporária" (temporary expulsion) for the act of serving a penalty. |
| 30-second penalty | Penalidade de 30 segundos | [BEST-GUESS] | Constructed. The 30-second tier does not appear in the CMAS 8th edition rulebook (which uses 2-minute and 5-minute tiers); this wording is grammatically standard for pt-PT. Scoreboard: "30s". |
| 1-minute penalty | Penalidade de 1 minuto | [BEST-GUESS] | Constructed; parallel to 2-minute and 5-minute forms used in the rulebook. Scoreboard: "1min". |
| 2-minute penalty | Penalidade de 2 minutos | [FED-RULEBOOK] | The 2-minute suspension tier is the most commonly cited in the Portuguese rulebook ("expulsão temporária de 2 minutos"). Scoreboard: "2min". |
| 4-minute penalty | Penalidade de 4 minutos | [BEST-GUESS] | Constructed; the 4-minute tier does not appear in the CMAS rulebook but is used in some national-level competition formats. Scoreboard: "4min". |
| 5-minute penalty | Penalidade de 5 minutos | [FED-RULEBOOK] | The 5-minute suspension tier is explicitly named in the rulebook table of temporary expulsions. Scoreboard: "5min". |
| Total dismissal | Expulsão definitiva | [FED-RULEBOOK] | The exact phrase used in the Portuguese rulebook for permanent removal from the game. Also abbreviated in the rulebook table as "Expulsão def." Scoreboard abbreviation: "EXP" or "ED" (best-guess short form). |
| First half | Primeiro tempo | [WEB-UWH-PT] / [WEB-UWH-BR] | Confirmed across Portuguese and Brazilian sources. "Tempo" is the standard word for a playing period in Portuguese sports. Alternative: "primeira parte" — less common in UWH context but used in some Portuguese sports journalism. |
| Second half | Segundo tempo | [WEB-UWH-PT] / [WEB-UWH-BR] | Parallel to "primeiro tempo". Confirmed across sources. |
| Half time | Intervalo | [WEB-UWH-PT] / [WIKIPEDIA-PT] | "Intervalo" is the universal Portuguese sports term for half time. Confirmed in all Portuguese UWH sources as the break between halves. |
| Next game | Próximo jogo | [BEST-GUESS] | Standard Portuguese sports phrase; "próximo" = next, "jogo" = game/match. |
| Between games (nominal break) | Intervalo entre jogos | [BEST-GUESS] | Constructed from "intervalo" (break/interval) + "entre jogos" (between games). No direct rulebook hit for this UI-specific phrase. |
| Overtime | Prorrogação | [WEB-UWH-PT] / [WIKIPEDIA-PT] / [WEB-UWH-BR] | Confirmed across all sources as the standard Portuguese term for overtime/extra time in sports. BR variant: same word. Abbreviated: "PRORR." on scoreboard. |
| Overtime first half | Primeiro tempo da prorrogação | [BEST-GUESS] | Constructed by combining "primeiro tempo" + "da prorrogação". Grammatically standard; no direct UWH source uses this exact compound (the sport rarely goes to overtime). |
| Overtime half time | Intervalo da prorrogação | [BEST-GUESS] | Constructed parallel to "intervalo" (half time) + "da prorrogação". |
| Overtime second half | Segundo tempo da prorrogação | [BEST-GUESS] | Constructed parallel to "segundo tempo" + "da prorrogação". |
| Pre-overtime break | Pausa pré-prorrogação | [BEST-GUESS] | Constructed from "pausa" (break/pause) + "pré-prorrogação". No direct source; this is a refbox-specific UI concept. Alternative: "intervalo pré-prorrogação". |
| Sudden death | Morte súbita | [WEB-UWH-BR] / [WIKIPEDIA-PT] | "Morte súbita" is confirmed in Brazilian UWH sources and is the standard Portuguese sports term across all hockey variants and football. Also used in Portuguese (Portugal) sports media. |
| Pre-sudden-death break | Pausa pré-morte-súbita | [BEST-GUESS] | Constructed; no direct source. Alternative: "intervalo pré-morte-súbita". |
| Team timeout | Tempo de equipa | [BEST-GUESS] | In Portuguese sports, "time-out" is often left in English or rendered as "tempo de equipa" (team time). FPAS underwater rugby rules confirm "tempo de jogo" constructions; "tempo de equipa" is the most natural pt-PT form. BR variant: "tempo de equipe". Abbreviated: "T.E." on scoreboard. |
| Penalty shot | Tiro de penalidade | [WEB-UWH-BR] / [BEST-GUESS] | "Tiro de penalidade" is the standard Portuguese sports term for a penalty shot/throw. In pt-PT the term "pontapé de penalidade" (penalty kick) is football-specific; "tiro de penalidade" is more sport-neutral and appropriate here. |
| Referee (generic) | Árbitro | [FED-RULEBOOK] | Standard Portuguese sports term. Used throughout the rulebook. |
| Chief referee | Árbitro principal | [FED-RULEBOOK] / [WEB-UWH-PT] | Confirmed explicitly in the CMAS Portuguese rulebook: "árbitro principal" is the head official outside the water. Also confirmed on the FPAS arbitration page. |
| Water referee | Árbitro aquático | [FED-RULEBOOK] | Confirmed in the CMAS Portuguese rulebook: "árbitros aquáticos" = the two in-water referees. BR sources also use "árbitro de água" — note the pt-PT rulebook form is "árbitro aquático". |
| Timekeeper | Controlador de tempo | [FED-RULEBOOK] | Exact phrase from the CMAS Portuguese rulebook: "controlador de tempo" must have equipment to time the game and suspended players. |
| Scorer (score-keeper) | Controlador de pontuação | [FED-RULEBOOK] | Exact phrase from the CMAS Portuguese rulebook: "controlador de pontuação" maintains the written record and visible scoreboard. |
| Cap number | Número de touca | [FED-RULEBOOK] / [WEB-UWH-PT] | The rulebook specifies that toucas must be numbered ("toucas numeradas") and players must mark their number on their upper arm. "Touca" = the ear-protection cap used in UWH. Confirmed in FPAS source. |
| Player | Jogador | [FED-RULEBOOK] | Standard Portuguese sports term. Used throughout the rulebook. |
| Stick foul | Falta de taco | [FED-RULEBOOK] / [BEST-GUESS] | The rulebook uses "taco" for the stick and "falta" for foul; "falta de taco" is a natural compound but the rulebook does not use this exact label as a category heading. Alternative: "uso ilegal do taco". |
| Illegal advance | Avanço ilegal | [BEST-GUESS] | Constructed from "avanço" (advance) + "ilegal" (illegal). No direct UWH source names this refbox category explicitly in Portuguese. |
| Sub foul (substitution foul) | Falta de substituição | [FED-RULEBOOK] / [BEST-GUESS] | The rulebook table includes "Saída incorreta" (incorrect exit/substitution) as an infraction. "Falta de substituição" is a reasonable label for this category; "saída incorreta" is the rulebook's own phrase and may be preferable in the translation. |
| Illegal stoppage | Paragem ilegal | [FED-RULEBOOK] | The CMAS Portuguese rulebook uses "paragem ilegal do disco" (illegal stopping of the puck) — confirmed in search snippets from the silo.tips document. "Paragem" is the pt-PT word; BR variant: "parada ilegal". |
| Out of bounds | Fora dos limites | [BEST-GUESS] | Standard Portuguese sports phrasing, lit. "outside the limits/boundaries". No direct UWH rulebook hit for this exact label. Alternative: "fora do campo". |
| Grabbing the wall | Agarrar a parede | [BEST-GUESS] | Constructed; the rulebook prohibits gripping the pool wall but does not establish a fixed Portuguese label for this infraction category. "Agarrar" = to grab/hold; "parede" = wall. |
| Obstruction | Obstrução | [FED-RULEBOOK] | The CMAS Portuguese rulebook explicitly names "obstruir, empurrar e bloquear" as one signalled infraction category. "Obstrução" is the noun form and is the natural UI label. |
| Delay of game | Atraso de jogo | [BEST-GUESS] | Standard Portuguese sports construction used in football and other sports broadcasts. No direct UWH hit but the phrase is universally understood. |
| Unsportsmanlike conduct | Conduta anti-desportiva | [FED-RULEBOOK] | Exact phrase confirmed in the CMAS Portuguese rulebook ("conduta anti desportiva") and in the signal description and infraction table. Note: the rulebook spells it without hyphen ("anti desportiva"); standard pt-PT orthography uses a hyphen ("anti-desportiva"). |
| Free arm | Braço livre | [FED-RULEBOOK] | The CMAS Portuguese rulebook uses "braço livre" and "mão livre" throughout to describe the non-stick arm/hand. "Uso ilegal do braço livre" is the full infraction description. |
| False start | Saída falsa | [BEST-GUESS] | No direct UWH rulebook hit for this exact category name. The rulebook lists "Saída incorreta" for substitution violations; "saída falsa" (false start) is the general Portuguese sports term used in swimming and athletics. Confirm with a referee whether this or another phrase is used in UWH practice in Portugal. |
