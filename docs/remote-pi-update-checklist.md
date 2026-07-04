# Updating the Refbox on a Raspberry Pi — Over the Network

*Plain-English checklist. Goal: replace the refbox program on a Pi with a newer
version **without opening the case or removing the SD card**, by connecting to
the Pi over the network from a laptop.*

> **Status:** Draft procedure. The parts marked `‹fill in›` are details that
> depend on how your specific Pi was set up. They are not knowable from the
> software — they must be confirmed the first time someone connects to the Pi.
> **Do not rely on this at a live tournament until it has been done successfully
> at least once in a calm setting.**

---

## Before you start — what you need in hand

- [ ] **The new refbox program file.** A single file named `refbox`, built for
      the Raspberry Pi (the "aarch64" / Pi 4–5 version). This comes from the
      project's Releases page, or is built with `just build-rpi`.
- [ ] **A laptop on the same network as the Pi.** Wi-Fi or Ethernet — it just
      has to be the same network the Pi is on.
- [ ] **The pass keys** (the SSH credentials) that let the laptop log into the
      Pi. Location: `‹fill in — where these keys are stored›`
- [ ] **The Pi's address on the network.** Either a name (like `refbox.local`)
      or a number (like `192.168.1.50`). `‹fill in›`
- [ ] **The login username on the Pi.** Often `pi`, but may be different.
      `‹fill in›`
- [ ] **About 20 quiet minutes** — not during a game.

---

## The update, step by step

### 1. Connect to the Pi from the laptop
Using the pass keys, open a remote connection to the Pi. On the laptop this is
typically a single command:

```
ssh ‹username›@‹pi-address›
```

If it asks to "trust" the device the first time, say yes. If it connects and
shows you a prompt that mentions the Pi, you're in. **If this step fails, stop
here** — the rest can't proceed, and it means the remote-access setup needs to
be sorted out first (a one-time job).

### 2. Find where the current refbox file lives
Note the location of the refbox program currently on the Pi, so the new one goes
in the same place. `‹fill in — the folder path, e.g. /home/pi/refbox›`

### 3. Stop the refbox if it's running
The program file can't be replaced while it's in use. How it's stopped depends
on how it was started:
- If it auto-starts on boot (a "service"): stop that service. `‹fill in — the
  service name / stop command›`
- If someone starts it by hand: just close it.

### 4. Keep a copy of the old version (safety net)
Before overwriting, rename the existing file so you can put it back instantly if
the new one misbehaves. For example, rename `refbox` to `refbox-old`.

### 5. Copy the new file onto the Pi
From the laptop (in a separate window, not the remote connection), send the new
file across using the pass keys:

```
scp ‹path-to-new-refbox-on-laptop› ‹username›@‹pi-address›:‹folder-on-pi›
```

### 6. Make sure the new file is allowed to run
Newly copied files sometimes need to be marked as runnable:

```
chmod +x ‹folder-on-pi›/refbox
```

### 7. Start the refbox again
Start it the same way it normally runs (the service, or by hand — see step 3).

### 8. Confirm the new version is running
Open the refbox on the Pi's screen and check that it behaves as expected / shows
the new version. Watch it through a normal start-up to be sure nothing broke.

### 9. If anything looks wrong — roll back
Stop the refbox, delete the new file, rename `refbox-old` back to `refbox`, and
start it again. You're back to exactly where you were. This is why step 4
matters.

---

## Safety rules

- **Never do this for the first time during a tournament.** Prove it once in a
  calm setting first.
- **Always keep the old version** (step 4) until the new one is confirmed good.
- **One Pi at a time.** If you have several, update and confirm one fully before
  touching the next.
- **Have the SD-card method as a fallback.** If remote updating ever fails, the
  old manual method (swap the file via the SD card) still works.

---

## Filling in the blanks (one-time)

The `‹fill in›` items above are the only things standing between this checklist
and a repeatable routine. Capture them the first time someone connects:

1. Pi network address — `‹fill in›`
2. Login username — `‹fill in›`
3. Where the pass keys are kept — `‹fill in›`
4. Folder on the Pi where `refbox` lives — `‹fill in›`
5. How the refbox starts (auto-service vs. by hand) and how to stop/start it —
   `‹fill in›`

Once these five are known and written in, this becomes a fixed routine that a
non-technical helper can follow.
