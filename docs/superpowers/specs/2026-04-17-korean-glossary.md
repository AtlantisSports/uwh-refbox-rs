# Korean UWH Glossary — First Pass

**Date:** 2026-04-17
**Purpose:** Vet UWH-specific Korean terms before generating ko-KR/refbox.ftl.
**Review status:** Pending human review. Native-speaker review is a separate, later step.

## Source tags

- `[FED-RULEBOOK]` — from a Korean or CMAS federation rulebook (cite URL/section)
- `[WEB-UWH-CLUB]` — from a Korean UWH club/team page (cite URL)
- `[WIKIPEDIA-KO]` — from Korean Wikipedia (ko.wikipedia.org)
- `[NAMU-WIKI]` — from Namu Wiki (namu.wiki), a major Korean reference wiki
- `[BEST-GUESS]` — no direct Korean UWH source found; proposed from general Korean sports vocabulary

## Important caveat

Korean UWH is a small emerging sport. The Korea Underwater Association (대한수중핀수영협회, KUA / kua.or.kr) governs underwater hockey under CMAS, but **no publicly indexed Korean-language rulebook or official UWH terminology document was found during research**. The KUA website did not expose a searchable underwater hockey rules page. The namu.wiki article on 수중하키 and the Korean Wikipedia article on 수중 하키 provide basic structural information but no technical vocabulary for penalties, infractions, or referee types.

As a result, the majority of terms below are sourced from confirmed Korean general sports vocabulary (football, ice hockey, water polo) and standard Korean dictionary usage, with `[BEST-GUESS]` tags applied wherever a UWH-specific Korean term could not be sourced. **All `[BEST-GUESS]` rows require native-speaker and ideally KUA review before use in production.**

## Sources consulted

- https://namu.wiki/w/%EC%88%98%EC%A4%91%ED%95%98%ED%82%A4 — Namu Wiki "수중하키" article. Yielded: basic game structure (6 active players per team, puck/스틱 equipment), team formation notes. No penalty or infraction vocabulary.
- https://ko.wikipedia.org/wiki/%EC%88%98%EC%A4%91_%ED%95%98%ED%82%A4 — Korean Wikipedia "수중 하키" article. Yielded: 수중 하키 (sport name), CMAS as governing body, basic gameplay description. No technical vocabulary for penalties or referees.
- https://ko.wikipedia.org/wiki/%EB%8C%80%ED%95%9C%EC%88%98%EC%A4%91%ED%95%80%EC%88%98%EC%98%81%ED%98%91%ED%9A%8C — Korean Wikipedia article on 대한수중핀수영협회 (KUA). Confirmed the KUA is the CMAS affiliate for Korea and oversees underwater hockey. No rulebook terminology.
- https://kua.or.kr/ — KUA official website. No publicly indexed underwater hockey rulebook or terminology page was found via search.
- https://www.facebook.com/UWHKorea/ — Underwater Hockey Korea (한국 수중하키 모임) Facebook page. Confirmed existence of an active Korean UWH community; no accessible terminology content via search snippets.
- https://www.instagram.com/underwater_hockey_korea/ — UWH Korea 범고래 team Instagram. Confirmed Korean club activity; no terminology content accessible.
- https://archives.cmas.org/federation-list/federation-korea-underwater-association — CMAS registry page for Korea Underwater Association. Confirmed KUA affiliation; no Korean-language rules content.
- https://wiki.atlantissports.org/game — Atlantis Sports UWH Wiki (English). Yielded: overtime/sudden death protocol details, team/timeout/warning structures used to inform best-guess Korean translations.
- https://ko.wikipedia.org/wiki/%EC%88%98%EA%B5%AC — Korean Wikipedia "수구" (water polo) article. Yielded: 전반/후반, 주심, 계시원, 반칙, 퇴수 (time penalty), 페널티 스로 — useful parallel vocabulary from the closest Korean-documented aquatic team sport.
- https://ko.wikipedia.org/wiki/%EC%84%9C%EB%93%A0_%EB%8D%B0%EC%8A%A4 — Korean Wikipedia "서든 데스". Confirmed: 서든 데스 is the established Korean sports term for sudden death.
- https://ko.wikipedia.org/wiki/%ED%83%80%EC%9E%84%EC%95%84%EC%9B%83_(%EC%8A%A4%ED%8F%AC%EC%B8%A0) — Korean Wikipedia "타임아웃 (스포츠)". Confirmed: 타임아웃 is the standard Korean sports term for timeout.
- https://wordrow.kr/%EC%9D%98%EB%AF%B8/%EC%84%9C%EB%93%A0%20%EB%8D%B0%EC%8A%A4/ — Korean dictionary entry for 서든 데스. Confirmed definition: the system where the first team to score in extra time wins and the game immediately ends.
- https://www.maniareport.com/view.php?ud=2024062106075626215e8e941087_19 — Korean sports journalism column explaining 전반전/후반전 etymology and usage. Confirmed: 전반전 (first half), 후반전 (second half) are standard across Korean team sports including water sports.
- https://en.bab.la/dictionary/korean-english/%EC%8B%AC%ED%8C%90 — bab.la Korean–English entry for 심판. Confirmed: 심판 = referee/umpire (general Korean sports term).
- https://hinative.com/questions/20092175/ — HiNative entry on 청팀/백팀. Confirmed: in Korean recreational sports, teams divided by colour are typically called 청팀 (blue team) / 백팀 (white team) or by colour name + 팀. "검정팀" (black team) and "흰팀" / "백팀" (white team) are both natural for colour-coded team identification.

## Glossary (39 terms)

Trimmed to rows that correspond to concepts actually present in `en-US/refbox.ftl`.

| English (from en-US/refbox.ftl) | Korean | Source | Notes |
|---|---|---|---|
| Black team | 검정 팀 | [BEST-GUESS] | "검정" = black (colour); "팀" = team. In Korean recreational colour-coded sports, "검정팀" or "흑팀" are natural. "검정 팀" follows the dark-cap team in UWH. Scoreboard caps-style: "검정". |
| White team | 흰 팀 | [BEST-GUESS] | "흰" = white (colour). "흰팀" or "백팀" are both natural Korean alternatives. Scoreboard caps-style: "흰색". Alternative: "백팀". |
| Score | 점수 | [WIKIPEDIA-KO] | Standard Korean for score/points. Used universally across Korean sports. "득점" (deukjeom) is the scored goal itself; "점수" is the accumulated score figure on the board. Scoreboard: "점수". |
| Confirm (score) | 확인 | [BEST-GUESS] | Standard Korean UI/sports word for confirm/verify. "점수 확인" = confirm the score. |
| Foul (general) | 반칙 | [WIKIPEDIA-KO] / [NAMU-WIKI] | 반칙 (ban-chik) is the standard Korean sports term for foul/violation, used in football, ice hockey, and water polo. Widely confirmed across sources. |
| Warning (caution) | 경고 | [WIKIPEDIA-KO] | Standard Korean term for caution/warning issued by a referee. Used in football (yellow card = 경고), ice hockey, and water polo. Alternative: "주의" (lighter caution). |
| Penalty (time-served) | 페널티 | [NAMU-WIKI] | English loanword fully established in Korean sports; used in ice hockey and water polo contexts. Formal alternative: "시간 벌칙" (time punishment). |
| 30-second penalty | 30초 페널티 | [BEST-GUESS] | Constructed from "초" (second) + "페널티". The 30-second tier is not cited in Korean UWH sources; phrase is grammatically standard. Scoreboard: "30초". |
| 1-minute penalty | 1분 페널티 | [BEST-GUESS] | Constructed from "분" (minute) + "페널티". Ice hockey and water polo use "1분 퇴장" for a 1-minute exclusion. Either form is acceptable. Scoreboard: "1분". |
| 2-minute penalty | 2분 페널티 | [BEST-GUESS] | The most common timed penalty tier across Korean hockey coverage. Ice hockey equivalent: "2분 마이너 페널티". Scoreboard: "2분". |
| 4-minute penalty | 4분 페널티 | [BEST-GUESS] | Constructed; not singled out in Korean sources. Grammatically standard. Scoreboard: "4분". |
| 5-minute penalty | 5분 페널티 | [BEST-GUESS] | Ice hockey sources cite "5분 메이저 페널티". Scoreboard: "5분". |
| Total dismissal | 완전 퇴장 | [BEST-GUESS] | "퇴장" = dismissal/ejection from a game (widely used in Korean sports). "완전 퇴장" = total/permanent dismissal for the remainder of the game. Alternative: "게임 퇴장". Scoreboard abbreviation: "퇴장" (best-guess Korean equivalent of "TD"). |
| First half | 전반 | [WIKIPEDIA-KO] / [NAMU-WIKI] | 전반 (전반전) is the standard Korean term for the first half of a game, used across football, basketball, handball, and water polo. |
| Second half | 후반 | [WIKIPEDIA-KO] / [NAMU-WIKI] | 후반 (후반전) is the standard Korean term for the second half. Parallel form to 전반. |
| Half time | 하프타임 | [WIKIPEDIA-KO] | 하프타임 is the established Korean loanword for half time. Korean Wikipedia has a dedicated article. Also expressed as "전반 휴식" (first-half break) in formal contexts. |
| Between games | 경기 사이 휴식 | [BEST-GUESS] | Constructed from "경기" (game/match) + "사이" (between) + "휴식" (rest/break). No direct Korean UWH source for this UI label. Alternative: "게임 간격". |
| Overtime | 연장전 | [WIKIPEDIA-KO] | 연장전 is the established Korean sports term for overtime/extra time, used across all major Korean sports. |
| Overtime first half | 연장 전반 | [BEST-GUESS] | Constructed from "연장" (overtime) + "전반" (first half). Natural Korean compound; parallel to "연장 후반". |
| Overtime half time | 연장 하프타임 | [BEST-GUESS] | Constructed from "연장" + "하프타임". Alternative: "연장전 휴식". |
| Overtime second half | 연장 후반 | [BEST-GUESS] | Constructed from "연장" (overtime) + "후반" (second half). Natural parallel to "연장 전반". |
| Sudden death | 서든 데스 | [WIKIPEDIA-KO] | 서든 데스 is the confirmed established Korean sports term for sudden death — used in football, golf, and ice hockey. Korean Wikipedia has a dedicated article. The Korean dictionary (wordrow.kr) defines it as the system where the first team to score wins immediately. |
| Team timeout | 팀 타임아웃 | [WIKIPEDIA-KO] | 타임아웃 is the confirmed Korean sports loanword. "팀 타임아웃" = team timeout; distinguished from "심판 타임아웃" (referee timeout). |
| Referee timeout | 심판 타임아웃 | [BEST-GUESS] | Constructed from "심판" (referee) + "타임아웃". Natural Korean compound; no direct UWH source. |
| Penalty shot | 페널티 샷 | [BEST-GUESS] | English loanword compound. Ice hockey uses "페널티 샷"; water polo uses "페널티 스로". "페널티 샷" is the most natural for UWH context. |
| Referee (generic) | 심판 | [WIKIPEDIA-KO] | 심판 (sim-pan) is the universal Korean sports word for referee/umpire/official. Confirmed across all sources. |
| Chief referee | 주심 | [WIKIPEDIA-KO] | 주심 (ju-sim) = chief/head referee. Used in water polo (수구) and football. The water polo article on Korean Wikipedia uses "주심 2명" for the two head officials. |
| Water referee | 수중 심판 | [BEST-GUESS] | Constructed from "수중" (underwater/in water) + "심판" (referee). No Korean UWH source uses this exact phrase; "수중 심판" is the most natural literal Korean compound. Alternative: "수하 심판". |
| Timekeeper | 계시원 | [WIKIPEDIA-KO] | 계시원 (gye-si-won) = timekeeper. Used directly in the Korean Wikipedia water polo article ("계시원 및 간사"). Also used in Korean ice hockey officiating. |
| Scorer (score-keeper) | 기록원 | [WIKIPEDIA-KO] | 기록원 (gi-rok-won) = scorekeeper/recorder. Used in Korean ice hockey officiating ("기록원 1명") and general Korean sports administration. |
| Cap number | 모자 번호 | [BEST-GUESS] | "모자" = cap/hat; "번호" = number. Natural Korean compound. The namu.wiki UWH article notes players wear numbered caps (모자). Alternative: "캡 번호" (loanword form). |
| Player | 선수 | [WIKIPEDIA-KO] / [NAMU-WIKI] | 선수 (seon-su) is the standard Korean word for player/athlete, used universally. |
| Stick foul | 스틱 반칙 | [BEST-GUESS] | Constructed from "스틱" (stick, loanword used in UWH namu.wiki article) + "반칙" (foul). The namu.wiki UWH article uses "스틱" for the playing stick. |
| Illegal advance | 불법 전진 | [BEST-GUESS] | Constructed from "불법" (illegal) + "전진" (advance/forward movement). Standard Korean sports construction. Alternative: "규정 위반 전진". |
| Sub foul (substitution foul) | 교체 반칙 | [BEST-GUESS] | Constructed from "교체" (substitution/swap) + "반칙" (foul). "선수 교체" is the standard Korean phrase for player substitution; "교체 반칙" is the most concise form for this refbox label. |
| Illegal stoppage | 불법 정지 | [BEST-GUESS] | Constructed from "불법" (illegal) + "정지" (stoppage/halt). Alternative: "규정 위반 정지". |
| Out of bounds | 경계 밖 | [BEST-GUESS] | Standard Korean sports phrasing. "경계" = boundary/line; "밖" = outside. Alternative: "아웃 오브 바운즈" (loanword). |
| Grabbing the wall | 벽 잡기 | [BEST-GUESS] | Constructed from "벽" (wall) + "잡기" (grabbing/holding). No established Korean UWH term found; literal translation is clear and natural in Korean. |
| Obstruction | 방해 | [BEST-GUESS] | "방해" = obstruction/interference. Used in Korean sports contexts for blocking/obstruction infractions. Alternative: "차단" (blocking). |
| Delay of game | 경기 지연 | [BEST-GUESS] | Standard Korean sports construction; widely used in football broadcasts ("경기 지연"). Directly parallel to ice hockey's "게임 지연" usage. |
| Unsportsmanlike conduct | 비신사적 행위 | [WIKIPEDIA-KO] / [NAMU-WIKI] | Confirmed in Korean ice hockey sources (아이스하키 나무위키: "비신사적인 행위"). Korean Wikipedia ice hockey article also uses this exact phrase. The standard Korean sports term for unsportsmanlike conduct. |
| Free arm | 자유 팔 반칙 | [BEST-GUESS] | Literal translation: "자유" (free) + "팔" (arm) + "반칙" (foul). Korean UWH sources do not name "free arm" as a category; no established Korean equivalent found. Literal translation is the clearest option. |
| False start | 부정 출발 | [BEST-GUESS] | "부정 출발" is the standard Korean athletics and swimming term for false start (used in track and field and swimming). Widely understood in Korean sports contexts. Alternative: "잘못된 출발". |
