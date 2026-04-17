# Turkish UWH Glossary — First Pass

**Date:** 2026-04-17
**Purpose:** Vet UWH-specific Turkish terms before generating tr-TR/refbox.ftl.
**Review status:** Pending human review. Native-speaker review is a separate, later step.

## Source tags

- `[FED-RULEBOOK]` — from the TSSF rulebook (cite URL/section)
- `[WEB-UWH-CLUB]` — from a Turkish UWH club/team page (cite URL)
- `[WIKIPEDIA-TR]` — from Turkish Wikipedia
- `[BEST-GUESS]` — no direct source found, proposed from general Turkish sports vocabulary

## Sources consulted

- https://tssf.gov.tr/wp-content/uploads/2023/03/2023-SUALTI-HOKEYI-GENEL-TALIMAT-VE-KURALLAR.pdf — The 2023 TSSF Sualtı Hokeyi Genel Talimat ve Kurallar (official federation rulebook). Fetched successfully but its PDF content stream is compressed binary; automated text extraction did not work. Used only as confirmation the document exists; terms below come from the other sources below, which derive from or paraphrase this same rulebook.
- https://www.tssf.gov.tr/sualti-hokeyi/ — TSSF's official "Sualtı Hokeyi" branch landing page. Yielded: sualtı hokeyi, pak, hokey sopası, takım, oyuncu, yedek, gol atmak, kale, faul, hakem, 2/5-dakika oyundan uzaklaştırma cezası.
- https://www.tssf.gov.tr/yonetmelik-ve-talimatlar/ — TSSF regulations index page; used to locate the rulebook PDF.
- https://tr.wikipedia.org/wiki/Sualtı_hokeyi — Turkish Wikipedia. Yielded: pak, sopa, palet, maske, şnorkel, su topu başlığı, eldiven, devre, devre arası, mola hakkı, gol, kale, beraberlik.
- https://www.sualtihokeyi.com/oyunun-kurallari — Turkish UWH club rules summary, closely tracking the federation rulebook. Yielded: ilk devre, ikinci devre, devre arası, uzatma, 10 dakikalık uzatma (5+5), penaltı atışı, faul atışı, hakem atışı, avantajlı disk, uyarı, ikaz, ihraç, yüzey hakemi, su hakemi, 1 dakika mola, sportmenlik dışı davranış, hatalı başlangıç, disksiz engelleme, diske elle müdahale.
- https://www.sualtihokeyi.com/takim-kurulusu-ve-ozel-malzeme — club team/equipment reference. Yielded: takım kaptanı, bone, bone numaralı olmalıdır, mayolar, skor tutanlar, zaman ayarlayıcı.
- https://www.bilimselyuzme.com/kurs-eğitim/su-altı-hokeyi/sualtı-hokeyi-oyun-kuralları — Turkish UWH rules overview (scraping returned 403; title and snippet via Google confirmed terminology overlap with the club source above).
- https://dagci.com.tr/su-sporlari/sualti-sporlari/sualti-hokeyi/ — secondary club overview: sualtı hokeyi, devre, mola, faul, penaltı, gol, hakem, siyah, beyaz.
- https://sporium.net/su-alti-hokeyi-nedir-nasil-oynanir/ — secondary overview; confirmed "sualtı hokeyi", "pak", "hakem", team colour terminology.
- https://www.barcin.com/sportmen/heyecanli-su-sporu-sualti-hokeyinin-temelleri-ve-kurallari/ — sporting goods blog overview of rules; confirmed common vocabulary (dead-end as a primary source, consistent with above).
- https://marmarasport.com/blog/sualti-hokeyi-nedir.html — Turkish club blog; confirmed general vocabulary, no new terms.
- https://www.tssf.gov.tr/tssf-hakem-talimatnamesi/ — TSSF referees-regulations landing page; confirmed "başhakem" used as head-of-officials role in the TSSF context.

## Glossary (39 terms)

Trimmed to rows that correspond to concepts actually present in `en-US/refbox.ftl`.

| English (from en-US/refbox.ftl) | Turkish | Source | Notes |
|---|---|---|---|
| Black team | Siyah takım | [WEB-UWH-CLUB] | Confirmed: "koyu renk başlık/mayo/sopa" = the dark-capped team. Caps-title style used on scoreboard: "SİYAH". |
| White team | Beyaz takım | [WEB-UWH-CLUB] | "açık renk başlık/mayo/sopa" = white team. Caps-title style: "BEYAZ". |
| Score | Skor | [WEB-UWH-CLUB] / [BEST-GUESS] | "Skor" is the common Turkish sports term; "skor tutanlar" = scorekeepers. |
| Confirm (score) | Onayla | [BEST-GUESS] | Standard Turkish UI/sports vocabulary for "confirm"; "skoru onayla" = confirm the score. |
| Foul (general) | Faul | [FED-RULEBOOK] / [WIKIPEDIA-TR] | Widely used exact loanword. |
| Warning (caution) | Uyarı | [WEB-UWH-CLUB] | Alternative: "ikaz". UWH rulebooks use both; "uyarı" is the more common and neutral choice for a card/warning. |
| Penalty (time-served) | Ceza | [WEB-UWH-CLUB] | Generic "ceza" = penalty; "oyundan uzaklaştırma cezası" is the full formal phrase. |
| 30-second penalty | 30 saniyelik ceza | [BEST-GUESS] | Constructed from "saniye" (second) + "ceza" (penalty). The 30-second tier is uncommon in Turkish domestic UWH coverage but the phrase is grammatically standard. On the scoreboard: "30sn". |
| 1-minute penalty | 1 dakikalık ceza | [WEB-UWH-CLUB] | Referenced directly in Turkish rule summaries. Scoreboard: "1dk". |
| 2-minute penalty | 2 dakikalık ceza | [FED-RULEBOOK] / [WEB-UWH-CLUB] | The 2-minute tier is the most widely cited in Turkish sources. Scoreboard: "2dk". |
| 4-minute penalty | 4 dakikalık ceza | [BEST-GUESS] | Constructed; 4-minute tier not singled out in the Turkish summaries but phrase is standard. Scoreboard: "4dk". |
| 5-minute penalty | 5 dakikalık ceza | [FED-RULEBOOK] / [WEB-UWH-CLUB] | Cited directly. Scoreboard: "5dk". |
| Total dismissal | İhraç | [WEB-UWH-CLUB] | Literal meaning "expulsion". Also seen: "oyundan ihraç" (dismissal from the game). Scoreboard abbreviation: "İHR" (best-guess Turkish equivalent of "TD"). |
| First half | İlk devre | [WEB-UWH-CLUB] | "Devre" = period/half; "ilk devre" = first half. |
| Second half | İkinci devre | [WEB-UWH-CLUB] | Parallel form. |
| Half time | Devre arası | [WIKIPEDIA-TR] / [WEB-UWH-CLUB] | Literally "between the periods". Standard across Turkish sports. |
| Between games | Maçlar arası | [BEST-GUESS] | Constructed from "maç" (match) + "arası" (between). No direct rulebook hit for this exact UI phrase. Alternative: "oyunlar arası". |
| Overtime | Uzatma | [WEB-UWH-CLUB] | Direct hit; 10-minute overtime split 5+5. |
| Sudden death | Ani ölüm | [BEST-GUESS] | Literal Turkish translation; widely used in Turkish broadcasts for ice hockey and football-cup tie-breakers. The Turkish UWH sources I found describe the "keep playing until a goal is scored" rule in prose rather than naming it — "ani ölüm" is the common sports-media term for this concept in Turkish. |
| Team timeout | Takım molası | [WEB-UWH-CLUB] | Also seen as just "mola"; "mola hakkı" = right to a timeout. |
| Penalty shot | Penaltı atışı | [WEB-UWH-CLUB] | Literally "penalty throw/shot". |
| Referee (generic) | Hakem | [FED-RULEBOOK] | Standard Turkish sports word. |
| Chief referee | Başhakem | [FED-RULEBOOK] | Confirmed on the TSSF referee page. |
| Water referee | Su hakemi | [WEB-UWH-CLUB] | "Su" = water. Two water referees per game. |
| Timekeeper | Zaman ayarlayıcı | [WEB-UWH-CLUB] | Also commonly "zaman hakemi" in other Turkish sports contexts. |
| Scorer (score-keeper) | Skor tutanlar | [WEB-UWH-CLUB] | Plural form; singular: "skor tutan" or "skor hakemi". |
| Cap number | Başlık numarası | [WEB-UWH-CLUB] | "Bone numarası" also attested; "bone" is the swim-cap, "başlık" is the protective water-polo-style cap used in UWH. |
| Player | Oyuncu | [FED-RULEBOOK] | Standard. |
| Stick foul | Sopa faulü | [BEST-GUESS] | Constructed from "sopa" (stick) + "faul" (foul) + possessive suffix. Rulebook describes stick-related infractions but does not name this refbox category directly. |
| Illegal advance | Kural dışı ilerleme | [BEST-GUESS] | Constructed. Turkish sources describe offside-like situations but do not have a single established label. Alternative: "hatalı ilerleme". |
| Sub foul (substitution foul) | Oyuncu değişikliği faulü | [BEST-GUESS] | Constructed from standard sports phrase "oyuncu değişikliği" (player substitution) + "faul". |
| Illegal stoppage | Kural dışı durdurma | [BEST-GUESS] | Constructed. Alternative: "yasa dışı durdurma". |
| Out of bounds | Oyun alanı dışı | [BEST-GUESS] | Standard Turkish sports phrasing, lit. "outside the playing area". |
| Grabbing the wall | Duvardan tutunma | [BEST-GUESS] | Constructed; rulebook bans gripping the pool wall but I did not find a fixed Turkish idiom for this refbox infraction label. |
| Obstruction | Engelleme | [WEB-UWH-CLUB] | "Disksiz engelleme" = obstructing without the puck; standard term is "engelleme". |
| Delay of game | Oyunu geciktirme | [BEST-GUESS] | Standard Turkish sports construction; widely used in football broadcasts. |
| Unsportsmanlike conduct | Sportmenlik dışı davranış | [WEB-UWH-CLUB] | Direct hit — exact phrase used in Turkish UWH summary. |
| Free arm | Serbest kol | [BEST-GUESS] | Literal translation; Turkish sources describe hand-use infractions ("diske elle müdahale") but do not name "free arm" as a refbox category. |
| False start | Hatalı başlangıç | [WEB-UWH-CLUB] | Direct hit — this exact phrase appears in the Turkish UWH rules summary. |
