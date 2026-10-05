# Streaming setup: overlay, overlay-bridge and Stream Manager

How to build and install everything the stream needs at a tournament: the score **overlay**,
**overlay-bridge** and **Stream Manager**. Stream Manager splits each court's stream into one
YouTube video per game; see `docs/decisions/026-per-game-youtube-streams.md`.

Written so that a person **or Claude running on the tournament PC** can follow it step by step.

The overlay sends its picture to vMix over **NDI®**. NDI® is a registered trademark of Vizrt NDI AB.
Learn more about NDI and get NDI Tools at https://ndi.video.

---

## Read this first (for Claude, and for anyone following this guide)

- **Do the steps in order.** Each one ends with a **Check** line. Don't go on until the check passes.
- **⛔ STOP — ask a human** marks steps that need a person. Examples: typing a password or PIN,
  signing in to Google, choosing the YouTube channel, providing a secret file, or approving
  anything that changes the live YouTube channel. Never type passwords or secrets yourself, and
  never print the contents of `client_secret.json` or `youtube-token.json`.
- **Never turn Practice mode off, press Start day, or create videos on YouTube** unless a human
  asks for it right then.
- Commands below are for **PowerShell** on Windows. Paths assume the user is `<you>`.
- If a check fails, look in **Troubleshooting** at the end before changing anything.

---

## 1. What runs where

```
 Court 1 refbox ──(port 8000)──┐
                               ▼
 ┌───────────────────── Court 1 mini PC ─────────────────────┐
 │ overlay-bridge ──► overlay ──NDI──► vMix ──► YouTube       │
 │   (status page :8098)                ▲                     │
 │ Stream Manager ──(vMix control :8088)┘  ──► YouTube API    │
 │   (control page :8090)  also reads the refbox (port 8000)  │
 └────────────────────────────────────────────────────────────┘
 Court 2: the same again on Court 2's mini PC.
```

- **Each court has its own mini PC.** It runs vMix, the overlay, overlay-bridge and Stream Manager.
- **Each Stream Manager handles only its own court.** To see the other court, open its control page
  in another browser tab: `http://<other mini PC's IP>:8090`.
- The refbox is a separate device at the court. The overlay-bridge and Stream Manager both connect to it.

**Where the programs come from:**

| Program | Code | Ready-made download |
|---|---|---|
| overlay, overlay-bridge | `master` (the NDI picture fix and offline team names from PR #3383 are merged) | `streaming-tools-windows.zip` (A0) |
| Stream Manager | `feat/workspace/stream-manager` until its PR is merged, then `master` | the same zip |

---

## Part A — Get the programs

There are two ways. **A0 (download) is the normal way.** A1–A4 (build from source) are the
fallback, e.g. when you need a change that isn't in a download yet.

### A0. Download the ready-made programs (no build tools needed)

GitHub builds all three Windows programs together as **`streaming-tools-windows.zip`**. It contains
`overlay.exe` (with NDI and the bridge feed built in), `overlay-bridge.exe`, `stream-manager.exe`,
this guide and a `README.txt`.

- **From a release** (once a release includes it): GitHub → the repository → **Releases** → the
  latest release → **`streaming-tools-windows.zip`**.
- **From the latest build**, e.g. before the next release: GitHub → **Actions** → **Streaming
  tools** → the newest run with a green tick, on `master` or on the branch you want → at the bottom
  under **Artifacts**, **`streaming-tools-windows`**. You must be **logged in to GitHub**. These
  downloads are kept for 90 days.
  - ⛔ **STOP — ask a human** to log in to GitHub, if you are Claude.

Unzip it, then go to **Part B**. Skip A1–A4.

**Check:** the folder contains `overlay.exe`, `overlay-bridge.exe` and `stream-manager.exe`.

### A1. Install the build tools (once per build computer, fallback only)

The build computer can be one of the mini PCs or any Windows PC. You build once, then copy the
three `.exe` files to each mini PC. Expect **1–2 hours and several GB of downloads** for the tools.
Install them **before travelling**, not at the venue.

1. **Git**: https://git-scm.com/download/win
   - **Check:** `git --version` prints a version.
2. **Visual Studio Build Tools** with the **"Desktop development with C++"** workload:
   https://visualstudio.microsoft.com/visual-cpp-build-tools/ (needed by Rust on Windows).
3. **Rust**: https://rustup.rs. The repo pins its Rust version (`rust-toolchain.toml`), and the
   right one installs automatically on the first build.
   - **Check:** `cargo --version` prints a version.
4. **For the overlay only:**
   - the **NDI SDK** (https://ndi.video/for-developers/ndi-sdk/download/), default location
     `C:\Program Files\NDI\NDI 6 SDK`. Installing it means accepting the NDI SDK licence.
   - **LLVM/Clang** (https://github.com/llvm/llvm-project/releases, Windows installer), with
     **"Add LLVM to the system PATH"** ticked
   - **Check:** `clang --version` prints a version. If it doesn't, run this in the same window
     before building: `$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"`

⛔ **STOP — ask a human** before installing software, if you are Claude.

### A2. Get the code (one folder)

All three programs are in the same repository. Until the Stream Manager PR is merged, use its
branch (it already contains everything in `master`); afterwards, use `master`.

```powershell
cd $HOME\Downloads
git clone -b feat/workspace/stream-manager https://github.com/AtlantisSports/uwh-refbox-rs.git uwh-streaming
```

If the folder already exists, update it instead:

```powershell
cd $HOME\Downloads\uwh-streaming; git pull
```

**Check:** `git -C $HOME\Downloads\uwh-streaming branch --show-current` prints the branch you chose.

### A3. Build

**Important:** `cargo` builds whichever folder the terminal is in. Always `cd` into the right
folder first.

```powershell
cd $HOME\Downloads\uwh-streaming
cargo build --release -p overlay --features ndi,bridge
cargo build --release -p overlay-bridge
cargo build --release -p stream-manager
```

- Build the overlay with **both** `ndi,bridge`. With only `ndi` it builds fine, but it silently
  skips the bridge and shows the wrong team names.
- Close any running copy of a program before rebuilding it. Otherwise you get "Access is denied".

**Check:** these three files exist and were just written (look at the time):

```
uwh-streaming\target\release\overlay.exe
uwh-streaming\target\release\overlay-bridge.exe
uwh-streaming\target\release\stream-manager.exe
```

### A4. Collect the files

Copy the three `.exe` files into one folder, e.g. a USB stick folder `Streaming\`. Each program is a
single file; the overlay's images and fonts are built into it.

---

## Part B — Set up each mini PC (repeat for Court 1 and Court 2)

### B1. Software and files

1. **vMix 29 or newer.** Older versions (e.g. 26) lack the **FFMPEG6** option and can't use the
   NVIDIA encoder with current drivers.
   - **Check:** in vMix, Help → About shows 29 or newer.
2. **The NDI engine: nothing to do here.** The overlay needs NDI's engine (the "NDI runtime") to
   send its picture. If it isn't installed, the overlay installs it itself the first time it
   starts (B3). Installing **NDI Tools** (https://ndi.video/tools/) is optional; it includes the
   same engine plus NDI's own monitoring apps.
3. Create `C:\Streaming\` and copy `overlay.exe`, `overlay-bridge.exe` and `stream-manager.exe`
   (from A0 or A4) into it.
4. Make a desktop shortcut for each (right-click the `.exe` → Send to → Desktop).

**Check:** the three shortcuts are on the desktop.

### B2. overlay-bridge

1. Double-click **overlay-bridge**. A console window opens and the **status page** opens in the
   browser at `http://127.0.0.1:8098`. (Port 8099 is taken by vMix, so the bridge uses 8098.)
2. On the status page, set the **refbox** to this court's refbox (its IP address, port `8000`).
3. Choose where team names come from: **UWH Portal** (normal), or **Local** with the two CSV files
   (for venues without portal access).

**Check:** the status page shows the refbox as connected, and the current game's team names.

Its settings are saved in `%APPDATA%\overlay-bridge\config\default-config.toml`, and uploaded CSV
files in `%APPDATA%\overlay-bridge\config\csv-files\`.

### B3. overlay

1. Double-click **overlay**.
   - **First time on this PC, without the NDI engine:** the overlay downloads NDI's official engine
     installer (https://ndi.link/NDIRedistV6) and only uses it if Windows confirms it's signed by
     Vizrt (NDI's owner). Progress shows in yellow at the top-left of the overlay's window
     ("NDI: …").
   - ⛔ **STOP — a human:** Windows asks "Allow this app to make changes?" → **Yes**. NDI's
     installer opens → read and **accept NDI's licence** → **Install**. When it finishes, the
     overlay starts its NDI output by itself; no restart needed.
   - If the engine is already installed, there's nothing to see: it's found automatically.
   - Then close the overlay. This first start also created its settings file at
     `%APPDATA%\overlay\config\default-config.toml`.
2. Open that file in Notepad and check `bridge_url = "http://127.0.0.1:8098"`. That's the default
   when the bridge runs on the same PC. Save if you changed anything.
3. Start **overlay** again and leave it running. Its window shows a small preview; the real picture
   goes out over NDI.
4. **Windows Firewall:** if Windows asks, allow it on **Private** networks. If the NDI source
   doesn't appear in vMix, open Windows Defender Firewall → "Allow an app through the firewall",
   and allow `overlay.exe`. Also look for an automatic **Block** rule for it under Advanced settings
   → Inbound Rules.
5. In vMix: **Add Input → NDI**, pick the source named like `<PC NAME> (UWH Overlay)`, and put it
   on an overlay channel.

**Check:** in vMix the overlay appears **with transparency** (the camera picture shows around the
graphics), and the team names match the bridge's status page.

### B4. vMix streaming

⛔ **STOP — ask a human** for the YouTube stream keys. They are secrets.

In YouTube Studio → Go live → Stream there are two reusable stream keys per court, named exactly
**`Court N - A`** and **`Court N - B`**, with resolution set to auto-detect / variable.

In vMix → Streaming settings, set up **Destination 1** and **Destination 2**:

| Setting | Value |
|---|---|
| Destination | **Custom RTMP Server**. Don't use vMix's built-in YouTube sign-in. |
| URL | `rtmp://a.rtmp.youtube.com/live2` |
| Stream key | Destination 1 = `Court N - A`, Destination 2 = `Court N - B` |
| Application | **FFMPEG6** |
| Use Hardware Encoder | **ticked** |
| Quality (gear icon) | 1920×1080, **12000** kbps, audio 128, Profile **High** (or Main), Level **4.2** (or Auto), Preset **P5** (not a Low Latency preset), keyframes **2 s**, **Strict CBR on, NAL CBR on** |

Also set:
- **Frame rate** (Settings → General): 50 Hz countries use 50 or 25; 60 Hz countries use 59.94
  or 29.97. Lower frame rates give sharper graphics on YouTube at the same bitrate (being tested
  2026-10).
- **Web Controller:** Settings → Web Controller → **enabled**, port **8088**. Stream Manager uses
  it to start and stop the destinations.
- **Recording:** enabled to the local disk, as a backup.

**Don't press Stream yourself.** Stream Manager starts and stops the destinations.

**Check:** start Destination 1 by hand for a moment. Task Manager → Performance → GPU →
**Video Encode** moves, and vMix's stream status shows a `bitrate=` close to 12000. Then stop it.

### B5. Stream Manager

1. ⛔ **STOP — ask a human** for the Google sign-in file. Create the folder
   `C:\Users\<you>\Documents\stream-manager\` and have the human put **`client_secret.json`**
   there. It comes from Google Cloud project "Stream-Manager" → Credentials → the Desktop OAuth
   client → download. It's a secret: don't open it, print it or copy it anywhere else.
2. Double-click **stream-manager**. A console window opens and the control page opens at
   `http://127.0.0.1:8090`.
3. ⛔ **STOP — ask a human** to choose the **PIN**. Use the same PIN on both mini PCs.
4. **Settings tab:**
   - **Portal:** Live portal (Dev portal only for tests). **Event:** pick it from the list.
   - **Privacy:** Unlisted for tests. ⛔ Public only when a human says so.
   - **Courts:** **only this mini PC's court**. Remove any other. Court name exactly as the portal
     uses it (e.g. `1`), refbox IP and port `8000`, **vMix address `127.0.0.1:8088`**. Leave the
     stream key names empty to use `Court N - A` / `Court N - B`.
   - **Practice mode:** leave **ON** for now.
   - Click **Save settings**. **Check:** "Saved ✓".
5. ⛔ **STOP — a human must do this.** Click **Connect YouTube**. In the browser, sign in with the
   Google account that manages the channel, choose the **Atlantis Sports** channel, and click
   through "Google hasn't verified this app" (Advanced → Go to Stream-Manager) → Allow.
   - **Check:** the header shows the channel name with a green dot.
6. Click **Check stream keys**. **Check:** both of this court's keys are found.
7. ⛔ **STOP — a human must do this.** Under **Portal sign-in (for "watch" links)**, enter the email and
   password of a portal account with **admin** rights → **Save and check**. Stream Manager checks them
   with the portal and stores the password encrypted, so only this Windows user on this PC can read it.
   From then on, every Prepare puts each game's YouTube link into the "watch" space of the portal
   schedule, and deleting test videos clears them again.
   - **Check:** "Signed in as … (admin)" in green. Without this, everything else still works, but the
     portal gets no watch links.
8. **Live tab.** **Check:** "Refbox connected" and "vMix connected" are both green.
9. **Windows Firewall:** if Windows asks, allow stream-manager on **Private** networks. That lets the
   other mini PC and the Stream Deck reach the control page.

Settings and secrets live in `Documents\stream-manager\`:

| File | What it is |
|---|---|
| `config.toml` | settings, including the PIN |
| `client_secret.json` | the Google sign-in file (secret) |
| `youtube-token.json` | the saved YouTube connection (secret) |
| `state-<event>.json` | the videos and playlists it created |

**Never put this folder in the code repository.**

### B6. Stream Deck (Companion)

In Companion, add a **Generic HTTP** connection. Each button sends a **GET** to the address shown
at the bottom of the Live tab ("Companion / Stream Deck links"), for example:

```
http://<mini PC IP>:8090/api/court/1/hold-toggle?pin=<PIN>
```

The actions are `start`, `hold-toggle`, `next` (Switch now) and `end`.

---

## Part C — Before the day and on the day

### C1. Prepare the videos (the day before)

⛔ **STOP — a human confirms.** This creates videos on the YouTube channel.

Prepare tab → choose the **day** and this mini PC's **court** → **1. Preview**. Check the number of
videos, the privacy and the cost (YouTube's daily allowance is 10,000 units) → **2. Create on
YouTube**.

**Check:** the "Videos for this event" list shows every game in schedule order, each linked to
stream key A or B, alternating. The Prepare log ends with "Portal: watch links set for N game(s)",
and the portal's schedule page shows a watch link for each game. If the portal couldn't be reached,
use **Send links to portal again** later.

### C2. Dry run (strongly recommended)

1. With Practice mode **ON**, set the refbox to the event, court and first game. Press **Start
   day**. The log shows "(practice) GO LIVE…". Let a short game finish. The log shows
   "(practice) SWITCH…" at 3:15 before the next game. Press **End day**.
2. ⛔ With a human's OK, do the same with Practice mode **OFF** (Settings, only possible while no
   day is running), using unlisted test videos:
   - vMix starts the destination.
   - The video goes live.
   - The switch at 3:15 starts the other destination, posts a chat message and ends the old video.
3. Afterwards: **Test tools → Delete** removes the test videos.

### C3. On the tournament day

- Set the refbox to the event, court and first game. Before the first game it may show "Game 0".
  That's normal.
- Press **Start day** when you want the stream to begin (e.g. 10 minutes before the first game).
- After that, each game's video switches automatically **3:15 before the next game**, just before
  the overlay shows the rosters, so interviews stay in the previous game's video.
- **Hold** stops the automatic switch (e.g. a long interview). **Switch now** switches
  immediately.
- If a switch fails, the court shows a **red message** and goes to Hold. The old video stays live.
  Fix the cause, then press **Switch now**.
- At the end, press **End day**. It ends the last video and stops both vMix destinations.

---

## Troubleshooting

| Problem | Cause and fix |
|---|---|
| "Couldn't use port 8090 … (is Stream Manager already running?)" | Another copy is running, maybe minimised. Task Manager → Details → `stream-manager.exe` → End task (press **End day** first if a day is running). |
| "Access is denied" when building | That program is running. Close it, then build again. |
| Rebuilt, but nothing changed | The build ran in the wrong folder. `cd` into the right folder first (A3). |
| NDI source missing in vMix | Look at the top-left of the overlay's window: a yellow "NDI…" line says why NDI isn't running. If the engine install failed or was cancelled, install it from https://ndi.link/NDIRedistV6 and restart the overlay. Also allow `overlay.exe` through the firewall (B3). |
| Wrong team names on the overlay | The overlay was built without `bridge`. Rebuild with `--features ndi,bridge`. |
| vMix: "Cannot get the preset configuration: unsupported param (12)" | That destination uses the old **FFMPEG**. Set Application to **FFMPEG6** (vMix 29+). |
| YouTube: "bitrate … lower than recommended" / new graphics look blocky for a few seconds | Turn on **Strict CBR + NAL CBR**, use Preset **P5**, and check vMix's `bitrate=` shows about 12000. |
| Red: "The refbox is on Game X, which isn't in …'s schedule" | The refbox isn't on this event or court. Select the right event and court on the refbox. |
| Red: "Game X has no YouTube video yet" | Run **Prepare** for that day and court. |
| overlay-bridge window opens and closes at once / "could not start the bridge's HTTP server on 0.0.0.0:8099" | An old version uses port 8099, which vMix always occupies. Use the current download (port 8098). Or start it once from PowerShell with `.\overlay-bridge.exe --port 8098`; it remembers the port. |
| Overlay shows no team names or scores | The overlay can't reach the bridge. Check `bridge_url` in `%APPDATA%\overlay\config\default-config.toml` matches the bridge's port (`http://127.0.0.1:8098`), and that the bridge's status page shows the refbox connected. Restart the overlay after changing the file. |
| Prepare log: "⚠ Portal watch links not updated: …" | The message says why: wrong portal password (save it again in Settings), an account without admin rights, or the portal unreachable. Fix it, then press **Send links to portal again** on the Prepare tab. |
| "vMix not reachable" | vMix isn't running, or Web Controller is off or not on port 8088 (B4). |
| Google: "Access blocked … can only be used within its organization" (org_internal) | The Google app is set to Internal but the channel is a Brand Account. Set the app's Audience to **External**. |
| YouTube sign-in stops working after about 7 days | The Google app is in **Testing** mode. Switch it to **In production**, or press Connect YouTube again. |
| End day pressed, but vMix still streaming | Older versions stopped only one destination. Update Stream Manager; End day now stops both. |

---

## Before a real tournament (one-time admin)

- **Google app "In production"** (Google Cloud → Google Auth Platform → Audience). In Testing mode
  the sign-in expires every 7 days.
- **Higher YouTube daily allowance** for 2 full courts: apply via the YouTube API Services audit and
  quota extension form. It needs a privacy policy and terms page.
- **Venue upload:** at least **50 Mbps** measured for two courts. Each court streams about 12 Mbps
  all the time, and about 24 Mbps briefly during a switch.
- Do a full dry run (C2) on the actual mini PCs.
