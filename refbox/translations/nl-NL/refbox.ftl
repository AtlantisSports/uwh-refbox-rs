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
save = OPSLAAN
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
game-select = WEDSTRIJD:
game-options = WEDSTRIJDOPTIES
app-options = APP-OPTIES
display-options = WEERGAVEOPTIES
open-new-display = NIEUWE WEERGAVE OPENEN
front-display-layout = DISPLAY LAYOUT
layout-default = DEFAULT
layout-classic = CLASSIC
layout-big-time = BIG TIME
layout-corners = CORNERS
layout-scores-only = SCORES ONLY
sound-options = GELUIDSOPTIES
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
sound-settings = GELUIDSINSTELLINGEN
beep-test-edit-levels = NIVEAUS BEWERKEN
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
show-behind-schedule-time = ACHTERSTAND TONEN
delay = VERTRAGING
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
game-block = SPELBLOK
game-block-full = SPELBLOK:
game-block-help = Tijd van het begin van één wedstrijd tot het begin van de volgende
game-block-too-short = Te kort voor de wedstrijd plus de minimale pauze
game-block-tight = Krap — teamtime-outs kunnen wedstrijden buiten hun tijdslot duwen
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
game-block-info = Spelblok: { $game_block }
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
cancel-timeout = { cancel } { timeout }
cancel-timeout-line-1 = { cancel }
cancel-timeout-line-2 = { timeout }
cancel-ref-timeout = { cancel } { ref } { timeout }
cancel-ref-timeout-line-1 = { cancel } { ref }
cancel-ref-timeout-line-2 = { timeout }
cancel-pen-shot = { cancel } { pen-shot }
cancel-pen-shot-line-1 = { cancel }
cancel-pen-shot-line-2 = { pen-shot }
switch-to = OVERSCHAKELEN NAAR
ref = SCHEIDSRECHTER
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
revive-hold-line-1 = HOUD VAST OM
revive-hold-line-2 = HERSTELLEN
revive-deciding-line-2 = HERSTELD
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
team-timeouts = Team Time-outs: { $value }
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
beep-test = PIEPTEST

# Beep-test screen
beep-test-pre = PRE
beep-test-top-time-label = TIJD
beep-test-top-level-label = NIVEAU
beep-test-top-lap-label = RONDE
beep-test-start = START
beep-test-pause = PAUZE
beep-test-resume = HERVATTEN
beep-test-reset = RESET
beep-test-column-level = NIVEAU
beep-test-column-count = AANTAL
beep-test-column-duration = DUUR
beep-test-edit-selected = Niveau { $level }
beep-test-edit-time = TIJD
beep-test-edit-count = AANTAL
beep-test-edit-new = NIVEAU TOEVOEGEN
beep-test-edit-remove = NIVEAU VERWIJDEREN

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
portal-summary-title = { $portal } PORTAALSTATUS
portal-row-token-expired = Portaal-login verlopen — tik om opnieuw in te loggen
portal-row-stuck = Wedstrijd { $game } Fout bij verzenden score, tik om te herstellen
portal-row-pending = Wedstrijd { $game } Score niet verzonden, tik om opnieuw te proberen
portal-row-recent = Wedstrijd { $game } · Verzonden { $mins } min geleden
portal-row-attempt-suffix = (poging { $attempts })
portal-action-force-submit = Deze wedstrijduitslag opnieuw proberen
portal-action-discard = Deze wedstrijduitslag verwerpen
portal-action-discard-confirm = TIK NOGMAALS OM VERWERPEN TE BEVESTIGEN
portal-page-title-attention = Wedstrijd { $game } verzendfout
portal-page-attention-info = De wedstrijduitslag is niet geaccepteerd op { $portal } Portaal
portal-page-attention-score = Opgeslagen wedstrijduitslag: Licht { $white } - Donker { $black }
portal-page-attention-remediation = U kunt Opnieuw proberen als de verbinding is geverifieerd, of verwerpen om de fout te wissen
portal-advisory-at-game-end = Portaalprobleem gedetecteerd. Score blijft in wachtrij — zoek een beheerder om het op te lossen.

# 2 Halves / 1 Period selector (Half Length editor)
two-halves = 2 HELFTEN
one-period = 1 PERIODE
game-len = WEDSTRIJDDUUR
length-of-game-during-regular-play = De duur van de wedstrijd tijdens regulier spel

# Single-period (1 Period) info variants — Game Length, no half-time line
game-config-single-half = Wedstrijdduur: { $half_len }
    Plotselinge Dood Toegestaan: { $sd_allowed },  Verlenging Toegestaan: { $ot_allowed }
game-length-ot-allowed-single-half = Wedstrijdduur: { $half_length }
         Verlenging Toegestaan: { $overtime }

# Self-update / Updates page
check-version = Versie controleren
updates-current-version = Huidige versie
updates-check-for-updates = Controleren op updates
updates-install = Installeren
updates-do-revert = Terugzetten
updates-install-note = Klikken op Installeren downloadt en installeert de update en start de refbox opnieuw op
updates-revert-note = Klikken op Terugzetten herstelt de vorige versie en start de refbox opnieuw op
updates-unknown = Onbekend
updates-checking = Bezig met controleren…
updates-up-to-date = Up-to-date.
updates-available = Update beschikbaar: {$version}
updates-downloading = Bezig met downloaden…
updates-verifying = Download controleren…
updates-installing = Bezig met installeren…
updates-restarting = Bezig met opnieuw opstarten…
updates-confirm-revert = Terug naar de vorige versie ({$version})?
updates-rolled-back = Teruggezet naar de vorige versie omdat de update niet correct is gestart, probeer het opnieuw.
updates-revert = Terug naar vorige versie ({$version})
updates-error-no-internet = Kon de updateserver niet bereiken, controleer je internetverbinding
updates-error-bad-download = De gedownloade update was niet geldig en is niet geïnstalleerd.
updates-error-rate-limited = De updateserver is bezig, probeer het over een tijdje opnieuw.
updates-error-no-space = Niet genoeg vrije ruimte om de update te installeren.
updates-error-not-writable = De update kon niet worden opgeslagen (toegang geweigerd).
