# Custom-site login instructions — design

**Date:** 2026-08-12
**Status:** approved by Eric, not yet implemented
**Scope:** `refbox` only — one new translation key ×15, one view-data field, one conditional

## The problem

The access-token page tells the operator:

> Please go to the **UWH Portal >> Event Management >> Referee Management**, click on the + button
> to add a new Refbox, and enter this Refbox ID: …
>
> The **UWH Portal** will then provide a confirmation code for you to enter to the left using the
> number pad.

Under a custom third-party site those are instructions for a product the operator is not using.
Seen on screen 2026-08-12 during the linking walkthrough, with the game source set to CUSTOM.

This is the sixth Portal-worded string, and it was deliberately excluded from the five fixed in
#2219 for a reason that still holds: **the other five describe a connection and could simply drop
the word. This one describes navigating a specific product's menus, and refbox cannot know what a
third-party site's admin screens are called.** Neutral wording alone will not do — it needs
different text per source. Recorded as gap 15 of the third-party contract document.

## The decision

**Add a sibling key, do not rewrite the existing one.** A real UWH Portal operator should keep the
precise menu path, which is correct and useful for them. `portal-login-instructions` is untouched.

**The new text is Eric's literal — implement it exactly:**

```
Please provide this Refbox ID to your site:
    { $id }

    Then enter the confirmation code that your site provides using the number pad and press DONE
```

It carries **no `{ $portal }` variable at all** — only `{ $id }`. Nothing on the page mentions a
Portal when the source is custom.

Why "your site" rather than naming the address: the address the operator typed is the API endpoint,
not a page an administrator logs into. Substituting it would send someone to a URL that shows them
nothing.

## Architecture

The page is built by `make_portal_login_page` in
`refbox/src/app/view_builders/keypad_pages/portal_login.rs`, called from exactly one place —
`keypad_pages/mod.rs:180` — and currently receives `id`, `requested` and `mode`.

It does not receive `source`, which is what it needs to choose between the two strings.
`ViewData` (`refbox/src/app/view_data.rs`) carries `mode`, `clock_running`, `teams`,
`portal_indicator`, `has_led_panel` and `committed_site_url`, but not `source`.

**Add `source: GameSource` to `ViewData`** and pass it through to the page. This mirrors the
existing precedent: `ConfirmationKind::PortalTenantSwitch` was given `source` for exactly this
reason when the mode-switch warning needed different wording per source, and `committed_site_url`
was added to `ViewData` on the same feature.

Rejected alternative: inferring the source from `committed_site_url.is_empty()`. It happens to
correlate today, but it encodes "custom" as a side effect of another field, and a future change to
when that URL is populated would silently switch the operator's instructions.

## Acceptance criteria

1. With the game source set to **CUSTOM**, the access-token page shows the new text and the word
   "Portal" does not appear anywhere on it.
2. With the game source set to **UWH PORTAL**, the page is unchanged — the full menu path is still
   shown.
3. The Refbox ID renders on its own line in both, as it does today.
4. All 15 languages have the new key, translated, with `{ $id }` intact.
5. `just check` passes, including the translation-consistency tests merged in #2269 — which will
   themselves catch a missed locale or a mangled `{ $id }`.

## Explicitly not in scope

- **The linking mechanism.** The Refbox ID, the call to the site, the code entry and the DONE
  button are untouched. This is display text only.
- **`portal-login-instructions`**, which keeps its exact current wording.
- **The five strings merged in #2219.**
- **Manual mode**, which never reaches this page.
