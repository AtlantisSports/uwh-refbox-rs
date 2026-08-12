# Source-Neutral Health Wording Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop refbox telling an operator about "the Portal" on five screens that are really about the connection, so the wording is correct whether the game source is the UWH Portal or a third-party site.

**Architecture:** Text only. Five Fluent string values change in all 15 locales, and two now-unused `portal =` arguments are deleted from their `fl!` call sites. No key is renamed, no key is added or removed, and no behaviour changes.

**Tech Stack:** Rust 2024, Fluent (`.ftl`) translation files, `fl!` macro.

**Spec:** `docs/superpowers/specs/2026-08-12-source-neutral-health-wording-design.md`

**Branch:** `fix/refbox/source-neutral-health-wording`, off `origin/master` at `526fc7c9`. The spec is already committed there at `a0856abe`.

## Global Constraints

- **The five key names do not change.** `portal-advisory-at-game-end`, `portal-summary-title`, `portal-row-token-expired`, `portal-page-attention-info`, `uwhportal-token-no-pending-link` keep their names. Only their values change.
- **`portal-login-instructions` is out of scope.** Do not touch it. It names the UWH Portal's own menus and needs per-source text, which is a separate job.
- **The `portal-row-*` siblings are out of scope** (`portal-row-stuck`, `portal-row-pending`, `portal-row-stats-pending`, `portal-row-recent`, `portal-retry-all`). They are already source-neutral.
- **`portal_name_for_mode` is not touched.** It stays where it is and keeps its nine other call sites.
- **All 15 locales get real translated text.** No English placeholders, no untranslated key left behind. The exact strings are given verbatim in the tasks below — use them as written.
- **Every string in this plan is final and approved.** Do not paraphrase, "improve", or re-localise them. String 5 in particular is the human's own literal.
- **MSRV 1.85, Rust 2024, `-D warnings`.** Unchanged by this work but still enforced by `just check`.

## No test is written for this change — deliberately

This plan departs from the usual test-first cycle, and the reason is recorded so a reviewer does not read it as an oversight:

- The repository has **no translation-coverage test at all** — nothing asserts that a key exists in every locale. This was checked, not assumed.
- A test that asserts a literal piece of display copy would fail on every future wording tweak while catching nothing a human would not see instantly on screen.

The verification that replaces it is mechanical and is spelled out in each task: a `grep` count that proves every locale was edited and that the word "Portal" is gone from exactly the five keys, plus `just check`, plus an on-screen pass.

## File Structure

**Modified — translations (15 files, same 5 keys in each):**

- `refbox/translations/{de-DE,en-US,es,fr,id-ID,it-IT,ja-JP,ko-KR,ms-MY,nl-NL,pt-PT,th-TH,tl-PH,tr-TR,zh-CN}/refbox.ftl`

**Modified — Rust (2 files, one deleted argument each):**

- `refbox/src/app/view_builders/portal_detail.rs:37-40` — the `portal-summary-title` call.
- `refbox/src/app/view_builders/portal_attention_action.rs:61-64` — the `portal-page-attention-info` call.

Nothing else needs touching, and this was verified rather than assumed: `mode` stays in use in both files (it is passed to `make_game_time_button`), and both files reach `portal_name_for_mode` through `use super::*;` — Rust does not warn on unused glob imports, so no import needs removing.

---

### Task 1: English strings and the two argument deletions

This is the whole change, visible in English. Doing it in one task means the build proves the Rust edits and the English wording together.

**Files:**
- Modify: `refbox/translations/en-US/refbox.ftl` (lines 199, 427, 429, 439, 442)
- Modify: `refbox/src/app/view_builders/portal_detail.rs:37-40`
- Modify: `refbox/src/app/view_builders/portal_attention_action.rs:61-64`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: the five final English strings, which Task 2 translates. No Rust signature changes, so nothing downstream depends on this beyond the strings themselves.

- [ ] **Step 1: Edit the five English values**

In `refbox/translations/en-US/refbox.ftl`, replace exactly these five lines. Everything else in the file stays untouched.

```ftl
uwhportal-token-no-pending-link = The connection is not expecting communication.
    Please try again.
```

(The `    Please try again.` continuation line already exists with that exact four-space indent — keep it.)

```ftl
portal-summary-title = CONNECTION STATUS
portal-row-token-expired = Access token expired — tap to re-login
portal-page-attention-info = The game result has not been accepted
portal-advisory-at-game-end = Connection issue detected. Score will still be queued — find an admin to resolve.
```

Note the em dashes (`—`) in two of these are the character already used throughout the file. Copy them, do not substitute a hyphen.

- [ ] **Step 2: Delete the argument in `portal_detail.rs`**

At `refbox/src/app/view_builders/portal_detail.rs:37`, this:

```rust
    let title = text(fl!(
        "portal-summary-title",
        portal = portal_name_for_mode(mode)
    ))
```

becomes this:

```rust
    let title = text(fl!("portal-summary-title"))
```

- [ ] **Step 3: Delete the argument in `portal_attention_action.rs`**

At `refbox/src/app/view_builders/portal_attention_action.rs:61`, this:

```rust
            text(fl!(
                "portal-page-attention-info",
                portal = portal_name_for_mode(mode)
            ))
            .size(SMALL_PLUS_TEXT),
```

becomes this:

```rust
            text(fl!("portal-page-attention-info")).size(SMALL_PLUS_TEXT),
```

- [ ] **Step 4: Build and confirm it compiles clean**

Run: `cargo build -p refbox`
Expected: builds with no warnings. If an unused-variable warning names `mode`, stop — that means `make_game_time_button` no longer takes it and the plan's assumption has gone stale; report it rather than deleting the variable.

- [ ] **Step 5: Confirm English no longer says Portal in these five places**

Run:

```bash
grep -n "portal-advisory-at-game-end\|portal-summary-title\|portal-row-token-expired\|portal-page-attention-info\|uwhportal-token-no-pending-link" refbox/translations/en-US/refbox.ftl
```

Expected: five lines, none containing the word `Portal` and none containing `{ $portal }`.

- [ ] **Step 6: Commit**

```bash
git add refbox/translations/en-US/refbox.ftl refbox/src/app/view_builders/portal_detail.rs refbox/src/app/view_builders/portal_attention_action.rs
git commit -m "fix(refbox): make the English health wording source-neutral"
```

---

### Task 2: The remaining 14 locales

**Files:**
- Modify: the `refbox.ftl` of `de-DE`, `es`, `fr`, `id-ID`, `it-IT`, `ja-JP`, `ko-KR`, `ms-MY`, `nl-NL`, `pt-PT`, `th-TH`, `tl-PH`, `tr-TR`, `zh-CN`

**Interfaces:**
- Consumes: the five English strings finalised in Task 1.
- Produces: nothing further depends on this.

Two things to know before starting, both established by reading the files rather than guessed:

1. **Spanish and French do not currently say "portal" in `uwhportal-token-no-pending-link`.** They say "no pending link was found" — a different sentence from the English. They are being retranslated to the new English meaning, not merely having a word removed. That is intended.
2. **`portal-row-token-expired` must match each language's own `access-token` label**, which is the row the operator lands beside after tapping it. The label values are already in each file; the strings below were written from them.

- [ ] **Step 1: Apply `portal-advisory-at-game-end` in all 14**

```
de-DE  Verbindungsproblem erkannt. Spielstand wird trotzdem in die Warteschlange gestellt — bitte einen Administrator zur Lösung kontaktieren.
es     Problema de conexión detectado. El resultado se encolará igualmente — busca a un administrador para resolverlo.
fr     Problème de connexion détecté. Le résultat sera tout de même mis en file d'attente — trouvez un administrateur pour le résoudre.
id-ID  Masalah koneksi terdeteksi. Skor tetap akan diantrekan — temui admin untuk menyelesaikan.
it-IT  Rilevato problema di connessione. Il punteggio sarà comunque accodato — contatta un amministratore.
ja-JP  接続の問題を検出しました。スコアはキューに残ります — 管理者に解決を依頼してください。
ko-KR  연결 문제가 감지되었습니다. 점수는 계속 대기열에 있습니다 — 관리자에게 문의하세요.
ms-MY  Masalah sambungan dikesan. Markah masih akan dibariskan — cari admin untuk selesaikan.
nl-NL  Verbindingsprobleem gedetecteerd. Score blijft in wachtrij — zoek een beheerder om het op te lossen.
pt-PT  Problema de ligação detetado. O resultado será mantido em fila — contacte um administrador para resolver.
th-TH  ตรวจพบปัญหาการเชื่อมต่อ คะแนนจะยังคงอยู่ในคิว — ติดต่อผู้ดูแลเพื่อแก้ไข
tl-PH  May problema sa koneksyon. Ipipila pa rin ang puntos — humanap ng admin para ayusin.
tr-TR  Bağlantı sorunu algılandı. Skor yine de sıraya alınacak — çözmek için bir yöneticiye başvurun.
zh-CN  检测到连接问题。比分仍会排队 — 请联系管理员解决。
```

- [ ] **Step 2: Apply `portal-summary-title` in all 14**

This is a page title and is upper-case in the Latin-script locales, matching how each file already writes it.

```
de-DE  VERBINDUNGSSTATUS
es     ESTADO DE CONEXIÓN
fr     STATUT DE CONNEXION
id-ID  STATUS KONEKSI
it-IT  STATO CONNESSIONE
ja-JP  接続状態
ko-KR  연결 상태
ms-MY  STATUS SAMBUNGAN
nl-NL  VERBINDINGSSTATUS
pt-PT  ESTADO DA LIGAÇÃO
th-TH  สถานะการเชื่อมต่อ
tl-PH  STATUS NG KONEKSYON
tr-TR  BAĞLANTI DURUMU
zh-CN  连接状态
```

Each of these loses its `{ $portal }` placeholder entirely.

- [ ] **Step 3: Apply `portal-row-token-expired` in all 14**

```
de-DE  Zugriffstoken abgelaufen — zum erneuten Anmelden tippen
es     Token de acceso expirado — toca para iniciar sesión
fr     Jeton d'accès expiré — touchez pour vous reconnecter
id-ID  Token akses kedaluwarsa — ketuk untuk login ulang
it-IT  Token di accesso scaduto — tocca per accedere di nuovo
ja-JP  アクセストークンの有効期限が切れました — タップして再ログイン
ko-KR  액세스 토큰 만료됨 — 탭하여 다시 로그인
ms-MY  Token akses tamat tempoh — ketik untuk log masuk semula
nl-NL  Toegangstoken verlopen — tik om opnieuw in te loggen
pt-PT  Token de acesso expirou — toque para iniciar sessão novamente
th-TH  โทเค็นการเข้าถึงหมดอายุ — แตะเพื่อเข้าสู่ระบบใหม่
tl-PH  Nag-expire ang access token — pindutin para mag-login muli
tr-TR  Erişim tokeni sona erdi — yeniden giriş için dokunun
zh-CN  访问令牌已过期 — 点击重新登录
```

- [ ] **Step 4: Apply `portal-page-attention-info` in all 14**

Each of these loses its `{ $portal }` placeholder and its trailing "on/by … Portal" phrase.

```
de-DE  Das Spielergebnis wurde nicht angenommen
es     El resultado del juego no ha sido aceptado
fr     Le résultat du match n'a pas été accepté
id-ID  Hasil pertandingan belum diterima
it-IT  Il risultato della partita non è stato accettato
ja-JP  試合結果が受理されていません
ko-KR  경기 결과가 수락되지 않았습니다
ms-MY  Keputusan perlawanan tidak diterima
nl-NL  De wedstrijduitslag is niet geaccepteerd
pt-PT  O resultado do jogo não foi aceite
th-TH  ผลเกมยังไม่ได้รับการยอมรับ
tl-PH  Hindi pa tinatanggap ang resulta ng laro
tr-TR  Oyun sonucu kabul edilmedi
zh-CN  比赛结果尚未被接受
```

Note for `fr`: the line being replaced contains a typo, `n.a pas été accepté`. The replacement above spells it `n'a`. That is a correction of the line this change is already rewriting, not separate scope creep.

- [ ] **Step 5: Apply `uwhportal-token-no-pending-link` in all 14**

Each is two lines: the sentence, then the existing continuation line. The continuation text is unchanged from what is already in each file and is repeated here so it is not lost.

**Indentation warning.** The continuation lines are indented further below only so this table lines up on the page. In the file each continuation line takes **exactly four spaces**, matching what is already there and matching the `uwhportal-token-invalid-code` entry directly above it. Fluent treats the indent as significant — getting it wrong changes the rendered text. The safest edit is to replace only the first line of each entry and leave the existing continuation line untouched on disk.

```
de-DE  Die Verbindung erwartet keine Kommunikation.
           Bitte erneut versuchen.
es     La conexión no espera comunicación.
           Por favor, inténtelo de nuevo.
fr     La connexion n'attend aucune communication.
           Veuillez réessayer.
id-ID  Koneksi tidak mengharapkan komunikasi.
           Silakan coba lagi.
it-IT  La connessione non si aspetta comunicazioni.
           Riprova.
ja-JP  接続は通信を待っていません。
           もう一度試してください。
ko-KR  연결이 통신을 기다리지 않습니다.
           다시 시도하세요.
ms-MY  Sambungan tidak menjangka komunikasi.
           Sila cuba lagi.
nl-NL  De verbinding verwacht geen communicatie.
           Probeer het opnieuw.
pt-PT  A ligação não está à espera de comunicação.
           Tente novamente.
th-TH  การเชื่อมต่อไม่รอการสื่อสาร
           โปรดลองอีกครั้ง
tl-PH  Hindi inaasahan ng koneksyon ang komunikasyon.
           Pakisubukan muli.
tr-TR  Bağlantı iletişim beklemiyor.
           Lütfen tekrar deneyin.
zh-CN  连接未等待通信。
           请重试。
```

- [ ] **Step 6: Prove every locale was edited and none was missed**

Run:

```bash
grep -c "portal-advisory-at-game-end\|portal-summary-title\|portal-row-token-expired\|portal-page-attention-info\|uwhportal-token-no-pending-link" refbox/translations/*/refbox.ftl
```

Expected: **15 files, each reporting `5`.** A file reporting fewer means a key was accidentally renamed or deleted.

Then run:

```bash
grep -n "Portal\|portal\|{ \$portal }" refbox/translations/*/refbox.ftl | grep "portal-advisory-at-game-end\|portal-summary-title\|portal-row-token-expired\|portal-page-attention-info\|uwhportal-token-no-pending-link"
```

Expected: **only the key names themselves match** — no line may still contain `{ $portal }`, and no line's *value* may contain the word Portal in any casing.

- [ ] **Step 7: Run the full gate**

Run: `just check`
Expected: exit 0. Formatting, `-D warnings`, the full test suite and the security scan all clean.

- [ ] **Step 8: Commit**

```bash
git add refbox/translations/
git commit -m "fix(refbox): make the health wording source-neutral in all locales"
```

---

## Final verification, before the PR

- [ ] `just check` exit 0 at the final tree.
- [ ] `git diff --stat origin/master` shows only the 15 `.ftl` files, the 2 Rust files, and the spec and plan docs. Anything else is scope creep.
- [ ] On screen, with the game source set to CUSTOM and the stub running: the game-end advisory, the health page title, and the submission-error page all read neutrally.
- [ ] On screen, with the game source set to UWH PORTAL: the same screens still read correctly. The neutral wording has to be *accurate* there, not merely tolerable.

String 3 (expired access token) and string 5 (no pending link) are the two that need arranging rather than merely observing — string 3 needs a rejected token with an event selected, string 5 needs a link attempt the far end refuses with `NoPendingLink`. If they cannot be produced in reasonable time, say so plainly in the PR rather than implying they were seen.

## Deviations

_Record anything that diverged from this plan here, rather than in standalone commits._
