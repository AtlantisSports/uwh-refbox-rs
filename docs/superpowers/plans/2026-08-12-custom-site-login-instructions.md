# Custom-Site Login Instructions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The access-token page stops giving UWH Portal menu directions to an operator using a custom third-party site.

**Architecture:** A sibling translation key chosen by the game source. `ViewData` gains `source`, the page picks between two strings, and nothing about linking changes.

**Tech Stack:** Rust 2024, Fluent `.ftl`, iced 0.13.

**Spec:** `docs/superpowers/specs/2026-08-12-custom-site-login-instructions-design.md`

**Branch:** `fix/refbox/custom-site-login-instructions`, off `origin/master` at `adef2f6d`. Spec committed at `c577f8a3`.

## Global Constraints

- **`portal-login-instructions` is not modified.** A Portal operator keeps the exact menu path.
- **The new English text is the human's literal.** Do not paraphrase or "improve" it.
- **The new key uses `{ $id }` only** — no `{ $portal }`, no other variable.
- **All 15 locales get real translated text.** The translation-consistency tests merged in #2269 will fail the build otherwise, which is the intended safety net.
- **Each locale's text names its own DONE button label**, listed per language in Task 2.
- **MSRV 1.85, Rust 2024, `-D warnings`, `cargo fmt`.**

## File Structure

- `refbox/src/app/view_data.rs` — add `source: GameSource` to `ViewData`
- `refbox/src/app/mod.rs` — populate it where `ViewData` is constructed
- `refbox/src/app/view_builders/keypad_pages/mod.rs:180` — pass `source` to the page
- `refbox/src/app/view_builders/keypad_pages/portal_login.rs` — take `source`, choose the string
- `refbox/translations/*/refbox.ftl` — new key `custom-login-instructions` ×15

---

### Task 1: The English string and the source-aware page

**Files:**
- Modify: `refbox/src/app/view_data.rs`
- Modify: `refbox/src/app/mod.rs` (ViewData construction)
- Modify: `refbox/src/app/view_builders/keypad_pages/mod.rs`
- Modify: `refbox/src/app/view_builders/keypad_pages/portal_login.rs`
- Modify: `refbox/translations/en-US/refbox.ftl`

- [ ] **Step 1: Add the English key**

In `refbox/translations/en-US/refbox.ftl`, directly after the `portal-login-instructions` entry, add:

```ftl
custom-login-instructions = Please provide this Refbox ID to your site:
    { $id }

    Then enter the confirmation code that your site provides using the number pad and press DONE
```

The four-space indent on continuation lines is significant to Fluent — match the entry above it.

- [ ] **Step 2: Add `source` to `ViewData`**

In `refbox/src/app/view_data.rs`, add the field with a comment saying why it is there:

```rust
    /// Which game source is selected. The access-token page needs it to choose
    /// between the UWH Portal's menu directions and the generic wording a
    /// third-party site needs — refbox cannot know a custom site's admin
    /// screens, so the two cannot be merged into one string.
    pub(super) source: GameSource,
```

Import `GameSource` alongside the existing `use super::Mode;`.

- [ ] **Step 3: Populate it**

Find where `ViewData { .. }` is constructed in `refbox/src/app/mod.rs` and add `source: self.source,` beside the existing `mode:` field.

- [ ] **Step 4: Pass it to the page**

In `refbox/src/app/view_builders/keypad_pages/mod.rs`, destructure `source` from `ViewData` beside `mode`, and change the call at the `KeypadPage::PortalLogin` arm:

```rust
                KeypadPage::PortalLogin(id, requested) => {
                    make_portal_login_page(id, requested, mode, source)
                }
```

- [ ] **Step 5: Choose the string**

In `refbox/src/app/view_builders/keypad_pages/portal_login.rs`, take the new parameter and branch:

```rust
pub(super) fn make_portal_login_page<'a>(
    id: u32,
    requested: bool,
    mode: Mode,
    source: GameSource,
) -> Element<'a, Message> {
    // A custom site gets generic wording: refbox knows the address the operator
    // typed, not what that site calls its admin screens, so naming menus the
    // way the Portal's own instructions do would be a guess. Manual never
    // reaches this page.
    let instructions = if source == GameSource::Custom {
        fl!("custom-login-instructions", id = id)
    } else {
        fl!(
            "portal-login-instructions",
            id = id,
            portal = portal_name_for_mode(mode)
        )
    };

    column![
        text(instructions).width(Length::Fill),
```

The rest of the function is unchanged.

- [ ] **Step 6: Build**

Run: `cargo build -p refbox`
Expected: compiles with no warnings. If `mode` is now reported unused, it is not — the portal branch still uses it; a warning there means the branch was written wrongly.

- [ ] **Step 7: Commit**

```bash
git add refbox/src refbox/translations/en-US/refbox.ftl
git commit -m "fix(refbox): give a custom site its own linking instructions"
```

---

### Task 2: The remaining 14 locales

**Files:**
- Modify: the `refbox.ftl` of `de-DE`, `es`, `fr`, `id-ID`, `it-IT`, `ja-JP`, `ko-KR`, `ms-MY`, `nl-NL`, `pt-PT`, `th-TH`, `tl-PH`, `tr-TR`, `zh-CN`

Insert the key after each file's `portal-login-instructions` entry. Every entry is three lines: the sentence, the indented `{ $id }`, a blank line, then the indented closing sentence — matching the English shape.

Each language names **its own** DONE button label, taken from that file's `done` key.

- [ ] **Step 1: Add the key in all 14**

```
de-DE  Bitte geben Sie diese Refbox-ID an Ihre Website weiter:
       Geben Sie dann den Bestätigungscode Ihrer Website über das Nummernfeld ein und drücken Sie FERTIG

es     Proporcione este ID de Refbox a su sitio:
       Luego introduzca el código de confirmación que le proporcione su sitio con el teclado numérico y pulse HECHO

fr     Veuillez fournir cet identifiant Refbox à votre site :
       Saisissez ensuite le code de confirmation fourni par votre site à l'aide du pavé numérique et appuyez sur TERMINÉ

id-ID  Harap berikan ID Refbox ini ke situs Anda:
       Lalu masukkan kode konfirmasi yang diberikan situs Anda menggunakan papan angka dan tekan SELESAI

it-IT  Fornisci questo ID Refbox al tuo sito:
       Poi inserisci il codice di conferma fornito dal tuo sito usando il tastierino numerico e premi FATTO

ja-JP  この Refbox ID をご利用のサイトに伝えてください:
       次に、サイトから提供された確認コードをテンキーで入力し、完了 を押してください

ko-KR  이 Refbox ID를 사용 중인 사이트에 알려주세요:
       그런 다음 사이트에서 제공한 확인 코드를 숫자판으로 입력하고 완료 를 누르세요

ms-MY  Sila berikan ID Refbox ini kepada tapak anda:
       Kemudian masukkan kod pengesahan yang diberikan tapak anda menggunakan papan nombor dan tekan SELESAI

nl-NL  Geef dit Refbox-ID door aan uw site:
       Voer daarna de bevestigingscode van uw site in met het cijferblok en druk op KLAAR

pt-PT  Forneça este ID de Refbox ao seu site:
       Depois introduza o código de confirmação fornecido pelo seu site no teclado numérico e prima CONCLUÍDO

th-TH  โปรดแจ้งรหัส Refbox นี้แก่เว็บไซต์ของคุณ:
       จากนั้นป้อนรหัสยืนยันที่เว็บไซต์ของคุณให้มาโดยใช้แป้นตัวเลข แล้วกด เสร็จสิ้น

tl-PH  Pakibigay ang Refbox ID na ito sa inyong site:
       Pagkatapos ay ilagay ang confirmation code na ibinigay ng inyong site gamit ang number pad at pindutin ang TAPOS

tr-TR  Lütfen bu Refbox kimliğini sitenize iletin:
       Ardından sitenizin verdiği onay kodunu sayı tuşlarıyla girin ve TAMAM tuşuna basın

zh-CN  请将此 Refbox ID 提供给您的站点:
       然后使用数字键盘输入您的站点提供的确认码，并按 完成
```

- [ ] **Step 2: Prove every locale has it**

```bash
grep -c "^custom-login-instructions" refbox/translations/*/refbox.ftl
```

Expected: all 15 report `1`.

- [ ] **Step 3: Run the gate**

Run: `just check`
Expected: exit 0. The translation-consistency tests merged in #2269 independently confirm every locale has the key with `{ $id }` intact, and that the key is used.

- [ ] **Step 4: Commit**

```bash
git add refbox/translations/
git commit -m "fix(refbox): translate the custom-site linking instructions"
```

---

## Final verification, before the PR

- [ ] `just check` exit 0.
- [ ] `git diff --stat origin/master...HEAD` shows only: 4 Rust files, 15 `.ftl` files, 2 docs.
- [ ] **On screen, with the source set to CUSTOM:** the access-token page shows the new text and the word "Portal" appears nowhere on it.
- [ ] **On screen, with the source set to UWH PORTAL:** the page still shows the full Event Management / Referee Management menu path.

Both screen checks need the operator at the keypad page, which is reached by tapping the ACCESS TOKEN row — see the recorded walkthrough recipe.

## Deviations

_Record anything that diverged from this plan here, rather than in standalone commits._
