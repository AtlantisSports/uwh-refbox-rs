# Definitions for the translation file to use
-dark-team-name = Zwart
dark-team-name-caps = ZWART

-light-team-name = Wit
light-team-name-caps = WIT

# Multipage
done = KLAAR
restart-to-apply = OPNIEUW STARTEN
cancel = ANNULEREN
delete = VERWIJDEREN
back = TERUG
new = NIEUW

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
timeout-length = DUUR
    TIME-OUT

# Warning Add
team-warning = TEAM-
    WAARSCHUWING
team-warning-line-1 = TEAM-
team-warning-line-2 = WAARSCHUWING

# Configuration
none-selected = Niets Geselecteerd
loading = Laden...
game-select = Wedstrijd:
game-options = WEDSTRIJDOPTIES
app-options = APP-OPTIES
display-options = WEERGAVEOPTIES
sound-options = GELUIDSOPTIES
app-mode = APP-
    MODUS
hide-time-for-last-15-seconds = TIJD VERBERGEN
    LAATSTE 15 SEK
player-display-brightness = HELDERHEID
    SPELERSSCHERM
confirm-score-at-game-end = SCORE BEVESTIGEN
    BIJ WEDSTRIJDEINDE
track-cap-number-of-scorer = MUTSSNUMMER
    DOELPUNTENMAKER
event = EVENEMENT:
track-fouls-and-warnings = OVERTREDINGEN
    EN WAARSCHUWINGEN
court = BAAN:
single-half = ENKELE
    HELFT:
half-length-full = DUUR HELFT:
game-length = WEDSTRIJDDUUR:
overtime-allowed = VERLENGING
    TOEGESTAAN:
sudden-death-allowed = SUDDEN DEATH
    TOEGESTAAN:
half-time-length = DUUR
    RUST:
pre-ot-break-length = PAUZE VOOR
    VERLENGING:
pre-sd-break-length = PAUZE VOOR
    SUDDEN DEATH:
nominal-break-between-games = NOMINALE PAUZE
    TUSSEN WEDSTR.:
ot-half-length = DUUR HELFT
    VERLENGING:
timeouts-counted-per = TIME-OUTS
    GETELD PER:
game = WEDSTRIJD
half = HELFT
minimum-brk-btwn-games = MIN PAUZE
    TUSSEN WEDSTR.:
ot-half-time-length = RUST
    VERLENGING
using-uwh-portal = UWHPORTAL GEBRUIKEN:
starting-sides = STARTZIJDEN
sound-enabled = GELUID
    INGESCHAKELD:
whistle-volume = FLUIT-
    VOLUME:
manage-remotes = AFSTANDSBEDIENINGEN BEHEREN
whistle-enabled = FLUIT
    INGESCHAKELD:
above-water-volume = VOLUME
    BOVEN WATER:
auto-sound-start-play = AUTO GELUID
    SPEL STARTEN:
buzzer-sound = ZOEMER-
    GELUID:
underwater-volume = VOLUME
    ONDER WATER:
auto-sound-stop-play = AUTO GELUID
    SPEL STOPPEN:
alarm-button = ALARM-
    KNOP:
alarm = ALARM
hold-to-test = INGEDRUKT HOUDEN OM TE TESTEN
or-press-spacebar = Of Druk op Spatiebalk
or-hold-spacebar = Of Houd Spatiebalk Ingedrukt
game-info = WEDSTRIJDINFO
remotes = AFSTANDSBEDIENINGEN
default = STANDAARD
sound = GELUID: { $sound_text }
brightness = { $brightness ->
        *[Low] LAAG
        [Medium] GEMIDDELD
        [High] HOOG
        [Outdoor] BUITEN
    }

waiting = WACHTEN
add = TOEVOEGEN
half-length = DUUR HELFT
length-of-half-during-regular-play = De duur van een helft tijdens regulier spel
half-time-lenght = DUUR RUST
length-of-half-time-period = De duur van de rustperiode
nom-break = NOM PAUZE
system-will-keep-game-times-spaced = Het systeem probeert de starttijden van wedstrijden gelijkmatig te verdelen. De totale tijd van de ene start tot de volgende is 2 × [Duur Helft] + [Duur Rust] + [Nominale Tijd Tussen Wedstrijden] (voorbeeld: als [Duur Helft] = 15 min, [Duur Rust] = 3 min en [Nominale Tijd] = 12 min, dan is de tijd van start tot start 45 min. Genomen time-outs of andere onderbrekingen verkorten de 12 min tot de minimale tijd tussen wedstrijden is bereikt).
min-break = MIN PAUZE
min-time-btwn-games = Als een wedstrijd langer duurt dan gepland, is dit de minimale tijd tussen wedstrijden die het systeem toestaat. Als wedstrijden uitlopen, probeert het systeem bij volgende wedstrijden in te halen, waarbij altijd deze minimale tijd wordt gerespecteerd.
pre-ot-break-abreviated = PAUZE VOOR VERLENGING
pre-sd-brk = Als verlenging is ingeschakeld en nodig is, is dit de duur van de pauze tussen de tweede helft en de eerste helft van de verlenging
ot-half-len = DUUR HELFT VERL
time-during-ot = De duur van een helft tijdens de verlenging
ot-half-tm-len = RUST VERL DUUR
len-of-overtime-halftime = De duur van de rust tijdens de verlenging
pre-sd-break = PAUZE VOOR SD
pre-sd-len = De duur van de pauze tussen het voorgaande speelgedeelte en Sudden Death
language = TAAL
this-language = NEDERLANDS
portal-login-code = CODE
portal-login-instructions = Ga naar het UWH-portaal >> Evenementbeheer >> Scheidsrechterbeheer, klik op de + knop om een nieuwe Refbox toe te voegen en voer dit Refbox-ID in:
    { $id }

    Het UWH-portaal geeft vervolgens een bevestigingscode die u links via het nummerveld kunt invoeren.
    Druk op Klaar nadat u de code hebt ingevoerd

help = HELP:

# Confirmation
game-configuration-can-not-be-changed = De wedstrijdconfiguratie kan niet worden gewijzigd terwijl een wedstrijd bezig is.

    Wat wilt u doen?
apply-this-game-number-change = Hoe wilt u deze wijziging van het wedstrijdnummer toepassen?
UWHPortal-enabled = Als UWHPortal is ingeschakeld, moeten alle velden worden ingevuld.
uwhportal-token-invalid-code = Ongeldige code ingevoerd.
    Probeer het opnieuw.
uwhportal-token-no-pending-link = Portaal verwacht geen verbinding.
    Probeer het opnieuw.
go-back-to-editor = TERUG NAAR EDITOR
discard-changes = WIJZIGINGEN VERWERPEN
end-current-game-and-apply-changes = HUIDIGE WEDSTRIJD BEËINDIGEN EN WIJZIGINGEN TOEPASSEN
end-current-game-and-apply-change = HUIDIGE WEDSTRIJD BEËINDIGEN EN WIJZIGING TOEPASSEN
keep-current-game-and-apply-change = HUIDIGE WEDSTRIJD BEWAREN EN WIJZIGING TOEPASSEN
ok = OK
confirm-score = Is deze score correct?
    Bevestig met de hoofdscheidsrechter.

    Zwart: { $score_black }        Wit: { $score_white }

    { confirmation-count-down }
yes = JA
no = NEE

# Fouls
equal = GELIJK

# Game Info
refresh = VERNIEUWEN
refreshing = VERNIEUWEN...
settings = INSTELLINGEN
none = Geen
game-number-error = Fout ({ $game_number })
next-game-number-error = Fout ({ $next_game_number })
last-game-next-game = Vorige Wedstrijd: { $prev_game },
    Volgende Wedstrijd: { $next_game }
black-team-white-team = Zwart Team: { $black_team }
    Wit Team: { $white_team }
game-length-ot-allowed = Duur Helft: { $half_length }
         Duur Rust: { $half_time_length }
         Verlenging Toegestaan: { $overtime }
overtime-details = Duur Pauze Voor Verlenging: { $pre_overtime }
             Duur Helft Verlenging: { $overtime_len }
             Duur Rust Verlenging: { $overtime_half_time_len }
sd-allowed = Sudden Death Toegestaan: { $sd }
pre-sd = Duur Pauze Voor Sudden Death: { $pre_sd_len }
team-to-len = Duur Team Time-out: { $to_len }
time-btwn-games = Nominale Tijd Tussen Wedstrijden: { $time_btwn }
min-brk-btwn-games = Minimale Tijd Tussen Wedstrijden: { $min_brk_time }


# List Selecters
select-event = EVENEMENT SELECTEREN
select-court = BAAN SELECTEREN
select-game = WEDSTRIJD SELECTEREN

# Main View
add-warning = WAARSCHUWING TOEVOEGEN
add-foul = OVERTREDING TOEVOEGEN
start-now = NU STARTEN
end-timeout = TIME-OUT BEËINDIGEN
warnings = WAARSCHUWINGEN
penalties = STRAFFEN
dark-score-line-1 = SCORE
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = SCORE
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = STRAFFEN ZWART
white-penalties = STRAFFEN WIT

# Score edit
final-score = Voer de eindstand in
confirmation-count-down = Opmerking: De ongewijzigde score wordt automatisch bevestigd over { $countdown }

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = TIME-OUT
end-timeout-line-2 = BEËINDIGEN
switch-to = OVERSCHAKELEN NAAR
ref = SCHEIDSRECHTER
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = STRAF-
penalty-shot-line-2 = SCHOP
pen-shot = STRAFSCHOP
## Penalty string
served = Uitgezeten
penalty = #{$player_number} - {$time ->
        [pending] In afwachting
        [served] Uitgezeten
        [total-dismissal] Uitgesloten
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
infraction = Overtreding: {$infraction}
## Config String
error = Fout ({ $number })
two-games = Vorige Wedstrijd: { $prev_game },  Volgende Wedstrijd: { $next_game }
one-game = Wedstrijd: { $game }
teams = { -dark-team-name } Team: { $dark_team }
    { -light-team-name } Team: { $light_team }
game-config = Duur Helft: { $half_len },  Duur Rust: { $half_time_len }
    Sudden Death Toegestaan: { $sd_allowed },  Verlenging Toegestaan: { $ot_allowed }
team-timeouts-per-half = Team Time-outs Toegestaan Per Helft: { $team_timeouts }
team-timeouts-per-game = Team Time-outs Toegestaan Per Wedstrijd: { $team_timeouts }
stop-clock-last-2 = Klok Stoppen in Laatste 2 Minuten: { $stop_clock }
ref-list = Hoofdscheidsrechter: { $chief_ref }
    Tijdwaarnemer: { $timer }
    Waterscheidsrechter 1: { $water_ref_1 }
    Waterscheidsrechter 2: { $water_ref_2 }
    Waterscheidsrechter 3: { $water_ref_3 }
team-ref-list = Scheidsrechters: { $ref_team }
    Tijdwaarnemer/Scorer: { $ts_keeper_team }
unknown = Onbekend
## Game time button
next-game = VOLGENDE WEDSTRIJD
first-half = EERSTE HELFT
half-time = RUST
second-half = TWEEDE HELFT
pre-ot-break-full = PAUZE VOOR VERLENGING
overtime-first-half = VERLENGING EERSTE HELFT
overtime-half-time = RUST VERLENGING
overtime-second-half = VERLENGING TWEEDE HELFT
pre-sudden-death-break = PAUZE VOOR SUDDEN DEATH
sudden-death = SUDDEN DEATH
ot-first-half = VERL EERSTE HELFT
ot-half-time = VERL RUST
ot-2nd-half = VERL TWEEDE HELFT
white-timeout-short = WIT T/O
white-timeout-full = TIME-OUT WIT
black-timeout-short = ZWA T/O
black-timeout-full = TIME-OUT ZWART
ref-timeout-short = SCHEIDSRT/O
penalty-shot-short = STRAFSCHOP
## Make warning container
team-warning-abreviation = T
## Make time editor
zero = NUL

# Time edit
game-time = WEDSTRIJDTIJD
timeout = TIME-OUT
Note-Game-time-is-paused = Opmerking: De wedstrijdtijd is gepauzeerd op dit scherm

# Warning Fouls Summary
fouls = OVERTREDINGEN
edit-warnings = WAARSCHUWINGEN BEWERKEN
edit-fouls = OVERTREDINGEN BEWERKEN

# Warnings
black-warnings = WAARSCHUWINGEN ZWART
white-warnings = WAARSCHUWINGEN WIT

# Message
player-number = SPELER-
    NUMMER:
game-number = WEDSTRIJD-
    NUMMER:
num-tos-per-half = AANTAL T/O'S
    PER HELFT:
num-tos-per-game = AANTAL T/O'S
    PER WEDSTRIJD:

# Sound Controller - mod
off = UIT
low = LAAG
medium = GEMIDDELD
high = HOOG
max = MAX

# Config
hockey6v6 = HOCKEY 6T6
hockey3v3 = HOCKEY 3T3
rugby = RUGBY

# Infractions
stick-foul = Stokovertreding
illegal-advance = Ongeoorloofde Aanval
sub-foul = Wisselovertreding
illegal-stoppage = Ongeoorloofde Stop
out-of-bounds = Buiten het Veld
grabbing-the-wall = Muur Vastgrijpen
obstruction = Obstructie
delay-of-game = Vertraging van Spel
unsportsmanlike = Onsportief Gedrag
free-arm = Vrije Arm
false-start = Valse Start
