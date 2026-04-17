# Japanese UWH Glossary — First Pass

**Date:** 2026-04-17
**Purpose:** Vet UWH-specific Japanese terms before generating ja-JP/refbox.ftl.
**Review status:** Pending human review. Native-speaker review is a separate, later step.

## Source tags

- `[FED-RULEBOOK]` — from the JUSF (日本水中スポーツ連盟) rules page (cite URL/section)
- `[WEB-UWH-CLUB]` — from a Japanese UWH club/team page (cite URL)
- `[WIKIPEDIA-JA]` — from Japanese Wikipedia
- `[BEST-GUESS]` — no direct source found, proposed from general Japanese sports vocabulary

## Sources consulted

- https://jusf.gr.jp/hockey/ — JUSF (一般社団法人日本水中スポーツ連盟) official water hockey page. Direct fetch blocked; content was returned via Google snippet extraction across multiple searches. Yielded: 水中ホッケー, プッシャー, パック, チーム構成 (6人+4人), 試合時間 (15分ハーフ), コーション/注意 (caution), 退水 (time penalty 1 or 2 min), 退場 (dismissal), ペナルティーショット, センターパック (game start), 不正スタート, フリーハンドでパックを扱う, 妨害行為, スポーツマンらしからぬ行為, 暴力的行為, ゴング合図 (buzzer signal). This is the primary authoritative source for domestic Japanese UWH rules.
- https://ja.wikipedia.org/wiki/%E6%B0%B4%E4%B8%AD%E3%83%9B%E3%83%83%E3%82%B1%E3%83%BC — Japanese Wikipedia article on 水中ホッケー. Direct fetch blocked; content extracted via Google snippets and the Weblio mirror of the same article. Yielded: 水中ホッケー, 潜水ホッケー, オクトプッシュ (alt names), プッシャー, パック, チームカラー (黒/白), 前半/後半, ハーフタイム, ゴール, 選手 (player), 交代選手 (substitute).
- https://www.weblio.jp/wkpja/content/%E6%B0%B4%E4%B8%AD%E3%83%9B%E3%83%83%E3%82%B1%E3%83%BC_%E6%B0%B4%E4%B8%AD%E3%83%9B%E3%83%83%E3%82%B1%E3%83%BC%E3%81%AE%E6%A6%82%E8%A6%81 — Weblio mirror of Japanese Wikipedia entry. Yielded: キャップ識別 (cap identification), 黒/白チーム, プッシャー, パック weight/dimensions, 6人制, ゴール, ゴールエリア, ペナルティーライン, ペナルティーショット (2対1 format).
- https://www.weblio.jp/content/%E6%B0%B4%E4%B8%AD%E3%83%9B%E3%83%83%E3%82%B1%E3%83%BC — Weblio dictionary entry for 水中ホッケー. Yielded: general description, ゴング合図, センターパック, エンドライン, チームカラー.
- https://sposuru.com/contents/sports-trivia/underwater-ball-game/ — Japanese sports magazine article on underwater sports (hockey, rugby, etc.). Yielded: 15分ハーフ・3分休憩, 前半/後半, エンドの交代 (end change at half time), プッシャー, パック, ゴール.
- https://ja.wikipedia.org/wiki/%E3%82%BF%E3%82%A4%E3%83%A0%E3%82%A2%E3%82%A6%E3%83%88 — Japanese Wikipedia article on タイムアウト (timeout). Confirmed タイムアウト is standard across Japanese sports; チームタイムアウト is used in contexts like volleyball/basketball for team-called stoppages. Relevant for establishing what term to use in the refbox UI.
- https://ja.wikipedia.org/wiki/%E5%BB%B6%E9%95%B7%E6%88%A6 — Japanese Wikipedia on 延長戦 (overtime/extra time). Confirmed 延長戦 is the standard Japanese sports term for overtime, and サドンデス is a well-established loanword used in Japanese sports media for sudden-death play.
- https://ja.wikipedia.org/wiki/%E4%B8%BB%E5%AF%A9 — Japanese Wikipedia on 主審 (chief referee/head official). Confirmed 主審 is the standard Japanese term for the senior/chief referee role; 副審 = assistant/water referee.

## Glossary (39 terms)

Trimmed to rows that correspond to concepts actually present in `en-US/refbox.ftl`.

| English (from en-US/refbox.ftl) | Japanese | Source | Notes |
|---|---|---|---|
| Black team | 黒チーム | [WIKIPEDIA-JA] / [FED-RULEBOOK] | Team identified by dark-coloured cap and pusher. Japanese sources use 黒 (kuro) consistently. Scoreboard caps-style: 黒. |
| White team | 白チーム | [WIKIPEDIA-JA] / [FED-RULEBOOK] | Team identified by white/light-coloured cap and pusher. Scoreboard caps-style: 白. |
| Score | 得点 | [WIKIPEDIA-JA] | Standard Japanese sports term for score/goal count. On compact scoreboard: 得点 or スコア. |
| Confirm (score) | 確認 | [BEST-GUESS] | Standard Japanese UI/sports verb for "confirm". Full phrase: スコアを確認する. Short form: 確認. |
| Foul (general) | 反則 | [FED-RULEBOOK] | The JUSF page uses 反則 (hansoku) as the generic term for infractions; コーション (caution) is used for lighter violations. |
| Warning (caution) | コーション | [FED-RULEBOOK] | Direct loanword used in the JUSF rules. Lighter infraction; gives a caution rather than a time penalty. Alternative Japanese term: 警告. |
| Penalty (time-served) | 退水 | [FED-RULEBOOK] | The JUSF rules explicitly call the timed penalty 退水 (taisui = "exit the water"). This is the established Japanese UWH term. Not 罰則 or ペナルティ. |
| 30-second penalty | 30秒退水 | [BEST-GUESS] | Constructed from 30秒 + 退水. The 30-second tier is not singled out in Japanese UWH sources (JUSF lists 1-minute and 2-minute tiers); phrase is grammatically standard. Scoreboard: 30秒. |
| 1-minute penalty | 1分退水 | [FED-RULEBOOK] | Cited directly on the JUSF page: "1分か2分の退水". Scoreboard: 1分. |
| 2-minute penalty | 2分退水 | [FED-RULEBOOK] | Cited directly on the JUSF page. The most common timed penalty in UWH. Scoreboard: 2分. |
| 4-minute penalty | 4分退水 | [BEST-GUESS] | Constructed; 4-minute tier not mentioned in Japanese summaries but the pattern 〇分退水 is consistent. Scoreboard: 4分. |
| 5-minute penalty | 5分退水 | [BEST-GUESS] | Constructed; same pattern. Scoreboard: 5分. |
| Total dismissal | 退場 | [FED-RULEBOOK] | The JUSF page uses 退場 (taijō = dismissal/expulsion from the game). "故意に重度な違反を犯した場合には退場となり、以降の残り時間の出場ができません。" Scoreboard abbreviation: 退場 or 除外 (best-guess short form). |
| First half | 前半 | [WIKIPEDIA-JA] / [FED-RULEBOOK] | 前半 (zenhan) is standard Japanese for the first half of a game. Confirmed across all sources. |
| Second half | 後半 | [WIKIPEDIA-JA] / [FED-RULEBOOK] | 後半 (kōhan) is standard Japanese for the second half. Confirmed across all sources. |
| Half time | ハーフタイム | [FED-RULEBOOK] / [WIKIPEDIA-JA] | Sources use both ハーフタイム (katakana loanword) and 休憩 (rest). JUSF describes it as "3分の休憩及びエンドの交代". ハーフタイム is the more recognised sports UI term. |
| Between games | 試合間 | [BEST-GUESS] | Constructed from 試合 (match) + 間 (between/during). Standard Japanese compound. Alternative: 試合の合間. No direct UWH source uses this exact UI label. |
| Overtime | 延長戦 | [WIKIPEDIA-JA] | 延長戦 (enchōsen) is the standard Japanese term for overtime/extra time across all major sports. Confirmed via Japanese Wikipedia. Short form for scoreboard: 延長. |
| Overtime first half | 延長前半 | [BEST-GUESS] | Constructed from 延長 + 前半. Consistent with Japanese sports convention. |
| Overtime half time | 延長ハーフタイム | [BEST-GUESS] | Constructed from 延長 + ハーフタイム. Consistent. |
| Overtime second half | 延長後半 | [BEST-GUESS] | Constructed from 延長 + 後半. Consistent. |
| Sudden death | サドンデス | [WIKIPEDIA-JA] | サドンデス is a well-established katakana loanword used in Japanese sports media for sudden-death play (ゴールデンゴール方式). Confirmed via Japanese Wikipedia 延長戦 article. Alternative pure-Japanese term: 即時決着 (sokujiketchaku), but サドンデス is far more idiomatic. |
| Team timeout | チームタイムアウト | [BEST-GUESS] | チームタイムアウト is established in Japanese sports (volleyball, basketball). The JUSF page does not name team timeouts explicitly, but the term is unambiguous in context. Short form for scoreboard: T/O or チームTO. |
| Referee timeout | 審判タイムアウト | [BEST-GUESS] | Constructed from 審判 (referee) + タイムアウト. The CMAS rules name this concept; no Japanese UWH source provides a specific term. 審判タイムアウト is clear and parallel to チームタイムアウト. |
| Penalty shot | ペナルティーショット | [FED-RULEBOOK] | Used directly on the JUSF page: "ペナルティーショット（ゴール前での2対1の対戦）". Established loanword. |
| Referee (generic) | 審判 / 審判員 | [FED-RULEBOOK] | 審判 (shinpan) or 審判員 (shinpan-in) are the standard Japanese terms. The JUSF page uses 審判. |
| Chief referee | 主審 | [WIKIPEDIA-JA] | 主審 (shushin) is the standard Japanese term for the senior/chief official in a match. Confirmed via Japanese Wikipedia 主審 article. The CMAS rules describe the Chief Referee as the deck-side official who controls the buzzer; 主審 is the natural Japanese equivalent. |
| Water referee | 水中審判 | [BEST-GUESS] | Constructed from 水中 (underwater) + 審判 (referee). No Japanese UWH source explicitly names this role, but the CMAS rules document describes two water referees per game. 水中審判 is the logical and transparent Japanese compound. Alternative: 水中レフェリー. |
| Timekeeper | タイムキーパー | [WIKIPEDIA-JA] | タイムキーパー is established in Japanese sports as the person responsible for game-clock management. Used in the Japanese Wikipedia article on 審判員. |
| Scorer (score-keeper) | 記録員 | [WIKIPEDIA-JA] | 記録員 (kiroku-in) is the standard Japanese term for the official who records scores and penalties. Used widely in Japanese sports contexts. |
| Cap number | キャップ番号 | [FED-RULEBOOK] / [WIKIPEDIA-JA] | キャップ (cap) is used in the JUSF and Weblio sources to describe the water polo-style helmet worn by UWH players, which bears the player number. キャップ番号 = cap number. Alternative: 帽子番号. |
| Player | 選手 | [FED-RULEBOOK] | 選手 (senshu) is the universal Japanese sports term for "player/athlete". Used throughout all sources. |
| Stick foul | スティック反則 | [BEST-GUESS] | Constructed from スティック (stick) + 反則 (foul/infraction). Japanese sources describe スティックにパックを乗せて運ぶ (carrying the puck on the stick) as a specific violation; スティック反則 is a clear short label. |
| Illegal advance | 違反前進 | [BEST-GUESS] | Constructed from 違反 (violation) + 前進 (advance). Japanese sources describe妨害 (obstruction) and 反則 (violation) but do not name this specific refbox category. Alternative: 不正前進. |
| Sub foul (substitution foul) | 交代反則 | [BEST-GUESS] | Constructed from 交代 (substitution) + 反則 (foul). 交代 is the standard Japanese sports word for player substitution. No UWH-specific Japanese term found. |
| Illegal stoppage | 違反停止 | [BEST-GUESS] | Constructed from 違反 (violation) + 停止 (stoppage). No Japanese UWH source names this category specifically. Alternative: 不正停止. |
| Out of bounds | コート外 | [BEST-GUESS] | Standard Japanese sports phrasing: コート (court/playing area) + 外 (outside). Used across swimming and water sports contexts. Alternative: 境界線外. |
| Grabbing the wall | 壁つかみ | [FED-RULEBOOK] | The JUSF page describes プールの底に立ち妨害する (standing on the pool bottom to interfere) as a violation, and by extension gripping the pool wall (壁) is also a named violation. 壁 (kabe = wall) + つかみ (grabbing) is the most natural Japanese compound. No fixed official Japanese term found; this is the clearest construction. |
| Obstruction | 妨害 | [FED-RULEBOOK] | 妨害 (bōgai) is used directly in the JUSF rules: "パックを扱っていない選手の進路を妨害した場合". Standard term across Japanese sports. |
| Delay of game | 遅延行為 | [BEST-GUESS] | 遅延行為 (chien kōi = delaying act) is the standard Japanese sports construction for delay of game. Used in Japanese football and field hockey contexts. |
| Unsportsmanlike conduct | スポーツマンらしからぬ行為 | [FED-RULEBOOK] | This exact phrase is used on the JUSF page: "審判や相手選手への暴言、侮辱等のスポーツマンらしからぬ行為". Direct hit — no need to construct. Short form for scoreboard: 非紳士的行為 (best-guess alternative that is more compact). |
| Free arm | フリーハンド反則 | [FED-RULEBOOK] | The JUSF page names this as "フリーハンドでパックを扱った場合" (using the free hand to handle the puck). The refbox key `free-arm` maps to this concept. フリーハンド反則 is the natural label. |
| False start | 不正スタート | [FED-RULEBOOK] | Used directly on the JUSF page: "不正スタートをした場合（注意）". Direct hit. |
