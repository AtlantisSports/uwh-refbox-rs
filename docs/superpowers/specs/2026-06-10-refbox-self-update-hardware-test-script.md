# Spare-Pi Hardware Test Script — Restart & Self-Update

**Date:** 2026-06-10
**Purpose:** The hands-on test an operator runs on a **spare Raspberry Pi** (one you can afford to
disrupt) to validate the restart fix and the self-update feature. This is the **hard gate**: the
feature is not "done" and must not be used at a real event until these steps pass.

**Why hardware:** the riskiest behaviour (relaunching the app, releasing and reclaiming the
scoreboard and radio, replacing the program file) cannot be proven on a developer laptop — it only
shows its true behaviour on a real Pi wired like a tournament Pi.

---

## Setup

1. Set up the spare Pi **exactly like a tournament Pi**: connect the LED scoreboard over its serial
   cable and the wireless referee button; if you use the stream overlay, connect that too.
2. **Record the exact command (or service) used to launch refbox** on this Pi — including the
   full-screen, serial-port, and real-hardware options. *(This is the one detail not written down
   anywhere; capture it now.)*
3. Launch refbox the same way the real Pi launches it.

## Baseline (prove the starting point works)

4. Confirm: the scoreboard shows the clock; the refbox window is **full-screen**; pressing the
   wireless button **sounds the buzzer**; the overlay connects if used.

## Test A — the restart fix (do this first, it's the prerequisite)

5. With **no update involved**, trigger the existing restart by **changing the language** (or the
   app mode) and confirming the restart prompt.
6. Observe after relaunch — **all must hold**:
   - the window returns to **full-screen**,
   - the **scoreboard reconnects** and keeps showing the clock,
   - the **wireless button still sounds the buzzer**,
   - the overlay reconnects if used,
   - logs continue going to the **same location** as before.
7. If any of those fail, the restart fix is not complete — stop and report which one.

## Test B — a successful update

8. On the Updates page, press **Check for Updates**; confirm it finds the newer version (status
   changes from *Unknown* to *Update available: X*).
9. Press **Install Update**, confirm the restart prompt.
10. Watch it download, verify, and restart. After relaunch, confirm **everything from step 6 again**
    (full-screen, scoreboard, buzzer, overlay, logs) **and** that the **version now reads the new
    one**.

## Test C — revert

11. On the Updates page, press **Revert to Previous Version**, confirm.
12. After relaunch, confirm the version is back to the **previous** one and everything from step 6
    still holds.

## Test D — failure handling (nothing should break)

13. **No internet:** disconnect the network, press Check for Updates → expect
    *"Couldn't reach the update server, please check your internet connection"*, and the app
    keeps running unchanged.
14. **Game-in-progress gate:** start a game (and separately, enter a half-time/break and a
    timeout) → confirm the **Check Version** button is disabled in each of those states.
15. **Interrupted download:** start an install, then press **Cancel** mid-download → confirm it
    returns to *Update available* and nothing changed.

## Test E — the worst case (recoverability)

16. Confirm the **manual SD-card method still works** as a fallback (sanity check that we haven't
    removed that escape route).
17. Optional, if you can safely simulate it: pull power **during** an install → on next boot the Pi
    should still start a working refbox (either the new one if the swap completed, or the previous
    one). It must **never** fail to start.

---

## Pass criteria

- Tests A, B, C: the app always comes back **full-screen, with the scoreboard and buzzer working**,
  on the expected version.
- Test D: every failure shows a plain message and **leaves the running app untouched**.
- Test E: the Pi **always boots into a working refbox**, no matter when interrupted.

Only when all of the above pass on the spare Pi should the feature be considered ready for use at
an event.
