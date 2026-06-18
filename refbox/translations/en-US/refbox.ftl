# Definitions for the translation file to use
-dark-team-name = Black
dark-team-name-caps = BLACK
-light-team-name = White
light-team-name-caps = WHITE

# Multipage
done = DONE
restart-to-apply = RESTART TO APPLY
cancel = CANCEL
delete = DELETE
back = BACK
apply = APPLY
save = SAVE
user-options = USER OPTIONS
new = NEW

# Penalty Edit
total-dismissal = TD
penalty-kind = {$kind ->
    [thirty-seconds] 30s
    [one-minute] 1m
    [two-minutes] 2m
    [four-minutes] 4m
    [five-minutes] 5m
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# Team Timeout Edit
timeout-length = TEAM TIMEOUT
    LENGTH:
team-timeout-count = TEAM TIMEOUT
    COUNT:

# Warning Add
team-warning = TEAM
    WARNING
team-warning-line-1 = TEAM
team-warning-line-2 = WARNING

# Configuration
none-selected = None Selected
loading = Loading...
game-select = GAME:
game-options = GAME OPTIONS
app-options = APP OPTIONS
display-options = DISPLAY OPTIONS
open-new-display = OPEN NEW DISPLAY
front-display-layout = DISPLAY LAYOUT
layout-default = DEFAULT
layout-classic = CLASSIC
layout-big-time = BIG TIME
layout-corners = CORNERS
layout-scores-only = SCORES ONLY
sound-options = SOUND OPTIONS
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
sound-settings = SOUND SETTINGS
beep-test-edit-levels = EDIT LEVELS
app-mode = APP
    MODE
hide-time-for-last-15-seconds = HIDE TIME FOR
    LAST 15 SECONDS
player-display-brightness = PLAYER DISPLAY
    BRIGHTNESS
confirm-score-at-game-end = CONFIRM SCORE
    AT GAME END
track-cap-number-of-scorer = TRACK CAP NUMBER
    OF SCORER
event = EVENT:
track-fouls-and-warnings = TRACK FOULS
    AND WARNINGS
show-behind-schedule-time = SHOW BEHIND TIME/DELAY
delay = DELAY
court = COURT:
single-half = SINGLE
    HALF:
half-length-full = HALF LENGTH:
game-length = GAME LENGTH:
overtime-allowed = OVERTIME 
    ALLOWED:
sudden-death-allowed = SUDDEN DEATH 
    ALLOWED:
half-time-length = HALF TIME
    LENGTH:
pre-ot-break-length = PRE OT
    BREAK LENGTH:
pre-sd-break-length = PRE SD
    BREAK LENGTH:
nominal-break-between-games = NOMINAL BRK
    BTWN GAMES:
ot-half-length = OT HALF
    LENGTH:
timeouts-counted-per = TIMEOUTS
    COUNTED PER:
game = GAME
half = HALF
minimum-brk-btwn-games = MINIMUM BRK
    BTWN GAMES:
ot-half-time-length = OT HALF
    TIME LENGTH
using-portal = USING { $portal }PORTAL:
starting-sides = STARTING SIDES 
sound-enabled = SOUND
    ENABLED:
whistle-volume = WHISTLE
    VOLUME:
manage-remotes = MANAGE REMOTES
whistle-enabled = WHISTLE 
    ENABLED:
above-water-volume = ABOVE WATER
    VOLUME:
auto-sound-start-play = AUTO SOUND
    START PLAY:
buzzer-sound = BUZZER 
    SOUND:
underwater-volume = UNDERWATER
    VOLUME:
auto-sound-stop-play = AUTO SOUND
    STOP PLAY:
alarm-button = ALARM
    BUTTON:
alarm = ALARM
hold-to-test = HOLD TO TEST
or-press-spacebar = Or Press Spacebar
or-hold-spacebar = Or Hold Spacebar
game-info = GAME INFO
remotes = REMOTES
default = DEFAULT
sound = SOUND: { $sound_text }
brightness = { $brightness ->
        *[Low] LOW
        [Medium] MEDIUM
        [High] HIGH
        [Outdoor] OUTDOOR
    }

waiting = WAITING
add = ADD
half-length = HALF LEN
length-of-half-during-regular-play = The length of a half during regular play
half-time-lenght = HALF TIME LEN
length-of-half-time-period = The length of the Half Time period
nom-break = NOM BREAK
game-block = GAME BLOCK
game-block-full = GAME BLOCK:
game-block-help = Time from the start of one game to the start of the next
game-block-too-short = Too short to fit the game plus the minimum break
game-block-tight = Tight — team timeouts could push games past their slot
system-will-keep-game-times-spaced = The system will try to keep the game start times evenly spaced, with the
    total time from one start to the next being 2 * [Half Length] + [Half Time
    Length] + [Nominal Time Between Games] (example: if games have [Half
    Length] = 15m, [Half Time Length] = 3m, and [Nominal Time Between Games] =
    12m, the time from the start of one game to the next will be 45m. Any
    timeouts taken, or other clock stoppages, will reduce the 12m time down
    until the minimum time between game value is reached).
min-break = MIN BREAK
min-time-btwn-games = If a game runs longer than scheduled, this is the minimum time between
            games that the system will allot. If the games fall behind, the system will
            automatically try to catch up after subsequent games, always respecting
            this minimum time between games.
pre-ot-break-abreviated = PRE OT BREAK
pre-sd-brk = If overtime is enabled and needed, this is the length of the break between
            Second Half and Overtime First Half
ot-half-len = OT HALF LEN
time-during-ot = The length of a half during overtime
ot-half-tm-len = OT HLF TM LEN
len-of-overtime-halftime = The length of Overtime Half Time
pre-sd-break = PRE SD BREAK
pre-sd-len = The length of the break between the preceeding play period and Sudden Death
language = LANGUAGE
this-language = ENGLISH
portal-login-code = CODE
portal-login-instructions = Please go to the { $portal } Portal >> Event Management >> Referee Management, click on the + button to add a new Refbox, and enter this Refbox ID:
    { $id }

    The { $portal } Portal will then provide a confirmation code for you to enter to the left using the number pad.
    Press done once you have entered the code

help = HELP: 

# Confirmation
game-configuration-can-not-be-changed = The game configuration can not be changed while a game is in progress.
    
    What would you like to do?
apply-this-game-number-change = How would you like to apply this game number change?
portal-enabled = When { $portal }PORTAL is enabled, all fields must be filled out.
mode-switch-portal-tenant = Changing mode from { $from_mode } to { $to_mode } will disable the link to { $from_portal }PORTAL and you must re-connect to { $to_portal }PORTAL.
uwhportal-token-invalid-code = Invalid code entered.
    Please try again.
uwhportal-token-no-pending-link = Portal not expecting a connection.
    Please try again.
go-back-to-editor = GO BACK TO EDITOR
discard-changes = DISCARD CHANGES
end-current-game-and-apply-changes = END CURRENT GAME AND APPLY CHANGES
end-current-game-and-apply-change = END CURRENT GAME AND APPLY CHANGE
keep-current-game-and-apply-change = KEEP CURRENT GAME AND APPLY CHANGE
ok = OK
confirm-score = Is this score correct?
    Confirm with chief referee.
    
    Black: { $score_black }        White: { $score_white }

    { confirmation-count-down }
yes = YES
no = NO

# Fouls
equal = EQUAL

# Game Info
refresh = REFRESH
refreshing = REFRESHING...
settings = SETTINGS 
none = None
game-number-error = Error ({ $game_number })
next-game-number-error = Error ({ $next_game_number })
last-game-next-game = Last Game: { $prev_game },
    Next Game: { $next_game }
black-team-white-team = Black Team: { $black_team }
    White Team: { $white_team }
game-length-ot-allowed = Half Length: { $half_length }
         Half Time Length: { $half_time_length }
         Overtime Allowed: { $overtime }
overtime-details = Pre-Overtime Break Length: { $pre_overtime }
             Overtime Half Length: { $overtime_len }
             Overtime Half Time Length: { $overtime_half_time_len }
sd-allowed = Sudden Death Allowed: { $sd }
pre-sd = Pre-Sudden-Death Break Length: { $pre_sd_len }
team-to-len = Team Timeout Duration: { $to_len }
time-btwn-games = Nominal Time Between Games: { $time_btwn }
game-block-info = Game Block: { $game_block }
min-brk-btwn-games = Minimum Time Between Games: { $min_brk_time }

# Game-info table labels
gi-prior-game = Prior Game
gi-team-light = { -light-team-name }
gi-team-dark = { -dark-team-name }
gi-current-game = Current Game
gi-next-game = Next Game
gi-game-block = Game Block
gi-half-length = Half Length
gi-half-time-length = Half-Time Length
gi-game-length = Game Length
gi-timeouts = Timeouts
gi-timeout-duration = Timeout Duration
gi-overtime = Overtime
gi-sudden-death = Sudden Death
gi-pre-overtime-break = Pre-Overtime Break
gi-pre-sudden-death-break = Pre-Sudden Death Break
gi-overtime-half-length = Overtime Half Length
gi-overtime-half-time-length = Overtime Half-Time Length
gi-minimum-game-break = Minimum Game Break
gi-stop-clock-last-2 = Stop Clock in Last 2 Min
gi-ref-chief = Chief Referee
gi-ref-timekeeper = Time/Score Keeper
gi-ref-timekeeper-helper = Time/Score Helper
gi-ref-water-1 = Water Referee 1
gi-ref-water-2 = Water Referee 2
gi-ref-water-3 = Water Referee 3
gi-ref-water-referees = Water Referees
gi-ref-deck-referees = Deck Referees

# List Selecters
select-event = SELECT EVENT
select-court = SELECT COURT
select-game = SELECT GAME

# Main View
add-warning = ADD WARNING
add-foul = ADD FOUL
start-now = START NOW
end-timeout = END TIMEOUT
warnings = WARNINGS
penalties = PENALTIES
dark-score-line-1 = SCORE
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = SCORE
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = BLACK PENALTIES
white-penalties = WHITE PENALTIES

# Score edit
final-score = Please enter the final score
confirmation-count-down = Note: The unchanged score will be automatically confirmed in { $countdown }

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = END
end-timeout-line-2 = { timeout }
cancel-timeout = { cancel } { timeout }
cancel-timeout-line-1 = { cancel }
cancel-timeout-line-2 = { timeout }
cancel-ref-timeout = { cancel } { ref } { timeout }
cancel-ref-timeout-line-1 = { cancel } { ref }
cancel-ref-timeout-line-2 = { timeout }
cancel-pen-shot = { cancel } { pen-shot }
cancel-pen-shot-line-1 = { cancel }
cancel-pen-shot-line-2 = { pen-shot }
switch-to = SWITCH TO
ref = REF
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
revive-hold-line-1 = HOLD TO
revive-hold-line-2 = RESTORE
revive-deciding-line-2 = RESTORED
penalty-shot-line-1 = PENALTY
penalty-shot-line-2 = SHOT
pen-shot = PEN SHOT
## Penalty string
served = Served
penalty = #{$player_number} - {$time ->
        [pending] Pending
        [served] Served
        [total-dismissal] Dismissed
       *[number] {$time}
    } {$time ->
        [total-dismissal] {""}
       *[other] ({$kind ->
           *[any] { penalty-kind }
        })
    }
foul = {$player_number ->
        [none] {$infraction}
        *[number] #{$player_number} - {$infraction}
    }
warning = {$player_number ->
        [none] { team-warning-abreviation } - {$infraction}
        *[number] #{$player_number} - {$infraction}
    }
infraction = Infraction: {$infraction}
## Config String
error = Error ({ $number })
two-games = Last Game: { $prev_game },  Next Game: { $next_game }
one-game = Game: { $game }
teams = { -dark-team-name } Team: { $dark_team }
    { -light-team-name } Team: { $light_team }
game-config = Half Length: { $half_len },  Half Time Length: { $half_time_len }
    Sudden Death Allowed: { $sd_allowed },  Overtime Allowed: { $ot_allowed }
team-timeouts = Team Timeouts: { $value }
team-timeouts-label = TEAM
    TIMEOUTS:
stop-clock-last-2 = Stop Clock in Last 2 Minutes: { $stop_clock }
ref-list = Chief Ref: { $chief_ref }
    Timer: { $timer }
    Water Ref 1: { $water_ref_1 }
    Water Ref 2: { $water_ref_2 }
    Water Ref 3: { $water_ref_3 }
team-ref-list = Referees: { $ref_team }
    T/S Keeper: { $ts_keeper_team }
unknown = Unknown
## Game time button
next-game = NEXT GAME
first-half = FIRST HALF
half-time = HALF TIME
second-half = SECOND HALF
pre-ot-break-full = PRE OVERTIME BREAK
overtime-first-half = OVERTIME FIRST HALF
overtime-half-time = OVERTIME HALF TIME
overtime-second-half = OVERTIME SECOND HALF
pre-sudden-death-break = PRE SUDDEN DEATH BREAK
sudden-death = SUDDEN DEATH
ot-first-half = OT FIRST HALF
ot-half-time = OT HALF TIME
ot-2nd-half = OT 2ND HALF
white-timeout-short = WHT T/O
white-timeout-full = WHITE TIMEOUT
black-timeout-short = BLK T/O
black-timeout-full = BLACK TIMEOUT
ref-timeout-short = REF TMOUT
penalty-shot-short = PNLTY SHT
## Make warning container
team-warning-abreviation = T
## Make time editor
zero = = 0

# Time edit
game-time = GAME TIME
timeout = TIMEOUT
Note-Game-time-is-paused = Note: Game time is paused while on this screen

# Warning Fouls Summary
fouls = FOULS
edit-warnings = EDIT WARNINGS
edit-fouls = EDIT FOULS

# Warnings
black-warnings = BLACK WARNINGS
white-warnings = WHITE WARNINGS

# Message
player-number = PLAYER
    NUMBER:
game-number = GAME
    NUMBER:
num-tos-per-half = NUM OF TEAM
    T/Os PER HALF:
num-tos-per-game = NUM OF TEAM
    T/Os PER GAME:

# Sound Controller - mod
off = OFF
low = LOW
medium = MEDIUM
high = HIGH
max = MAX

# Config
hockey6v6 = HOCKEY6V6
hockey3v3 = HOCKEY3V3
rugby = RUGBY
beep-test = BEEP TEST

# Beep-test screen
beep-test-pre = PRE
beep-test-top-time-label = TIME
beep-test-top-level-label = LEVEL
beep-test-top-lap-label = LAP
beep-test-start = START
beep-test-pause = PAUSE
beep-test-resume = RESUME
beep-test-reset = RESET
beep-test-column-level = LEVEL
beep-test-column-count = COUNT
beep-test-column-duration = DURATION
beep-test-edit-selected = Level { $level }
beep-test-edit-time = TIME
beep-test-edit-count = COUNT
beep-test-edit-new = ADD LEVEL
beep-test-edit-remove = REMOVE LEVEL

# Infractions
stick-foul = Stick Foul
illegal-advance = Illegal Advance
sub-foul = Sub Foul
illegal-stoppage = Illegal Stoppage
out-of-bounds = Out Of Bounds
grabbing-the-wall = Grabbing The Wall
obstruction = Obstruction
delay-of-game = Delay Of Game
unsportsmanlike = Unsportsmanlike
free-arm = Free Arm
false-start = False Start

# Portal Health Indicator
portal-summary-title = { $portal } PORTAL STATUS
portal-row-token-expired = Portal login expired — tap to re-login
portal-row-stuck = Game { $game } Score send error, tap to fix
portal-row-pending = Game { $game } Score not sent, tap to retry
portal-row-attempt-suffix = (attempt { $attempts })
portal-row-recent = Game { $game } · Submitted { $mins } min ago
portal-action-force-submit = Retry this game result
portal-action-discard = Discard this game result
portal-action-discard-confirm = TAP AGAIN TO CONFIRM DISCARD
portal-page-title-attention = Game { $game } submission error
portal-page-attention-info = The game result has not been accepted on { $portal } Portal
portal-page-attention-score = Stored game result: Light { $white } - Dark { $black }
portal-page-attention-remediation = You can Retry if connection is verified, or discard to clear the error
portal-advisory-at-game-end = Portal issue detected. Score will still be queued — find an admin to resolve.

# 2 Halves / 1 Period selector (Half Length editor)
two-halves = 2 HALVES
one-period = 1 PERIOD
game-len = GAME LEN
length-of-game-during-regular-play = The length of the game during regular play

# Single-period (1 Period) info variants — Game Length, no half-time line
game-config-single-half = Game Length: { $half_len }
    Sudden Death Allowed: { $sd_allowed },  Overtime Allowed: { $ot_allowed }
game-length-ot-allowed-single-half = Game Length: { $half_length }
         Overtime Allowed: { $overtime }

# Self-update / Updates page
check-version = Check Version
updates-current-version = Current version
updates-check-for-updates = Check for Updates
updates-install = Install
updates-do-revert = Revert
updates-install-note = Clicking install will download, install, and restart the refbox
updates-revert-note = Clicking revert will restore the previous version and restart the refbox
updates-unknown = Unknown
updates-checking = Checking…
updates-up-to-date = Up to date.
updates-available = Update available: {$version}
updates-downloading = Downloading…
updates-verifying = Checking the download…
updates-installing = Installing…
updates-restarting = Restarting…
updates-confirm-revert = Revert to the previous version ({$version})?
updates-rolled-back = Reverted to the previous version because the update didn’t start correctly, please try again.
updates-revert = Revert to Previous Version ({$version})
updates-error-no-internet = Couldn’t reach the update server, please check your internet connection
updates-error-bad-download = The downloaded update wasn’t valid and was not installed.
updates-error-rate-limited = The update server is busy, please try again in a little while.
updates-error-no-space = Not enough free space to install the update.
updates-error-not-writable = The update couldn’t be saved (permission denied).
gi-unknown = ???
