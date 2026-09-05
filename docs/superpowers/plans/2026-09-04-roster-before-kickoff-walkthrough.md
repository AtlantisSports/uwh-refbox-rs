# Walkthrough: player rosters before kickoff

Branch `fix/refbox/roster-before-kickoff`. Follow the steps in order and note what you see
against each expected result. Where something differs, write down what actually happened rather
than what you expected — a surprise here is the point of the exercise.

Roughly **15 minutes**, most of it waiting for a game and a break to run.

---

## The trap in this change — read this first

**Before a game kicks off, the picker shows the plain 0-9 pad whatever FORCE KEYPAD NUMBERS is
set to.** So "I see the pad" tells you nothing on its own: a working fix and a completely missing
roster look *identical* on screen.

That is why every step below is done **twice** — once with FORCE KEYPAD NUMBERS set to NO, once
with it set to YES. Four states, not two. A pass at NO only means something when it sits next to
a pass at YES. If you only have time for half of this, you have proved nothing.

## Starting the app

I will launch it for you. It runs against the **test** portal with its own separate settings, so
nothing here can touch your real portal link or your normal settings.

The event is **Kings Cup (`1889-B`)**, **court 1**, first game up is **game 27 — Brisbane A vs
Melbourne Scubadorks**.

- **Brisbane A has 7 cap numbers:** 2, 3, 6, 7, 11, 35, 46
- **Melbourne Scubadorks has 12:** 3, 6, 9, 12, 15, 18, 21, 24, 33, 44, 55, 66

**BLACK is the dark team and WHITE is the light team**, so for game 27 BLACK shows Melbourne's 12
and WHITE shows Brisbane's 7. If you see 7 where you expected 12, you are looking at the other
team, not a bug.

**FORCE KEYPAD NUMBERS** is on the App options page in settings. You will switch it several
times, so find it before you start.

---

## Part 1 — Before the first kickoff (the requirement)

The app starts sitting in a countdown to game 27. **Do not start the game yet.**

### Step 1 — FORCE KEYPAD NUMBERS = **NO**

1. Set FORCE KEYPAD NUMBERS to **NO** and return to the main screen.
2. Confirm the clock area says **NEXT GAME** — you have not kicked off.
3. Open **ADD WARNING**, then choose **BLACK**.

> **Expected:** a grid of **Melbourne's 12 numbers** — 3, 6, 9, 12, 15, 18, 21, 24, 33, 44, 55, 66.
>
> **This is the fix.** Before this change you got the plain blue 0-9 keypad here.

4. Without leaving the page, switch to **WHITE**.

> **Expected:** a grid of **Brisbane's 7 numbers** — 2, 3, 6, 7, 11, 35, 46.

5. Cancel out.

### Step 2 — FORCE KEYPAD NUMBERS = **YES**

1. Set FORCE KEYPAD NUMBERS to **YES**. Still do not start the game.
2. Open **ADD WARNING** → **BLACK**, then **WHITE**.

> **Expected:** the plain blue **0-9 keypad** for both.
>
> **This is what makes Step 1 mean something.** It shows the grid in Step 1 was a real roster
> that the setting can switch off — not the app failing to find one.

3. Cancel out and set FORCE KEYPAD NUMBERS back to **NO**.

---

## Part 2 — During a game (nothing should have changed)

1. Start game 27.
2. Open **ADD WARNING** → **BLACK**, then **WHITE**.

> **Expected:** the same two grids as Step 1 — Melbourne's 12, Brisbane's 7. Identical to how the
> app behaved before this change.

3. Set FORCE KEYPAD NUMBERS to **YES** and look again.

> **Expected:** the 0-9 keypad. Also unchanged.

4. Set it back to **NO**.

### Step 3 — REFRESH must not move the grid — **REMOVED, do not walk this**

This step used to say: note the numbers, press REFRESH on the Game Info page, confirm they have
not changed.

**It cannot fail, so it proves nothing.** REFRESH re-pulls the event from the portal, but unless
somebody has actually edited a team's cap numbers on the portal *during the game*, it pulls back
the numbers already on screen. "Unchanged" is then the answer whether the grid is frozen or not.
Eric queried it on 2026-09-04 and was right to.

The property it was aiming at — during a game the grid is fixed at kickoff, so numbers cannot
shift under the operator's hand mid-entry — is covered by the unit test
`during_play_keeps_the_kickoff_copy`, which is where a check of that shape belongs.

**A real version of this check would need** a second person editing a cap number on the dev portal
while a game is running here, then confirming the grid does *not* adopt it until the next kickoff.
Worth doing once if the freeze is ever suspected; not worth it as routine.

---

## Part 3 — In the break, the upcoming game's players

Let game 27 finish, or wind the clock down to end it. The app moves into the break.

**The break has two halves and you need to check both.** The app names a new current game with
two different teams straight away, at the whistle. But about **2 minutes** later it does a
changeover, which you can see because the score on screen resets to **0-0**. The fix has to work
on both sides of that moment, so do Step 4 first, then Step 5.

### Step 4 — Immediately after the whistle, before the score resets

Do this within the first two minutes, while the score on screen is still **game 27's final
score**.

1. Open **ADD WARNING** → **BLACK**, then **WHITE**.

> **Expected:** the numbers of the **two teams named as the current game** — the game about to
> start, not Melbourne and Brisbane.
>
> **This is the bug you originally found**, in the state you found it in. If you see Melbourne's
> 12 or Brisbane's 7 here, the fix has not worked.

2. Cancel out.

### Step 5 — After the changeover

Wait until the score resets to **0-0** (or wind the break clock down with TIME EDIT to get there
faster).

1. Open **ADD WARNING** → **BLACK**, then **WHITE**.

> **Expected:** the same two teams as Step 4. Nothing should change across the changeover.

2. Set FORCE KEYPAD NUMBERS to **YES** and look again.

> **Expected:** the 0-9 keypad. (Same reasoning as Step 2.)

3. Set it back to **NO**.

---

## Part 4 — With the portal switched off

1. Switch the app to Manual (no portal event).
2. Open **ADD WARNING** → **BLACK** and **WHITE**.

> **Expected:** the plain 0-9 keypad for both, exactly as the app has always done. There is no
> portal to get numbers from.

---

## What to report back

For each step: **as expected**, or **what you actually saw**. The three that matter most:

- **Step 1** — the requirement you asked for.
- **Step 2** — without it, Step 1 proves nothing.
- **Step 4** — the bug you originally found, in the state where you found it.

## Two things this walkthrough cannot cover

**The other-court case.** If the last game on a court finishes with nothing scheduled after it,
the app invents a game number, which can name a real game on another court. The fix makes the
picker show the pad there rather than that game's players. Reaching it by hand needs a
multi-court event and a finished final game, so it is covered by an automated test instead.

**Anything I could check myself.** There is no screen-capture tool on this machine, so I cannot
drive the app and verify any of the above. It all rests on you seeing it. If a step is ambiguous
on screen, say so — that is a defect in this script, not in your reading of it.

## One thing you may notice, which is expected

If a player picker is open at the exact moment a game ends, the numbers will change from the
finished game's team to the upcoming game's. That is the fix doing its job. Anything you were
part-way through entering at that moment is discarded by the app regardless — a separate,
pre-existing bug, recorded under *Deferred* in the design document.

## Result — walked by Eric, 2026-09-04

- **Part 1** (before the first kickoff, FORCE KEYPAD both NO and YES) — **PASSED.** Screenshots
  confirmed BLACK offering Melbourne's twelve and WHITE offering Brisbane's seven with the clock
  reading NEXT GAME, and the plain pad at FORCE KEYPAD = YES. This is the requirement.
- **Part 2** (during a game) — **PASSED.**
- **Step 3** (REFRESH) — **not walked, and removed above as unfalsifiable.**
- **Part 3** (both halves of the break) — **PASSED.** The original reported bug, in the state it
  was reported in.
- **Part 4** (portal off) — **PASSED.**

Two notes for anyone repeating this. The dev schedule's start time for game 27 is long past, so
the app falls back to a **3-minute** break and kicks off on its own; wind the clock up with TIME
EDIT *before* starting, or the game begins mid-step (it did, on the first attempt). And reaching
the break needs the game to end naturally — TIME EDIT each period down rather than using
"end current game and apply", which takes a different path and skips the first half of the break.
