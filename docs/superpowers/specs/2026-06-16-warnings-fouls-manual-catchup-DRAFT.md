# Refbox User Manual — "Warnings and Fouls" — Catch-Up Draft (v0.4.2)

**How to use this file:** each section below is paste-ready text for the styled Google Doc. Find
the matching heading in the Doc and paste/replace. Sections tagged **— additional callouts** are
*additions* to an existing screen's numbered list; the final numbers depend on your screenshot, so
renumber to match the callouts you place. Sections tagged **(no change)** are unchanged and listed
only so the running order is complete. Screen-name links use the Doc's existing cross-reference
style.

Entries run in screen/navigation order so they line up with the existing manual.

---

## Renames to confirm

- **"Tournament Options Screen" → "Game Options Screen"** — the on-screen button is now
  **GAME OPTIONS**.
- **New "User Options Screen" section** — Display Options and Sound Options now live under a new
  **User Options** menu, not directly on the Settings screen.
- **"UWH Portal Screen"** — keep the name, but it is now the Portal mode of Game Options (reached by
  turning on **Using UWH Portal**).

Veto any of these and I'll keep the old wording.

---

# 1. Pre-Game Screen — (no change)

Existing entry stands. Note: the Portal connection light described under the In-Game Screen also
appears on the time banner here whenever the refbox is connected to the Portal.

# 2. In-Game Screen — additional callouts

Add these to the existing [In-Game Screen](#in-game-screen) list. Each only appears in the
situation noted.

9) DELAY readout — a red **DELAY** label and a running time next to the period clock, showing how
   far behind schedule the games are. Only appears when **Show Behind Time/Delay** is turned on
   (App Options), the games are actually behind, and no timeout is in progress (it hides during a
   timeout and returns afterward).
10) Portal connection light — a small green, yellow, or red dot beside the time.
    1. Green means the Portal connection is healthy, yellow means delayed, red means there is a
       problem to resolve.
    2. Clicking it opens the [Portal Status Detail Screen](#portal-status-detail-screen).
    3. Only shown when connected to the UWH/UWR Portal with an event selected.
11) Manual alarm button — shows **ALARM** during play (tap to sound the buzzer) or **HOLD TO TEST**
    during breaks and timeouts (hold to test the buzzer). The line **Or Press Spacebar** /
    **Or Hold Spacebar** appears beneath it. Only shown when **Alarm Button** is turned on
    (Sound Options).

# 3. Time Edit Screen — (no change)

# 4. Time Edit With Timeout Screen — (no change)

# 5. Score Screen — (no change)

# 6. Score Edit Screen — (no change)

# 7. Score Confirmation Screen {#score-confirmation-screen}

Only appears when **Confirm Score at Game End** is turned on (App Options). It is also shown when a
goal is added during sudden death.

1) Shows the running game time.
2) Displays the score with the prompt **CONFIRM SCORE — IS THIS SCORE CORRECT?**
3) Adjusts the score up or down for either team if a correction is needed.
4) **YES** confirms the score is correct and continues.
5) **NO** rejects the score and returns so it can be corrected.

# 8. Add Warning Screen — (no change)

# 9. Add Foul Screen — (no change)

# 10. Warning and Foul Summary Screen — (no change)

# 11. Edit Warnings and Fouls Screens — (no change)

# 12. Penalty Summary Screen — (no change)

# 13. Penalty Screen — (no change)

# 14. Timeout Screens — (no change)

# 15. Game Info Screen — additional callouts

Add these to the existing [Game Info Screen](#game-info-screen) list.

4) Referee names — when connected to the Portal, the assigned referees are listed: Chief Ref,
   Timer, and Water Ref 1, 2, and 3. Unassigned roles show "-". These are read-only and come from
   the Portal; they cannot be edited in the refbox.
5) Game Block — when **not** using the Portal, the details include **Game Block: \<time\>**, the
   time from the start of one game to the start of the next.
6) **REFRESH** — when connected to the Portal, reloads the schedule and game details from the
   Portal; shows **REFRESHING…** while it loads.

# 16. Settings Screen {#settings-screen}

The Settings menu has been reorganized. The game number is now set inside Game Options, and Display
and Sound options are now grouped under User Options.

1) Opens the [Game Options Screen](#game-options-screen), where game timing and the game number are
   set.
2) Opens the [App Options Screen](#app-options-screen).
3) Opens the [User Options Screen](#user-options-screen), which holds Display Options and Sound
   Options.
4) Opens the [Language Screen](#language-screen).
5) Returns to the previous screen.

# 17. Game Options Screen {#game-options-screen}

(Formerly "Tournament Options.") Sets the game timing and the game number when **not** using the
Portal.

1) **USING UWH PORTAL** — switch on to use the Portal and go to the
   [UWH Portal Screen](#uwh-portal-screen).
2) The game timing options are set here: half/game length, half-time, overtime and sudden-death
   breaks, minimum break between games, and team timeouts. Tapping a time opens the
   [Parameter Editor Screen](#parameter-editor-screen).
   1. The Half Length option includes a **2 Halves / 1 Period** choice — see the Parameter Editor.
3) **GAME BLOCK** — the time from the start of one game to the start of the next. Tapping it opens
   the Parameter Editor; it warns if the value is too short, or tight, for the schedule.
4) Sets the game number (opens a keypad).
5) **CANCEL** discards changes and returns.
6) **APPLY** saves changes and returns.

# 18. UWH Portal Screen {#uwh-portal-screen}

The Portal mode of Game Options. Shown when **Using UWH Portal** is on.

1) **USING UWH PORTAL** — switch off to set the game up manually and return to the
   [Game Options Screen](#game-options-screen).
2) **EVENT** — selects the tournament/event.
3) **COURT** — selects the court (after an event is chosen).
4) Selects the game from the event's schedule.
5) **UWHPORTAL TOKEN** — shows the connection status: **OK**, **FAILED**, or **CHECKING…**.
   1. Tapping it (when an event is selected) opens the [Portal Login Screen](#portal-login-screen)
      to enter a login code.
6) **APPLY** saves changes and returns. Apply is unavailable until an event, a court, and a valid
   game are all selected.

# 19. Portal Login Screen {#portal-login-screen}

Reached from the **UWHPORTAL TOKEN** status on the [UWH Portal Screen](#uwh-portal-screen).

1) Shows this refbox's ID (read-only) — used to authorize it on the Portal website.
2) Enter the login code on the keypad.
3) **CANCEL** discards and returns.
4) **DONE** submits the code.

# 20. Parameter Editor Screen {#parameter-editor-screen}

Opens when a time field is tapped on the Game Options Screen.

1) Shows the game time banner.
2) For Half Length only: a **2 HALVES** / **1 PERIOD** selector. **2 HALVES** plays two halves with
   a half-time break; **1 PERIOD** plays one continuous period with no half-time (the field is then
   labelled "Game Length"). The highlighted button is the current choice.
3) The name of the option being edited.
4) **?** opens the [Parameter Help Screen](#parameter-help-screen), which explains the option.
5) Sets the time using the keypad and the +/- buttons.
6) **CANCEL** discards changes and returns.
7) **DONE** applies and returns. When editing **Game Block**, a value that is too short shows in
   red and **DONE** stays disabled until it is long enough; a "tight" value shows in yellow.

# 21. Parameter Help Screen {#parameter-help-screen}

Opens from the **?** button on the Parameter Editor Screen.

1) Shows the game time banner.
2) Explains what the option does.
3) Returns to the [Parameter Editor Screen](#parameter-editor-screen).

# 22. App Options Screen {#app-options-screen}

1) **APP MODE** — selects the mode: **HOCKEY6V6**, **HOCKEY3V3**, **RUGBY**, or **BEEP TEST**.
2) **TRACK CAP NUMBER OF SCORER** — sets whether a cap number is asked for when adding a score.
3) **TRACK FOULS AND WARNINGS** — turns the foul and warning tracking features on or off.
4) **CONFIRM SCORE AT GAME END** — requires the final score to be confirmed at the end of a game
   (see the [Score Confirmation Screen](#score-confirmation-screen)).
5) **SHOW BEHIND TIME/DELAY** — shows a red DELAY readout on the In-Game screen when the games run
   behind schedule.
6) **CANCEL** discards changes and returns.
7) **Check Version** — opens the [Updates Screen](#updates-screen). Disabled while a game is in
   progress.
8) **APPLY** saves changes and returns.

# 23. Updates Screen {#updates-screen}

Opens from **Check Version** on the App Options Screen. Used to update the refbox software.

1) Shows the game time banner.
2) The current version, and a **Check for Updates** button.
3) A status line: **Checking…**, **Up to date.**, **Update available: \<version\>**,
   **Downloading…**, **Checking the download…**, **Installing…**, **Restarting…**, or an error
   message.
4) A note line — for example, "Clicking install will download, install, and restart the refbox."
5) **Revert to Previous Version (\<version\>)** — restores the previous version. Shown only when a
   backup of the previous version exists.
6) **BACK** (or **CANCEL** during a check or a revert) returns. It is disabled while installing or
   restarting.
7) The action button: **Install** when an update is available, or **Revert** when confirming a
   revert.

If an update is installed but the refbox does not restart correctly, it automatically returns to
the previous version and reopens this screen the next time it starts, with the message: "Reverted
to the previous version because the update didn't start correctly, please try again."

# 24. User Options Screen {#user-options-screen}

Reached from **User Options** on the Settings Screen.

1) Opens the [Display Options Screen](#display-options-screen).
2) Opens the [Sound Options Screen](#sound-options-screen).
3) **VIEW MODE** — switches the look between **LIGHT**, **DARK**, and **HIGH CONTRAST** (applies
   immediately).
4) Returns to the previous screen.

# 25. Display Options Screen {#display-options-screen}

1) **STARTING SIDES** — swaps which side each team starts on.
2) **HIDE TIME FOR LAST 15 SECONDS** — hides the time on the scoreboard for the last 15 seconds
   before a half or other play period starts.
3) **DISPLAY LAYOUT** — chooses the front-display layout: **DEFAULT**, **CLASSIC**, **BIG TIME**,
   **CORNERS**, or **SCORES ONLY**. Disabled when a physical LED panel is connected (the panel
   always uses the default layout).
4) **OPEN NEW DISPLAY** — opens a preview window of the front display. Disabled when a physical LED
   panel is connected.
5) **PLAYER DISPLAY BRIGHTNESS** — sets the LED panel brightness: **LOW**, **MEDIUM**, **HIGH**, or
   **OUTDOOR**. Only active when an LED panel is connected.
6) A live preview image of the chosen layout and starting sides.
7) **CANCEL** discards changes and returns.
8) **APPLY** saves changes and returns.

# 26. Sound Options Screen — additional callouts

Add these to the existing [Sound Options Screen](#sound-options-screen) list:

- **ALARM BUTTON** — turns on the manual alarm button that then appears on the In-Game screen
  (shown as **ALARM** / **HOLD TO TEST**).
- The remotes control is now a **MANAGE REMOTES** button that opens the
  [Manage Remotes Screen](#manage-remotes-screen).

# 27. Manage Remotes Screen {#manage-remotes-screen}

Reached from **Manage Remotes** on the Sound Options Screen. Up to four remotes are shown at a time.

1) Each paired remote's ID (read-only).
2) **SOUND** — sets which buzzer sound that remote plays.
3) **DELETE** — removes that remote.
4) **ADD** starts listening for a new remote; it shows **WAITING** while pairing.
5) **CANCEL** discards changes and returns.
6) **APPLY** saves changes and returns.

# 28. Language Screen {#language-screen}

Reached from **Language** on the Settings Screen.

1) Tap a language to select it (15 languages; the current one is highlighted). Languages still
   being checked show an "(unverified)" note in their own script.
2) **CANCEL** discards and returns.
3) **DONE** applies the language — or **RESTART TO APPLY** when the change needs a different font
   (the refbox restarts).

# 29. Portal Status Detail Screen {#portal-status-detail-screen}

Opens by tapping the Portal connection light on the time banner.

1) The Portal connection light (green / yellow / red).
2) A list of game results sent to the Portal, colour-coded: red = stuck or login expired,
   yellow = still sending, green = recently sent successfully.
3) Scrolls the list.
4) Tapping a red (stuck) row opens the
   [Portal Action Required Screen](#portal-action-required-screen).
5) **BACK** returns.

# 30. Portal Action Required Screen {#portal-action-required-screen}

Opens from a stuck (red) row on the Portal Status Detail Screen.

1) The game number, its score, and a description of the problem.
2) **FORCE THIS GAME RESULT** — sends the result again, overriding the conflict.
3) **DISCARD THIS SUBMISSION** — removes the stuck result (tap twice to confirm).
4) If the login has expired, a button to go to the login.
5) **BACK** returns to the Portal Status Detail Screen.

# 31. Confirmation Dialogs {#confirmation-dialogs}

The refbox shows a confirmation when a change needs a decision. The buttons depend on the situation:

- Changing game settings during a game: **GO BACK TO EDITOR**, **DISCARD CHANGES**, or
  **END CURRENT GAME AND APPLY CHANGES**.
- Changing the game number during a game: as above, plus **KEEP CURRENT GAME AND APPLY CHANGE**.
- Switching between Hockey and Rugby (which restarts the app): **CANCEL** or **RESTART TO APPLY**.
- Incomplete Portal setup, an invalid login code, or another error: a message with **OK**.

# 32. BeepTest Main Screen {#beeptest-main-screen}

Shown when **App Mode** is set to **BEEP TEST**. The Beep Test is a fitness test, not a game.

1) The info row shows **TIME**, **LEVEL**, and **LAP**.
2) The levels table; the current level/lap cell is highlighted while running.
3) **RESET** — clears the run (available after the test has been started once).
4) **SETTINGS** — opens the [BeepTest Settings Screen](#beeptest-settings-screen).
5) **START** begins the test; it then becomes **PAUSE**, and **RESUME** after a pause.

# 33. BeepTest Settings Screen {#beeptest-settings-screen}

Reached from **SETTINGS** on the BeepTest Main Screen.

1) **SOUND SETTINGS** — opens the
   [BeepTest Sound Settings Screen](#beeptest-sound-settings-screen).
2) **EDIT LEVELS** — opens the [BeepTest Edit Levels Screen](#beeptest-edit-levels-screen)
   (disabled while a test is running).
3) **APP MODE** — switches back to a game mode (disabled while running).
4) **LANGUAGE** — opens the [BeepTest Language Screen](#beeptest-language-screen) (disabled while
   running).
5) **BACK** returns to the BeepTest Main Screen.
6) **RESTART TO APPLY** — appears when the mode was changed; restarts the app into the new mode.

# 34. BeepTest Sound Settings Screen {#beeptest-sound-settings-screen}

The sound controls are the same as the main [Sound Options Screen](#sound-options-screen) (sound
on/off, whistle, buzzer sound, and volumes). **CANCEL** discards changes and **SAVE** keeps them.

# 35. BeepTest Edit Levels Screen {#beeptest-edit-levels-screen}

Reached from **EDIT LEVELS** (only when a test is not running).

1) The list of levels, each with its lap count and its time per lap.
2) The +/- buttons adjust the count and the time for the selected level.
3) **NEW** adds a level.
4) **DELETE** removes a level.
5) **CANCEL** discards changes and returns.
6) **SAVE** keeps the changes and returns.

# 36. BeepTest Language Screen {#beeptest-language-screen}

The same as the main [Language Screen](#language-screen): tap a language, then **CANCEL** or
**DONE** (or **RESTART TO APPLY** when a different font is needed).
