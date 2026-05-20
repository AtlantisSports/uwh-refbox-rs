# Definities voor het gebruik in het vertaalbestand
-dark-team-name = Donker
dark-team-name-caps = DONKER

-light-team-name = Licht
light-team-name-caps = LICHT

# Meerdere pagina's
done = KLAAR
restart-to-apply = OPNIEUW STARTEN OM TOE TE PASSEN
cancel = ANNULEREN
delete = VERWIJDEREN
back = TERUG
apply = TOEPASSEN
save = SAVE
user-options = GEBRUIKERSOPTIES
new = NIEUW

# Uitsluiting bewerken
total-dismissal = DEF
penalty-kind = {$kind ->
    [thirty-seconds] 30s
    [one-minute] 1m
    [two-minutes] 2m
    [four-minutes] 4m
    [five-minutes] 5m
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# Team time-out bewerken
timeout-length = TEAM TIME-OUT
    DUUR

# Waarschuwing toevoegen
team-warning = TEAM-
    WAARSCHUWING
team-warning-line-1 = TEAM-
team-warning-line-2 = WAARSCHUWING

# Configuratie
none-selected = Niets Geselecteerd
loading = Laden...
game-select = Wedstrijd:
game-options = WEDSTRIJDOPTIES
app-options = APP-OPTIES
display-options = WEERGAVEOPTIES
open-new-display = NIEUWE WEERGAVE OPENEN
sound-options = GELUIDSOPTIES
sound-settings = SOUND SETTINGS
beep-test-edit-levels = EDIT LEVELS
app-mode = APP-
    MODUS
hide-time-for-last-15-seconds = TIJD VERBERGEN
    LAATSTE 15 SEK
player-display-brightness = HELDERHEID
    SPELERSSCHERM
confirm-score-at-game-end = SCORE BEVESTIGEN
    BIJ WEDSTRIJDEINDE
track-cap-number-of-scorer = MUTSNUMMER
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
sudden-death-allowed = PLOTSELINGE DOOD
    TOEGESTAAN:
half-time-length = DUUR
    RUST:
pre-ot-break-length = PAUZE VOOR
    VERLENGING:
pre-sd-break-length = PAUZE VOOR
    PLTS DOOD:
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
using-portal = { $portal }PORTAL GEBRUIKEN:
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
pre-sd-break = PAUZE VOOR PLTS DOOD
pre-sd-len = De duur van de pauze tussen het voorgaande speelgedeelte en Plotselinge Dood
language = TAAL
this-language = NEDERLANDS
portal-login-code = CODE
portal-login-instructions = Ga naar het { $portal }-portaal >> Evenementbeheer >> Scheidsrechterbeheer, klik op de + knop om een nieuwe Refbox toe te voegen en voer dit Refbox-ID in:
    { $id }

    Het { $portal }-portaal geeft vervolgens een bevestigingscode die u links via het nummerveld kunt invoeren.
    Druk op Klaar nadat u de code hebt ingevoerd

help = HELP:

# Bevestiging
game-configuration-can-not-be-changed = De wedstrijdconfiguratie kan niet worden gewijzigd terwijl een wedstrijd bezig is.

    Wat wilt u doen?
apply-this-game-number-change = Hoe wilt u deze wijziging van het wedstrijdnummer toepassen?
portal-enabled = Als { $portal }PORTAL is ingeschakeld, moeten alle velden worden ingevuld.
mode-switch-portal-tenant = Het wijzigen van de modus van { $from_mode } naar { $to_mode } verbreekt de verbinding met { $from_portal }PORTAL en u moet opnieuw verbinding maken met { $to_portal }PORTAL.
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

    Donker: { $score_black }        Licht: { $score_white }

    { confirmation-count-down }
yes = JA
no = NEE

# Overtredingen
equal = GELIJK

# Wedstrijdinfo
refresh = VERNIEUWEN
refreshing = VERNIEUWEN...
settings = INSTELLINGEN
none = Geen
game-number-error = Fout ({ $game_number })
next-game-number-error = Fout ({ $next_game_number })
last-game-next-game = Vorige Wedstrijd: { $prev_game },
    Volgende Wedstrijd: { $next_game }
black-team-white-team = Donker Team: { $black_team }
    Licht Team: { $white_team }
game-length-ot-allowed = Duur Helft: { $half_length }
         Duur Rust: { $half_time_length }
         Verlenging Toegestaan: { $overtime }
overtime-details = Duur Pauze Voor Verlenging: { $pre_overtime }
             Duur Helft Verlenging: { $overtime_len }
             Duur Rust Verlenging: { $overtime_half_time_len }
sd-allowed = Plotselinge Dood Toegestaan: { $sd }
pre-sd = Duur Pauze Voor Plotselinge Dood: { $pre_sd_len }
team-to-len = Duur Team Time-out: { $to_len }
time-btwn-games = Nominale Tijd Tussen Wedstrijden: { $time_btwn }
min-brk-btwn-games = Minimale Tijd Tussen Wedstrijden: { $min_brk_time }


# Lijstselecties
select-event = EVENEMENT SELECTEREN
select-court = BAAN SELECTEREN
select-game = WEDSTRIJD SELECTEREN

# Hoofdweergave
add-warning = WAARSCHUWING TOEVOEGEN
add-foul = OVERTREDING TOEVOEGEN
start-now = NU STARTEN
end-timeout = TIME-OUT BEËINDIGEN
warnings = WAARSCHUWINGEN
penalties = UITSLUITINGEN
dark-score-line-1 = SCORE
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = SCORE
light-score-line-2 = { light-team-name-caps }

# Uitsluitingen
black-penalties = UITSLUITINGEN DONKER
white-penalties = UITSLUITINGEN LICHT

# Score bewerken
final-score = Voer de eindstand in
confirmation-count-down = Opmerking: De ongewijzigde score wordt automatisch bevestigd over { $countdown }

# Gedeelde elementen
## Time-out lint
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
penalty-shot-line-2 = SCHOT
pen-shot = STRAFSCHOT
## Uitsluitingsreeks
served = Uitgezeten
penalty = #{$player_number} - {$time ->
        [pending] In behandeling
        [served] Uitgezeten
        [total-dismissal] Definitief uitgesloten
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
## Configuratiereeks
error = Fout ({ $number })
two-games = Vorige Wedstrijd: { $prev_game },  Volgende Wedstrijd: { $next_game }
one-game = Wedstrijd: { $game }
teams = { -dark-team-name } Team: { $dark_team }
    { -light-team-name } Team: { $light_team }
game-config = Duur Helft: { $half_len },  Duur Rust: { $half_time_len }
    Plotselinge Dood Toegestaan: { $sd_allowed },  Verlenging Toegestaan: { $ot_allowed }
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
## Wedstrijdtijdknop
next-game = VOLGENDE WEDSTRIJD
first-half = EERSTE HELFT
half-time = RUST
second-half = TWEEDE HELFT
pre-ot-break-full = PAUZE VOOR VERLENGING
overtime-first-half = EERSTE VERLENGINGSTIJD
overtime-half-time = RUST VERLENGING
overtime-second-half = TWEEDE VERLENGINGSTIJD
pre-sudden-death-break = PAUZE VOOR PLOTSELINGE DOOD
sudden-death = PLOTSELINGE DOOD
ot-first-half = VERL EERSTE HELFT
ot-half-time = VERL RUST
ot-2nd-half = VERL TWEEDE HELFT
white-timeout-short = LCH T/O
white-timeout-full = TEAM TIME-OUT LICHT
black-timeout-short = DNK T/O
black-timeout-full = TEAM TIME-OUT DONKER
ref-timeout-short = SR T/O
penalty-shot-short = STRAFSCHOT
## Waarschuwingscontainer maken
team-warning-abreviation = T
## Tijdeditor maken
zero = = 0

# Tijd bewerken
game-time = WEDSTRIJDTIJD
timeout = TIME-OUT
Note-Game-time-is-paused = Opmerking: De wedstrijdtijd is gepauzeerd op dit scherm

# Samenvatting overtredingen en waarschuwingen
fouls = OVERTREDINGEN
edit-warnings = WAARSCHUWINGEN BEWERKEN
edit-fouls = OVERTREDINGEN BEWERKEN

# Waarschuwingen
black-warnings = WAARSCHUWINGEN DONKER
white-warnings = WAARSCHUWINGEN LICHT

# Bericht
player-number = SPELER-
    NUMMER:
game-number = WEDSTRIJD-
    NUMMER:
num-tos-per-half = AANTAL T/O'S
    PER HELFT:
num-tos-per-game = AANTAL T/O'S
    PER WEDSTRIJD:

# Geluidsregelaar - modus
off = UIT
low = LAAG
medium = GEMIDDELD
high = HOOG
max = MAX

# Configuratie
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
beep-test-stop = STOP
beep-test-reset = RESET
beep-test-column-level = LEVEL
beep-test-column-count = COUNT
beep-test-column-duration = DURATION
beep-test-edit-selected = Selected: Level { $level }
beep-test-edit-time = TIME
beep-test-edit-count = COUNT
beep-test-edit-new = + NEW
beep-test-edit-remove = REMOVE LEVEL

# Overtredingen
stick-foul = Stokfout
illegal-advance = Onrechtmatig Voortbewegen
sub-foul = Wisselfout
illegal-stoppage = Onrechtmatige Onderbreking
out-of-bounds = Buiten het Speelveld
grabbing-the-wall = Vasthouden aan de Wand
obstruction = Obstructie
delay-of-game = Vertraging van het Spel
unsportsmanlike = Onsportief Gedrag
free-arm = Vrije Arm
false-start = Vals Vertrek


# Portal Health Indicator
# NOTE: Awaiting native-speaker translation; English placeholders for now.
portal-summary-title = { $portal } PORTAL STATUS
portal-row-token-expired = Portal login expired — tap to re-login
portal-row-stuck = Game { $game } Score send error, tap to fix
portal-row-pending = Game { $game } Score not sent, tap to retry
portal-row-recent = Game { $game } · Submitted { $mins } min ago
portal-action-force-submit = Retry this game result
portal-action-discard = Discard this game result
portal-action-discard-confirm = TAP AGAIN TO CONFIRM DISCARD
portal-page-title-attention = Game { $game } submission error
portal-page-attention-info = The game result has not been accepted on { $portal } Portal
portal-page-attention-score = Stored game result: Light { $white } - Dark { $black }
portal-page-attention-remediation = You can Retry if connection is verified, or discard to clear the error
portal-advisory-at-game-end = Portal issue detected. Score will still be queued — find an admin to resolve.
