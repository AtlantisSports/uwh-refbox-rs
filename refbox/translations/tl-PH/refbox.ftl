# Mga kahulugan para gamitin sa translation file
-dark-team-name = Itim
dark-team-name-caps = ITIM

-light-team-name = Puti
light-team-name-caps = PUTI

# Maraming pahina
done = TAPOS
restart-to-apply = I-RESTART UPANG ILAPAT
cancel = KANSELAHIN
delete = BURAHIN
back = BUMALIK
new = BAGO

# Pag-edit ng Penalty
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

# Pag-edit ng Timeout ng Koponan
timeout-length = TIMEOUT NG
    KOPONAN

# Pagdagdag ng Babala
team-warning = BABALA NG
    KOPONAN
team-warning-line-1 = BABALA NG
team-warning-line-2 = KOPONAN

# Configuration
none-selected = Wala na Pinili
loading = Naglo-load...
game-select = Laro:
game-options = MGA OPSYON SA LARO
app-options = MGA OPSYON SA APP
display-options = MGA OPSYON SA DISPLAY
sound-options = MGA OPSYON SA TUNOG
app-mode = MODE NG
    APP
hide-time-for-last-15-seconds = ITAGO ANG ORAS SA
    HULING 15 SEGUNDO
player-display-brightness = LIWANAG NG
    DISPLAY NG MANLALARO
confirm-score-at-game-end = KUMPIRMAHIN ANG PUNTOS
    SA KATAPUSAN NG LARO
track-cap-number-of-scorer = SUBAYBAYAN ANG NUMERO
    NG GORRO NG MANLALARO
event = KAGANAPAN:
track-fouls-and-warnings = SUBAYBAYAN ANG POUL
    AT BABALA
court = KORTE:
single-half = ISANG
    KALAHATI:
half-length-full = HABA NG KALAHATI:
game-length = HABA NG LARO:
overtime-allowed = OVERTIME
    PINAHINTULUTAN:
sudden-death-allowed = SUDDEN DEATH
    PINAHINTULUTAN:
half-time-length = HABA NG
    PAHINGA:
pre-ot-break-length = HABA NG PAHINGA
    BAGO OT:
pre-sd-break-length = HABA NG PAHINGA
    BAGO SD:
nominal-break-between-games = NOMINAL NA PAHINGA
    SA PAGITAN NG MGA LARO:
ot-half-length = HABA NG
    KALAHATI SA OT:
timeouts-counted-per = MGA TIMEOUT
    BIBILANGIN PER:
game = LARO
half = KALAHATI
minimum-brk-btwn-games = PINAKAMALIIT NA PAHINGA
    SA PAGITAN NG MGA LARO:
ot-half-time-length = HABA NG PAHINGA
    SA OVERTIME
using-uwh-portal = GUMAGAMIT NG UWHPORTAL:
starting-sides = MGA PANIG SA SIMULA
sound-enabled = TUNOG
    PINAGANA:
whistle-volume = LAKAS NG
    SIPOL:
manage-remotes = PAMAHALAAN ANG MGA REMOTE
whistle-enabled = SIPOL
    PINAGANA:
above-water-volume = LAKAS SA
    IBABAW NG TUBIG:
auto-sound-start-play = AUTO TUNOG
    SIMULA NG LARO:
buzzer-sound = TUNOG NG
    BUZZER:
underwater-volume = LAKAS SA
    ILALIM NG TUBIG:
auto-sound-stop-play = AUTO TUNOG
    HINTO NG LARO:
alarm-button = PINDUTAN NG
    ALARMA:
alarm = ALARMA
hold-to-test = PINDUTIN NANG MATAGAL PARA SUBUKAN
or-press-spacebar = O Pindutin ang Spacebar
or-hold-spacebar = O Pindutin Nang Matagal ang Spacebar
game-info = IMPORMASYON NG LARO
remotes = MGA REMOTE
default = DEFAULT
sound = TUNOG: { $sound_text }
brightness = { $brightness ->
        *[Low] MABABA
        [Medium] KATAMTAMAN
        [High] MATAAS
        [Outdoor] PANLABAS
    }

waiting = NAGHIHINTAY
add = IDAGDAG
half-length = HABA NG KALAHATI
length-of-half-during-regular-play = Ang haba ng isang kalahati sa panahon ng regular na laro
half-time-lenght = HABA NG PAHINGA
length-of-half-time-period = Ang haba ng panahon ng pahinga
nom-break = NOMINAL NA PAHINGA
system-will-keep-game-times-spaced = Susubukan ng sistema na panatilihing pantay ang mga oras ng simula ng laro, na ang kabuuang oras mula sa isang simula hanggang sa susunod ay 2 × [Haba ng Kalahati] + [Haba ng Pahinga] + [Nominal na Oras sa Pagitan ng mga Laro] (halimbawa: kung ang [Haba ng Kalahati] = 15 min, [Haba ng Pahinga] = 3 min at [Nominal na Oras sa Pagitan ng mga Laro] = 12 min, ang oras mula simula hanggang simula ay magiging 45 min. Ang anumang mga timeout o iba pang paghinto ng orasan ay magbabawas ng 12 min hanggang maabot ang pinakamaliit na oras sa pagitan ng mga laro).
min-break = PINAKAMALIIT NA PAHINGA
min-time-btwn-games = Kung ang isang laro ay mas matagal kaysa sa nakatakda, ito ang pinakamaliit na oras sa pagitan ng mga larong itatalaga ng sistema. Kung nahuhuli ang mga laro, awtomatikong susubukan ng sistema na makahabol sa mga susunod na laro, palaging iginagalang ang pinakamaliit na oras na ito sa pagitan ng mga laro.
pre-ot-break-abreviated = PAHINGA BAGO OT
pre-sd-brk = Kung pinahintulutan ang overtime at kinakailangan, ito ang haba ng pahinga sa pagitan ng Ikalawang Kalahati at ng Unang Kalahati ng Overtime
ot-half-len = HABA NG KALAHATI SA OT
time-during-ot = Ang haba ng isang kalahati sa panahon ng overtime
ot-half-tm-len = HABA NG PAHINGA SA OT
len-of-overtime-halftime = Ang haba ng pahinga sa Overtime
pre-sd-break = PAHINGA BAGO SD
pre-sd-len = Ang haba ng pahinga sa pagitan ng nakaraang panahon ng laro at Sudden Death
language = WIKA
this-language = FILIPINO
portal-login-code = CODE
portal-login-instructions = Pumunta sa UWH Portal >> Pamamahala ng Kaganapan >> Pamamahala ng Referee, i-click ang + na pindutan upang magdagdag ng bagong Refbox, at ilagay ang Refbox ID na ito:
    { $id }

    Ang UWH Portal ay magbibigay ng confirmation code na ilalagay sa kaliwa gamit ang number pad.
    Pindutin ang Tapos kapag naipasok mo na ang code

help = TULONG:

# Kumpirmasyon
game-configuration-can-not-be-changed = Hindi mababago ang configuration ng laro habang nagaganap ang laro.

    Ano ang nais mong gawin?
apply-this-game-number-change = Paano mo nais ilapat ang pagbabago ng numero ng larong ito?
UWHPortal-enabled = Kapag pinagana ang UWHPortal, dapat mapunan ang lahat ng field.
uwhportal-token-invalid-code = Maling code ang naipasok.
    Pakisubukan muli.
uwhportal-token-no-pending-link = Hindi inaasahan ng Portal ang isang koneksyon.
    Pakisubukan muli.
go-back-to-editor = BUMALIK SA EDITOR
discard-changes = ITAPON ANG MGA PAGBABAGO
end-current-game-and-apply-changes = TAPUSIN ANG KASALUKUYANG LARO AT ILAPAT ANG MGA PAGBABAGO
end-current-game-and-apply-change = TAPUSIN ANG KASALUKUYANG LARO AT ILAPAT ANG PAGBABAGO
keep-current-game-and-apply-change = PANATILIHIN ANG KASALUKUYANG LARO AT ILAPAT ANG PAGBABAGO
ok = OK
confirm-score = Tama ba ang puntosing ito?
    Kumpirmahin sa punong referee.

    Itim: { $score_black }        Puti: { $score_white }

    { confirmation-count-down }
yes = OO
no = HINDI

# Mga Poul
equal = PANTAY

# Impormasyon ng Laro
refresh = I-REFRESH
refreshing = NIRI-REFRESH...
settings = MGA SETTING
none = Wala
game-number-error = Error ({ $game_number })
next-game-number-error = Error ({ $next_game_number })
last-game-next-game = Huling Laro: { $prev_game },
    Susunod na Laro: { $next_game }
black-team-white-team = Koponan ng Itim: { $black_team }
    Koponan ng Puti: { $white_team }
game-length-ot-allowed = Haba ng Kalahati: { $half_length }
         Haba ng Pahinga: { $half_time_length }
         Overtime Pinahintulutan: { $overtime }
overtime-details = Haba ng Pahinga Bago Overtime: { $pre_overtime }
             Haba ng Kalahati sa Overtime: { $overtime_len }
             Haba ng Pahinga sa Overtime: { $overtime_half_time_len }
sd-allowed = Sudden Death Pinahintulutan: { $sd }
pre-sd = Haba ng Pahinga Bago Sudden Death: { $pre_sd_len }
team-to-len = Tagal ng Timeout ng Koponan: { $to_len }
time-btwn-games = Nominal na Oras sa Pagitan ng mga Laro: { $time_btwn }
min-brk-btwn-games = Pinakamaliit na Oras sa Pagitan ng mga Laro: { $min_brk_time }


# Mga Listahan
select-event = PUMILI NG KAGANAPAN
select-court = PUMILI NG KORTE
select-game = PUMILI NG LARO

# Pangunahing View
add-warning = MAGDAGDAG NG BABALA
add-foul = MAGDAGDAG NG POUL
start-now = MAGSIMULA NA
end-timeout = TAPUSIN ANG TIMEOUT
warnings = MGA BABALA
penalties = MGA PARUSA
dark-score-line-1 = PUNTOS
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = PUNTOS
light-score-line-2 = { light-team-name-caps }

# Mga Parusa
black-penalties = MGA PARUSA NG ITIM
white-penalties = MGA PARUSA NG PUTI

# Pag-edit ng Puntos
final-score = Pakilagay ang panghuling puntos
confirmation-count-down = Tandaan: Ang hindi nabagong puntos ay awtomatikong makukumpirma sa loob ng { $countdown }

# Mga Pinagsamang Elemento
## Timeout ribbon
end-timeout-line-1 = TAPUSIN ANG
end-timeout-line-2 = { timeout }
switch-to = LUMIPAT SA
ref = REF
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = PENALTY
penalty-shot-line-2 = SHOT
pen-shot = PENALTY SHOT
## String ng parusa
served = Naisilbi
penalty = #{$player_number} - {$time ->
        [pending] Naghihintay
        [served] Naisilbi
        [total-dismissal] Tinanggal
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
infraction = Paglabag: {$infraction}
## Config String
error = Error ({ $number })
two-games = Huling Laro: { $prev_game },  Susunod na Laro: { $next_game }
one-game = Laro: { $game }
teams = Koponan ng { -dark-team-name }: { $dark_team }
    Koponan ng { -light-team-name }: { $light_team }
game-config = Haba ng Kalahati: { $half_len },  Haba ng Pahinga: { $half_time_len }
    Sudden Death Pinahintulutan: { $sd_allowed },  Overtime Pinahintulutan: { $ot_allowed }
team-timeouts-per-half = Mga Timeout ng Koponan na Pinahintulutan Bawat Kalahati: { $team_timeouts }
team-timeouts-per-game = Mga Timeout ng Koponan na Pinahintulutan Bawat Laro: { $team_timeouts }
stop-clock-last-2 = Ihinto ang Orasan sa Huling 2 Minuto: { $stop_clock }
ref-list = Punong Ref: { $chief_ref }
    Timer: { $timer }
    Water Ref 1: { $water_ref_1 }
    Water Ref 2: { $water_ref_2 }
    Water Ref 3: { $water_ref_3 }
team-ref-list = Mga Referee: { $ref_team }
    Tagapanatili ng Puntos: { $ts_keeper_team }
unknown = Hindi Alam
## Pindutan ng oras ng laro
next-game = SUSUNOD NA LARO
first-half = UNANG KALAHATI
half-time = PAHINGA
second-half = IKALAWANG KALAHATI
pre-ot-break-full = PAHINGA BAGO OVERTIME
overtime-first-half = UNANG KALAHATI NG OT
overtime-half-time = PAHINGA SA OVERTIME
overtime-second-half = IKALAWANG KALAHATI NG OT
pre-sudden-death-break = PAHINGA BAGO SUDDEN DEATH
sudden-death = SUDDEN DEATH
ot-first-half = UNANG KALAHATI NG OT
ot-half-time = PAHINGA SA OT
ot-2nd-half = IKALAWANG KALAHATI NG OT
white-timeout-short = PUTI T/O
white-timeout-full = TIMEOUT NG PUTI
black-timeout-short = ITIM T/O
black-timeout-full = TIMEOUT NG ITIM
ref-timeout-short = REF T/O
penalty-shot-short = PENALTY SHT
## Gawing warning container
team-warning-abreviation = K
## Gawing time editor
zero = SERO

# Pag-edit ng Oras
game-time = ORAS NG LARO
timeout = TIMEOUT
Note-Game-time-is-paused = Tandaan: Ang oras ng laro ay nakaka-pause habang nasa screen na ito

# Buod ng Babala at Poul
fouls = MGA POUL
edit-warnings = I-EDIT ANG MGA BABALA
edit-fouls = I-EDIT ANG MGA POUL

# Mga Babala
black-warnings = MGA BABALA NG ITIM
white-warnings = MGA BABALA NG PUTI

# Mensahe
player-number = NUMERO NG
    MANLALARO:
game-number = NUMERO NG
    LARO:
num-tos-per-half = BILANG NG TEAM
    T/Os BAWAT KALAHATI:
num-tos-per-game = BILANG NG TEAM
    T/Os BAWAT LARO:

# Sound Controller - mod
off = PATAY
low = MABABA
medium = KATAMTAMAN
high = MATAAS
max = MAX

# Config
hockey6v6 = HOCKEY6V6
hockey3v3 = HOCKEY3V3
rugby = RUGBY

# Mga Paglabag
stick-foul = Poul sa Stick
illegal-advance = Iligal na Pag-abante
sub-foul = Poul sa Pagpapalit
illegal-stoppage = Iligal na Paghinto
out-of-bounds = Labas ng Hangganan
grabbing-the-wall = Pagkapit sa Dingding
obstruction = Hadlang
delay-of-game = Pagkaantala ng Laro
unsportsmanlike = Hindi Sportsmanlike
free-arm = Libreng Braso
false-start = Maling Simula


# Portal Health Indicator
# NOTE: Awaiting native-speaker translation; English placeholders for now.
portal-summary-title = UWH PORTAL STATUS
portal-row-token-expired = Portal login expired — tap to re-login
portal-row-stuck = Game { $game } Score send error, tap to fix
portal-row-pending = Game { $game } Score not sent, tap to retry
portal-row-recent = Game { $game } · Submitted { $mins } min ago
portal-action-force-submit = Retry this game result
portal-action-discard = Discard this game result
portal-action-discard-confirm = TAP AGAIN TO CONFIRM DISCARD
portal-action-go-to-login = GO TO LOGIN
portal-page-title-attention = Game { $game } score submission error
portal-page-title-token-expired = Portal login expired
portal-page-body-token-expired = The UWH Portal login has expired. Queued scores cannot be sent until you log in again. Tap GO TO LOGIN to re-authenticate.
portal-page-attention-info = Likely due to a connection error, the game result has not been accepted on UWH Portal
portal-page-attention-score = Stored game result: Light { $white } - Dark { $black }
portal-advisory-at-game-end = Portal issue detected. Score will still be queued — find an admin to resolve.
