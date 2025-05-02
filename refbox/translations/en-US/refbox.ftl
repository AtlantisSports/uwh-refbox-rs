# Definitions for the translation file to use
-dark-team-name = Black
dark-team-name-caps = BLACK
-light-team-name = White
light-team-name-caps = WHITE

# Multipage 
done = DONE
cancel = CANCEL
delete = DELETE
back = BACK
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
    LENGTH

# Warning Add
team-warning = TEAM
    WARNING
team-warning-line-1 = TEAM
team-warning-line-2 = WARNING

# Configuration
none-selected = None Selected
loading = Loading...
game-select = Game:
game-options = GAME OPTIONS
app-options = APP OPTIONS
display-options = DISPLAY OPTIONS
sound-options = SOUND OPTIONS
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
using-uwh-portal = USING UWHPORTAL:
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

help = HELP: 

# Confirmation
game-configuration-can-not-be-changed = The game configuration can not be changed while a game is in progress.
    
    What would you like to do?
apply-this-game-number-change = How would you like to apply this game number change?
UWHPortal-enabled = When UWHPortal is enabled, all fields must be filled out.
go-back-to-editor = GO BACK TO EDITOR
discard-changes = DISCARD CHANGES
end-current-game-and-apply-changes = END CURRENT GAME AND APPLY CHANGES
end-current-game-and-apply-change = END CURRENT GAME AND APPLY CHANGE
keep-current-game-and-apply-change = KEEP CURRENT GAME AND APPLY CHANGE
ok = OK
confirm-score = Is this score correct?
    Confirm with chief referee.
    
    Black: { $score_black }        White: { $score_white }
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
min-brk-btwn-games = Minimum Time Between Games: { $min_brk_time }


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

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = END
end-timeout-line-2 = { timeout }
switch-to = SWITCH TO
ref = REF
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
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
team-timeouts-per-half = Team Timeouts Allowed Per Half: { $team_timeouts }
team-timeouts-per-game = Team Timeouts Allowed Per Game: { $team_timeouts }
stop-clock-last-2 = Stop Clock in Last 2 Minutes: { $stop_clock }
ref-list = Chief Ref: { $chief_ref }
    Timer: { $timer }
    Water Ref 1: { $water_ref_1 }
    Water Ref 2: { $water_ref_2 }
    Water Ref 3: { $water_ref_3 }
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
zero = ZERO

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
