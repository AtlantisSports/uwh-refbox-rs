# Court-finished walkthrough — steps for Eric

Ten acceptance criteria. **Do them in this order** — 2, 3 and 4 first, because those three are the
defects found on 2026-08-17 and each must be *seen*, not reasoned about.

I launch the app; you click. Tell me the result after **each numbered step** and then wait — one at
a time, so a failure stops us before you spend effort on the rest.

Setup I will have running before you start:
- Local mock portal, court 1 = games **1** and **3** (3 is the last), court 2 = games 2, 4, 5.
- Halves are 30s and the break 45s, so a game plays out in about 70 seconds.
- Every POST the app makes is logged, so "nothing was sent" is something I can show you.

---

## The three Criticals

**1. Play the last game out** — Court 1, game 3. Score a goal or two, end the game, confirm the
score. WATCH FOR: the clock stops dead. Banner reads `END` over `--:--`. The middle block, the
settings values and the referee names all read dashes. Game 3's final score STAYS on the tiles.
START NOW is greyed and does nothing when pressed. *(criterion 1)*

**2. Close and reopen the app TWICE, quickly** — no other action in between.
WATCH FOR: the finished state both times. No countdown at all, no game ever starts.
*(criterion 2 — this was the morning's worst defect: it used to replay game 1 and post a fresh 0-0
over your real result.)*

**3. Reopen and press REFRESH several times.**
WATCH FOR: finished every time. It must not "wake up" and adopt a game. *(criterion 3)*

**4. I take the network down, you reopen the app.**
WATCH FOR: finished, nothing starts, however long you leave it. Then I bring the network back:
nothing must be sent. *(criterion 4 — the old code invented a game here, played it unattended and
delivered a fake 0-0 on reconnect. I have already verified this one from the file side; this is the
on-screen half.)*

---

## The rest

**5. I add a game to court 1 in the schedule; you press REFRESH.**
WATCH FOR: it is adopted, the countdown runs, START NOW works again, and game 3's score stays on
screen until the new game actually starts. *(criterion 5)*

**6. Mid-event restart.** With a game upcoming (not finished), close and reopen.
WATCH FOR: it comes back to the SAME game, ready to start. It must not skip ahead. *(criterion 6)*

**7. Pick a game out of order** in Settings — choose a later game than the one offered — then press
REFRESH. WATCH FOR: your pick survives; the refresh does not overrule you. *(criterion 7)*

**8. Half-time of the last game on the court.** Start game 3, let it reach half time.
WATCH FOR: the half-time break behaves completely normally — the 30-second whistle sounds, and
START NOW works to begin the second half. *(criterion 8. This is the one most worth your attention:
"finished" describes the schedule AFTER this game, and the last game on a court is very often a
final. NOTE: this machine has no working audio, so the whistle may be inaudible here — I can show
you the log line instead, and it is worth confirming properly on the Pi.)*

**9. From the finished state, switch the portal OFF** — do this on the **Game** page, then APPLY.
WATCH FOR: the break starts counting, START NOW is live, and numbering restarts at **1**.
*(criterion 9. Use the Game page: there is a second route — staging manual on the Game page and
pressing APPLY on the App page — which leaves numbering at last+1 instead. That is pre-existing,
not something this branch caused, and I have flagged it as a possible follow-up.)*

**10. Point the refbox at court 2**, which this box has no history for.
WATCH FOR: it offers NO game and waits for you to pick one. It must not helpfully offer the
earliest game — that game may have been played hours ago by another box. *(criterion 10)*

---

## Things I already know differ, so they are not surprises

- **A custom site now asks you to pick a game after every restart.** Safe (it asks rather than
  guesses) but portal users do not pay that cost, because custom-site notes have never been read
  back on startup. Whether to close that gap is a separate decision.
- **Reopening onto a finished court parks at 0:00**, the same state a court reaches when you play
  its last game — deliberately, so a restart looks identical to finishing normally.
- **If a game that has already been played gets moved to a LATER time in the portal**, the court can
  report "day is done" when it is not. I chose the safer failure direction; the spec does not settle
  it. Your call.
