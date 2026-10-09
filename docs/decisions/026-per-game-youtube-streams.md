# 026 — Per-game YouTube live streams

**Date:** 2026-09-29
**Status:** accepted — approved by Eric, 2026-10-01; amended 2026-10-07 after review (see
"Amendments, 2026-10-07" at the end)

## Context

At tournaments we stream each court to YouTube as one long live video per day. Afterwards the
video is 10+ hours long and viewers cannot easily find a specific game. YouTube also only keeps
the last 12 hours of a live video, and our longest days are about 12 hours, so the start of a
day can be lost.

The tournament schedule (game numbers, teams, court, start time, timing rule) is already in
uwhportal, and the refbox already knows exactly when each game starts and ends. We want to use
that to split the stream into **one YouTube live video per game**, created automatically in
advance and listed in a playlist per court per day.

### Options considered

1. **One live video per game, switched automatically during the day** — chosen.
2. **Keep the all-day stream and add chapter timestamps afterwards** — rejected: still one long
   video, and it doesn't fix the 12-hour limit.
3. **Record locally, cut and upload each game afterwards** — rejected: nothing is available
   until after the day, it needs a lot of upload bandwidth at the venue, and uploads use up
   YouTube's daily allowance very quickly.

### Requirements gathered from the organizer

- One YouTube channel for everything; the organizer is the channel owner.
- One playlist per court per day, e.g. "Day 1 · Court 1", "Day 1 · Court 2".
- Streaming is done with **vMix** (not the Atem).
- At most **2 courts** streamed at once; the longest day is about **12 hours** (≈20–24 games
  per court).
- **Post-game interviews** happen after games and must stay inside that game's video.
- Live viewers clicking into the next game at each switch is acceptable.

## Decision

Build a new, separate program (working name: **`stream-manager`**) that runs on the Replay Box
(Mini PC) next to vMix. For each court it:

1. Reads the schedule from uwhportal.
2. Creates the YouTube live videos and playlists in advance.
3. Listens to that court's refbox to know where the game is.
4. Tells YouTube (and vMix) when to switch from one game's video to the next.

It does **not** change the refbox, `uwh-common`, or the overlay. It reads the portal using the
existing portal code in `uwh-common` and listens to the refbox the same way the overlay does,
without modifying either.

### 1. Preparing the day (run the day before)

The operator runs a "prepare" step for a chosen event and day. For each court:

- Create the playlist "Day N · Court X" if it doesn't exist.
- For every game on that court that day, create a scheduled YouTube live video:
  - **Title:** `<Event> · Court X · Game 14 · Team A vs Team B`
  - **Description:** event name, division/timing rule, scheduled start time, link to the portal.
  - **Scheduled start:** the portal start time (it only affects what fans see as "Upcoming").
  - Privacy (public or unlisted) and the "not made for kids" flag come from a config setting.
- Add each video to its playlist in game order.

Doing this the day before matters because YouTube's daily allowance resets every night (see
§7). The step can safely be re-run: it updates existing videos instead of creating duplicates.

### 2. Keeping titles in sync with the portal

While a day is running, the program re-reads the portal schedule every 10 minutes, and again
just before each switch. If the portal changed something for an upcoming game on this court
(placeholder teams resolved, e.g. "Winner G12" → real team name, or a changed start time), it
updates that game's YouTube title, description and scheduled start.

- Games removed from the portal are **not** deleted automatically. They're flagged on the
  operator page.
- When this court's allowance runs low (§7), the 10-minute checks stop, but the check just before
  each switch still runs, so the video that is about to go live always has the right title.
- Re-running Prepare also updates a changed start time, not only a changed title or
  description.

### 3. When to switch: just before the next game, not at the end of this one

A game's video keeps running after the final whistle, so the post-game interview, the result and
the teams leaving all stay in that game's video. The switch happens when the refbox's
between-games countdown for the next game reaches about **3:15** (configurable). So:

1. Game 14 ends. Game 14's video keeps running.
2. The interview takes place and stays in Game 14's video.
3. With about 3:15 to go before Game 15, the system switches to Game 15's video.
4. Game 15's video starts with the overlay's team rosters, then kickoff.

**Why 3:15:** the overlay follows the same refbox countdown. With more than ~3:02 to go it shows
the "next game" page (or the previous game's final score). From ~3:01 to 0:30 it shows the next
game's **team rosters** (player names and numbers). In the last 30 s it shows the pre-game
display. Switching at 3:15 means the rosters are never split between two videos. Because
stream-manager reads the same countdown as the overlay, the two stay in step even when games
run late.

**Protected roster window (3:01 → 0:30):** no automatic switch happens while the rosters are on
screen.

- If **Hold** is still on when the rosters start, the switch waits until the rosters finish
  (0:30), or until Hold is released after that. The next video then starts at the pre-game
  display, just before kickoff. The rosters stay whole in the previous video instead of being
  cut in half.
- **Switch now** always works, even inside the roster window. The operator decides.
- The operator page counts down to the roster start ("Rosters in 0:45") so the interviewer knows
  when to wrap up.

The first game of the day goes live when the operator presses **Start day** (or at the same
countdown point). The last game's video ends when the operator presses **End day**, so a final
interview is never cut off.

### 4. Operator controls

The program shows a small web page on the local network (streamdeck) for each court:

- **Now live** / **Up next**, and the time until the automatic switch.
- **Hold:** don't switch yet (e.g. interview still running). The current video keeps running
  until Hold is released, even if the next game has already started.
- **Switch now:** switch to the next game immediately.
- **Start day / End day.**
- Status: YouTube connected, vMix connected, this court's remaining allowance for today (§7),
  last error.

The operator never needs to touch the refbox.

**Stream Deck through Bitfocus Companion (primary control):** the organizer already programs
the Stream Deck with Companion for vMix. Each action (Hold, Switch now, Start day, End day) is a
simple web link on stream-manager. A Companion button triggers it with Companion's built-in
"Generic HTTP" connection, so no new software is needed on the Stream Deck side.

**Live status on the buttons.** Stream-manager pushes values into Companion's custom variables
through Companion's own remote-control interface, so buttons can show them:

- Hold on / off
- Time until the rosters ("Rosters in 0:45"), or "Rosters on screen"
- Now live ("Now: Game 14") and up next ("Next: Game 15")

It sends a value only when it changes. The Companion address is a setting, and the feature is
off while that setting is empty. A failure to reach Companion is shown on the operator page and
never affects switching. Companion version in use: **5.0.7**. The exact remote-control interface
is built against Companion's documentation and confirmed on the organizer's Stream Deck. The web
page stays as a status view and backup for when the Stream Deck isn't available.

**Building block:** the operator links and page use **axum**, a widely used Rust web-server
component built on the networking pieces the project already includes. Adding `stream-manager`
as a new workspace member was approved 2026-10-01; axum approved 2026-10-01.

**Test environment:** `dev.uwhportal.com` for the schedule, and the real YouTube channel with
**unlisted** videos and playlists.

### 5. Switching without a gap: two vMix outputs, taking turns

vMix can stream to several destinations at once, and it can be remote-controlled over the local
network. Each court uses **two YouTube stream connections (A and B)** that take turns: odd games
on A, even games on B. To switch from Game 14 (on A) to Game 15 (on B):

1. Tell vMix to start sending on output B.
2. Wait until YouTube confirms it is receiving B.
3. Put Game 15's video live.
4. End Game 14's video.
5. Tell vMix to stop output A (it will be used again for Game 16).

There's a few seconds of overlap instead of a gap, and Game 15 is already live when Game 14
ends, which gives YouTube's autoplay the best chance to pick it. During the overlap the upload
uses about double the bandwidth for a few seconds.

**Single-connection mode (per-court setting).** Each court has a setting for its stream keys:

- **Two keys, A and B** (default): the handoff above, with no gap.
- **One key:** vMix streams to that one key all day. At each switch the previous video is ended
  and the next one put live on the same stream. That leaves a gap of a few seconds, which lands in
  the line-up before kickoff. vMix's output is started at Start day and stopped at End day, and
  left running through every switch.

One-key mode is for venues whose upload can't take the brief doubling, or if the overlap turns
out not to work reliably. It is the same order already used when two games share a stream key.
The setting can only change while no day is running on that court.

#### vMix setup per court (GPU encoding confirmed working 2026-10-01)

- **YouTube Studio:** two reusable stream keys per court ("Court 1 – A", "Court 1 – B", …).
- **Streaming settings, Destinations 1 and 2:** Custom RTMP Server,
  `rtmp://a.rtmp.youtube.com/live2`, Destination 1 = A key, Destination 2 = B key. Do **not**
  use vMix's built-in YouTube sign-in (it creates its own live video).
- **Application:** **FFMPEG6**. The older "FFMPEG" fails with current NVIDIA drivers
  ("Cannot get the preset configuration: unsupported param").
- **Use Hardware Encoder:** ticked (NVIDIA eGPU). Check it in Task Manager → GPU → "Video
  Encode".
- **Quality (same on both destinations, confirmed 2026-10-01):** 1920×1080, 12000 kbps video,
  128 kbps AAC audio, Format H264, Profile High (or Main), Level 4.2 or Auto, **Preset P5** (not
  a Low Latency preset), keyframe frequency 2 s, stream delay 0, **Strict CBR and NAL CBR on**.
  - Without CBR the encoder dropped to about 0.3 Mbps on a still picture. New overlay graphics
    then arrived blocky and sharpened over several seconds, and YouTube warned about low
    bitrate. With both CBR options vMix sends a steady 12.1 Mbps and the problem is gone.
  - vMix 26 had no FFMPEG6 option; its old FFMPEG fails with current NVIDIA drivers. Use vMix
    29 (or newer) with FFMPEG6.
- **YouTube latency:** Normal latency gives the best picture.
- **Frame rate** (Settings → General → Video Frame Rate): 50 in 50 Hz countries, 59.94 in 60 Hz
  countries, to avoid pool-light flicker.
- **Web Controller** enabled (port 8088), so stream-manager can start and stop each
  destination.
- **Local recording** enabled as a backup.
- **Venue upload:** aim for ≥50 Mbps measured for 2 courts. With constant bitrate each court
  uses about 12 Mbps all the time, and about 24 Mbps briefly during a switch.

### 6. Helping viewers find the next game

YouTube autoplay can't be controlled, so at every switch the program also:

- Posts a message in the ending video's live chat: "Game 15 is live now → [link]".
- Adds "Next game: [link]" to the ending video's description (for replay viewers).

### 7. YouTube daily allowance (quota)

YouTube gives each app a daily allowance of 10,000 "units" by default. Almost every write
(creating a video, adding it to a playlist, going live, ending, a chat message, a title update)
costs about 50 units. **These costs are estimates and must be checked against Google's current
quota table before building.**

| Work | Per game | 2 courts × 24 games |
|------|---------:|--------------------:|
| Day before: create video, link to stream, add to playlist | ~150 | ~7,200 |
| On the day: go live, end, chat message, description update | ~200 | ~9,600 |
| On the day: title updates from the portal (some games only) | ~50 | up to ~2,400 |

Preparing the day before fits within one day's allowance. **On the day itself, 2 full courts are
at or over the default limit.** So:

- **Apply to Google for a higher allowance before the next tournament.** It's free but can take
  weeks. The channel owner applies through the Google Cloud console.
- Until it's granted, the program can drop the less important extras (chat message, description
  update) automatically when the allowance runs low. It **never** gives up the core switching.
- Checking YouTube's status costs 1 unit per check, so the program checks sparingly.

**How each court tracks its share.** The allowance belongs to the Google Cloud project, so all
courts share it. Each court's Stream Manager runs on its own mini PC and can only count what it
used itself. So:

- **Per-court share (setting).** Each court gets a share of the daily limit: the whole limit
  with one court, half each with two (the default). The daily limit is also a setting (10,000
  by default), raised once Google grants more.
- **Counted across restarts.** Units used are saved with the court's state, so a restart does
  not reset the count. The count resets when Google's day resets (midnight US Pacific time).
- **Shown as "remaining today".** The operator page shows this court's remaining share for
  today, not a count since the program started.
- **Extras stop first.** When the remaining share would no longer cover the rest of the day's
  switches on this court (plus a small margin), the program stops the extras: the chat message,
  the "Next game" link and the 10-minute title checks (§2). The page says so. The switches
  themselves, and the title check just before each switch, are never dropped.

The courts don't exchange counts with each other. That keeps each court independent of the
other mini PC, at the cost that one court can run short while the other has units left over.

### 8. Connecting YouTube

- The channel owner signs in once through Google's own sign-in page. The password is never typed
  into or stored by our program; only a Google-issued access token is kept on the Replay Box.
- Google Cloud project "Stream-Manager" was created 2026-10-01 with a **Desktop app** OAuth
  client.
  - The streams go to the **Atlantis Sports channel, a Brand Account**. Google does not
    count Brand Accounts as part of the Workspace organisation, so "Internal" was blocked
    (error 403 `org_internal`). The app is set to **External**.
  - **Publishing status to be confirmed by the channel owner before a tournament.** An earlier
    version of this section said it was changed to "In production" on 2026-10-01, while the
    open items said it couldn't be. It must be **In production**: in "Testing" the sign-in
    expires every 7 days. In production, Google shows a one-time "unverified app" warning at
    sign-in, and the app is limited to 100 users, which is irrelevant because only one sign-in
    is ever needed.
  - No logo and no listed scopes. The program requests
    `https://www.googleapis.com/auth/youtube` at sign-in.
- The downloaded client JSON file contains a secret. It must live **outside the project
  folder** (e.g. in the stream-manager settings folder on the Replay Box) so it can never be
  committed to GitHub. The program is told its location in its config.

### 9. When things go wrong

The rule: **a failure must never stop the vMix stream.** The worst outcome should be what
happens today: the current video keeps running longer than it should.

- **A switch fails (YouTube or internet unreachable, or YouTube refuses a step):** the current
  video stays live, the court goes to Hold, and the error is shown on the operator page. There
  are **no automatic retries**, so a failing step can't use up the daily allowance (§7). The
  operator retries with Switch now once it recovers.
- **Refbox connection lost:** automatic switching pauses; the operator uses Switch now.
- **Program crashes or restarts:** on start, each court asks YouTube which of its prepared videos
  is live (one status check), and carries on from there. The page shows "Resumed: Game 14 is
  live", and the operator does not press Start day again. If none is live, the court waits for
  Start day as usual. The A/B key in use follows from the live video's stream.

### Scope

**In scope (first version):** the new `stream-manager` program; the prepare step; automatic
switching driven by the refbox countdown; Hold / Switch now / Start day / End day on the web
page; vMix A/B handoff with single-connection mode as a per-court setting; the next-game chat
message and description link; portal title sync; quota-aware behaviour with a per-court share;
restart recovery; live status on the Stream Deck through Companion.

**Follow-up (separate PRs, after this one):** filling each game's watch link on the portal. Built
in the Stream Manager watch-links PR (stacked on #3521), together with the uwhportal PR. See
"Amendments, 2026-10-07".

**Out of scope for now (possible later):**

- Separate interview videos
- A per-court "always current game" link
- Automatic thumbnails with team flags
- The final score in the title or description
- Atem support
- Any change to the refbox, `uwh-common`, overlay, or wireless remote

### Changes to the workspace (need approval before building)

- A new workspace member `stream-manager` added to the root `Cargo.toml`.
- New dependencies for Google sign-in and the YouTube API, plus a small web server for the
  operator page. The exact choices will be proposed before anything is added.

### Acceptance criteria (what the organizer can check)

Tested at a practice session using **unlisted** videos and a small test event on the portal
(e.g. 3 short games on 2 courts):

1. After running "prepare", YouTube Studio shows playlists "Day 1 · Court 1" and "Day 1 ·
   Court 2", each with that court's games as upcoming live videos, correctly titled and in
   order.
2. Pressing Start day makes Game 1 go live.
3. After Game 1 ends, its video keeps running through a mock interview. It switches to Game 2
   about 3:15 before Game 2's start. Game 2's video opens with Game 2's full team rosters, and
   none of the rosters appear at the end of Game 1's video.
   - With Hold on past 3:00, no switch happens while the rosters are showing. After Hold is
     released, the switch happens at 0:30, when the rosters end.
4. Pressing Hold during the break stops the switch; releasing it (or pressing Switch now)
   switches.
5. The switch has no visible gap (A/B handoff), and a "Game 2 is live now" message appears in
   Game 1's chat.
6. Changing a team name on the portal updates the upcoming YouTube title within ~10 minutes.
7. Unplugging the internet for a minute doesn't stop vMix. Once reconnected, the operator can
   switch normally.
8. Each game's replay starts before kickoff and includes its interview.

### Rough task list

1. Check YouTube quota costs and the vMix remote-control commands; apply for a higher quota.
2. Set up the program skeleton, config file (event, courts → refbox address, vMix address,
   playlist naming, privacy, switch lead time), and the one-time YouTube sign-in.
3. Build the prepare step: read the portal schedule, create or update playlists and videos
   (safe to re-run).
4. Listen to the refbox per court and detect the switch point.
5. Build the switching sequence: A/B handoff through vMix, plus the single-connection
   fallback.
6. Build the operator web page: Hold, Switch now, Start day / End day, status.
7. Add the chat message, description link and portal title sync.
8. Add quota tracking and failure handling (§7 and §9).
9. Practice-session test against the acceptance criteria.

## Consequences

- Every game becomes its own video in a court/day playlist. Fans can find games directly and
  set reminders for upcoming ones.
- The 12-hour live-archive limit no longer matters.
- Live viewers are bounced to a new video at each switch. The chat message and autoplay help,
  but some viewers will need to click.
- The stream operator gains a new job during the day: using Hold / Switch now around interviews.
- We depend on YouTube's daily allowance. Two full courts need a quota increase from Google,
  which must be requested in advance.
- vMix needs two stream destinations configured per court, and the venue upload must handle a
  brief doubling during each switch.
- Running this is optional: if the program isn't started, streaming works exactly as today.

### Open items to verify before or during building

- **Google app publishing status (§8): to be confirmed by the channel owner.** If it is still
  "Testing", the organizer's account must be listed as a test user and the YouTube sign-in
  expires every 7 days. It **must be In production before a tournament.**

- Exact YouTube quota costs per action (§7).
- Whether YouTube lets the A/B connections overlap cleanly, and how long "waiting until YouTube
  receives B" takes in practice.
- The exact vMix remote-control commands for starting and stopping one specific output.
- How YouTube autoplay behaves for a viewer who opened the video from the playlist.
- That the refbox's between-games countdown refers to the *next* game's number, so the right
  video is picked.

## Deviations

- **Step 1 (2026-10-01):** stream-manager reads the public `/api/events/{slug}/schedule` response
  with its own small parser, instead of `uwh-common`'s `Schedule` type. The team fields in that
  type are private, and changing `uwh-common` is out of scope. The response already includes
  the event name and team names, so one request is enough.
- **Observed, not acted on:** each portal game has a `watchUrl` field (currently `null` on dev).
  A later step could fill it with the game's YouTube link so the portal links straight to the
  video. That needs a portal write API and a separate discussion.
- **Step 2 (2026-10-01):** commands `connect`, `check-youtube`, `prepare --day N [--court C]
  [--limit N]` and `cleanup` were added.
  - `prepare` shows the work and its estimated cost and asks for confirmation before changing
    anything.
  - What it creates is recorded in `Documents\stream-manager\state-<event>.json`, which is what
    makes re-runs safe.
  - Games whose portal start time has already passed (e.g. old test events) are scheduled
    15 minutes from now, because YouTube expects an upcoming start.
  - `cleanup` (for tests) permanently deletes the recorded videos and playlists after
    confirmation.
- **Step 3 (2026-10-01): control page.** The organizer asked for buttons instead of terminal
  commands.
  - Running `stream-manager` with no command (a double-click) starts a web control page on
    port 8090 and opens it in the browser.
  - Tabs: Live (per-court status and Start day / Hold / Switch now / End day), Prepare,
    Settings (event picker, courts, privacy, PIN, YouTube connect/check) and Test tools
    (delete test videos).
  - Protected by a PIN, set on first use from the laptop itself. Companion uses
    `GET /api/court/<court>/<start|hold|release|hold-toggle|next|end>?pin=<PIN>`.
    (Changed by Amendment 10: the page answers only the mini PC itself by default, and Companion
    uses a button key, not the PIN.)
  - ~~Runs on a separate laptop~~ — superseded on 2026-10-02: **one Stream Manager per court,
    on that court's mini PC** (with vMix, overlay and overlay-bridge), each handling only its own
    court. If one mini PC fails, the other court keeps switching. The other court is viewed by
    opening its control page (`http://<other mini PC>:8090`) in another tab. Setup steps:
    `docs/streaming-setup.md`.
  - The Live tab is in **practice mode**: switching decisions are shown and logged, but nothing
    is sent to YouTube or vMix until the next step.
  - The terminal `watch` command was removed (replaced by the Live tab).
  - Tested with a fake refbox: Start day, the automatic switch at 3:15, Hold blocking it, and
    release during rosters waiting until 0:30.
  - The phone-size layout is not yet visually verified.
- **Step 4 (2026-10-01): live switching.** Practice mode is now a setting (on by default, and
  can only change while no day is running).
  - With practice mode off, each court has a worker that carries out Start day / switches /
    End day on vMix and YouTube in the order of §5 and §6.
  - vMix Web Controller address per court: destination 1 = stream key A, destination 2 = B.
  - The chat message is posted *before* the old video ends, because its chat closes when it
    ends.
  - If both videos share a stream key (e.g. a skipped game), the old one ends first and the new
    one starts on the running stream, with a gap of a few seconds.
  - Verified so far: the failure path (no YouTube connection → nothing touched, clear error,
    court not started) and vMix reachability. **The real switch on YouTube/vMix is still to be
    tested by the organizer.**
- **Fix found in the organizer's test (2026-10-01):** the refbox keeps `game_number` at the
  *previous* game for the whole break ("0" before the first game) and only changes it at
  kickoff. The upcoming game is always `next_game_number` during a break. The earlier rule
  wrongly used `game_number` after the refbox reset, which caused "Game 0" on Start day and
  would have missed switches. Fixed, with a regression test.
- **Ready-made downloads (2026-10-03):** `.github/workflows/streaming-tools.yml` builds the three
  Windows programs for the court mini PCs (overlay with `ndi,bridge`, overlay-bridge,
  stream-manager) into `streaming-tools-windows.zip`.
  - It runs on PRs and pushes that touch them, can be started by hand, and is called by
    `release.yml` so every release carries the same zip.
  - The NDI SDK is installed on the build machine from NDI's official installer, after checking
    its SHA-256 (`.github/actions/setup-ndi-windows`, adapted from grafton-ndi's CI). None of the
    SDK's files are shipped. The zip's README and the setup guide carry the NDI® trademark notice
    and the https://ndi.video link, as the NDI SDK licence requires.
  - Approved by the organizer, who is responsible for the project and has read and accepted the
    NDI SDK License Agreement on its behalf (2026-10-03). The workflow installs the SDK on the
    project's behalf.
- **Portal watch links (2026-10-05, withdrawn 2026-10-07):** a first version filled each game's
  `watchUrl` on the portal through the admin endpoint `POST /api/admin/update-games-watch-urls`,
  signing in with a saved portal admin email and password. It was taken out of this PR on
  review. See "Amendments, 2026-10-07".
- **Portal watch links, built (2026-10-08):** none. Built as Amendment 9 describes.

## Amendments, 2026-10-07

Made after a review of the PR against this ADR, and approved by Eric.

1. **Restart recovery is built** (§9), so the old "restart only during a game" workaround is gone.
2. **The daily allowance is tracked per court** (§7): a share setting, counted across restarts,
   shown as "remaining today", with the extras dropped first when it runs low.
3. **Portal title sync is built** (§2), and Prepare also updates a changed start time.
4. **Single-connection mode is a per-court setting** (§5), not only the shared-key edge case.
5. **Live status on the Stream Deck is built** (§4), through Companion's custom variables.
6. **A failed switch goes to Hold with no automatic retries** (§9). This was already the
   behaviour, and §9 now says so instead of "retry".
7. **The Google app's publishing status is to be confirmed** (§8). The two earlier statements
   contradicted each other.
8. **The overlay and overlay-bridge changes made during this work move to their own PR**, which
   merges first: the NDI engine installed when the operator clicks **Install NDI** in the
   overlay's window (never by itself), the bridge moved off vMix's port 8099
   to 8098, and a refbox on the same PC listed once in the bridge's scan. This PR then changes
   no overlay or overlay-bridge code, as the Decision section says.
9. **Portal watch links move to follow-up PRs, signed in the way the refbox signs in.** No
   portal email or password is stored, ever.
   - **How it works:** Stream Manager shows its own ID. An event organiser enters that ID on the
     event's page in the portal and gets a short code. The operator types the code into Stream
     Manager's Settings tab. Stream Manager swaps the code for a key that works only for that
     event, and only for setting that event's watch links. It expires the day after the event and
     the organiser can cancel it at any time. This mirrors the refbox's link
     (`/api/events/{id}/access-keys/ref-box`), which issues a key for pushing scores only.
   - **What it needs:** a uwhportal PR first. It adds a Stream Manager link with its own key
     type, and lets the watch-link update accept that key. A follow-up Stream Manager PR then
     adds the code entry and fills the links after Prepare, clearing them again when test videos
     are deleted.
   - **Why it was taken out:** none of this work is in use yet, so nothing depends on the links in
     the meantime. Taking it out means this PR builds nothing that would later be thrown away:
     no stored admin login, and no password encryption through PowerShell (whose test failed on
     GitHub's Windows machines).
   - **Built (2026-10-08):** in the Stream Manager watch-links PR (stacked on #3521), together
     with the uwhportal PR. The Settings tab's **Portal watch links** box shows the ID and takes
     the code. Changing the event or the portal forgets the key.
10. **The control page answers only the mini PC it runs on, unless told otherwise; Stream Deck
    links use a button key, not the PIN.** A review showed that anyone on the venue network could
    find the 4-digit PIN in seconds by trying every PIN at once, because Companion's links carried
    it.
    - By default Stream Manager listens only on the mini PC itself. **Allow other devices** (a
      setting that can only be changed on the mini PC, applied after a restart) lets listed
      addresses in: a tablet, a laptop running Companion, or the other court's mini PC. Viewing
      the other court's page therefore needs this mini PC's address added on that court.
    - Companion's links carry a long random **button key**, shown and renewed only on the mini PC
      itself.
    - The PIN only signs in on the page. Sign-ins are checked one at a time, with a growing wait
      after each wrong PIN.
