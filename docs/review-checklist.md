# Review Checklist

Use this checklist every time Claude proposes a change and asks you to review or merge it. You do
not need to read any code to use this checklist — it is designed to help you verify that the
right things happened without requiring programming knowledge.

---

## Before Approving Any Change

Work through these steps in order. If any step fails, ask Claude to explain or fix it before
proceeding.

---

### Step 1 — Does CI pass?

Go to the pull request on GitHub and look for the status checks at the bottom of the page.

- **All green checkmarks** = CI passed. Proceed to step 2.
- **Any red X** = CI failed. Ask Claude: *"CI failed — what went wrong and how do we fix it?"*

Do not merge until all checks are green.

---

### Step 2 — Does the branch name match the convention?

Check the branch name (shown at the top of the PR page).

It should follow the format: `type/scope/description`

Examples of correct names:
- `fix/refbox/confirm-score-timing`
- `feat/schedule-processor/list-of-placements`
- `hotfix/uwh-common/wire-format-version`

If it does not follow this format, ask Claude to explain why — it may be a grandfathered legacy
branch (like `pr/*` branches), which is acceptable.

---

### Step 3 — Does the PR title and description match what you asked for?

Read the PR title and the description body.

- Does the title describe the change you requested?
- Does the plain-language summary in the body match what you asked Claude to do?
- Is there a "how to verify" section that tells you what to look for?

If the description is missing or doesn't match what you asked, ask Claude to update it before
merging.

---

### Step 4 — Are there unexpected files in the diff?

Click the **"Files changed"** tab in the pull request.

Look through the list of changed files. Ask yourself:

- Do all of these files make sense for what was requested?
- Are there any files you didn't expect to see?
- Are there changes to `Cargo.toml` files? (If yes, were new dependencies intentionally added?)

If you see a file you don't recognize or didn't expect, ask Claude: *"Why was [filename] changed?"*

A change should only touch files directly related to what was asked. Any file outside the stated
scope is a warning sign.

---

### Step 5 — Can you verify the change works?

For **bug fixes**: Try to reproduce the problem that existed before. Does it still happen? If the
fix worked, it should not.

For **new features**: Does the new behaviour work as expected? Claude should have told you what
to look for in the "how to verify" section of the PR.

If you cannot verify the change because you need hardware or a specific setup, ask Claude what
the automated test coverage is and whether it is sufficient.

---

### Step 6 — Final check before merging

- [ ] CI is green (all checkmarks on GitHub)
- [ ] Branch name follows convention (or is a known legacy exception)
- [ ] PR title and description match what was requested
- [ ] No unexpected files in the diff
- [ ] No unexpected new dependencies in any `Cargo.toml`
- [ ] The change has been verified to work (or has sufficient test coverage)

Only merge when all boxes are checked.

---

## When to Ask Claude for Help

At any point during this review, if something doesn't look right, ask:

- *"Why was [filename] changed?"*
- *"What does [term or file] do in plain English?"*
- *"CI failed — what went wrong?"*
- *"Is this change outside the scope of what I asked for?"*
- *"What tests cover this change?"*

You do not need to understand the code to ask good questions. If Claude's answer doesn't make
sense in plain English, ask for a clearer explanation.
