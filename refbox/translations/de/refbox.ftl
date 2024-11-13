# Definitions for the translation file to use
-dark-team-name = Schwarz
dark-team-name-caps = SCHWARZ
-light-team-name = Weiß
light-team-name-caps = WEISS

# Multipage 
done = FERTIG
cancel = ABBRECHEN
delete = LÖSCHEN
back = ZURÜCK
new = NEU

# Penalty Edit
total-dismissial = TD

# Team Timeout Edit
timeout-length = AUSZEITDAUER

# Warning Add
team-warning = TEAM
    WARNUNG
team-warning-line-1 = TEAM
team-warning-line-2 = WARNUNG

# Configuration
none-selected = Keine Auswahl
loading = Laden...
game = Spiel:
tournament-options = TURNIEROPTIONEN
app-options = APP-OPTIONEN
display-options = ANZEIGEOPTIONEN
sound-options = SOUNDOPTIONEN
app-mode = APP
    MODUS
hide-time-for-last-15-seconds = ZEIT VERBERGEN FÜR
    DIE LETZTEN 15 SEKUNDEN
track-cap-number-of-scorer = KAPAZITÄTSNUMMER
    DER TORSCHÜTZEN VERFOLGEN
track-fouls-and-warnings = FOULS UND
    WARNUNGEN VERFOLGEN
tournament = TURNIER:
court = FELD:
half-length-full = HALBZEITDAUER:
overtime-allowed = VERLÄNGERUNG 
    ERLAUBT:
sudden-death-allowed = SUDDEN DEATH 
    ERLAUBT:
half-time-length = HALBZEITPAUSE
    DAUER:
pre-ot-break-length = VOR OT
    PAUSE DAUER:
pre-sd-break-length = VOR SD
    PAUSE DAUER:
nominal-break-between-games = NOMINALE PAUSE
    ZWISCHEN SPIELEN:
ot-half-length = OT HALBZEIT
    DAUER:
num-team-tos-allowed-per-half = ANZAHL TEAM AUSZEITEN
    ERLAUBT PRO HALBZEIT:
minimum-brk-btwn-games = MINIMALE PAUSE
    ZWISCHEN SPIELEN:
ot-half-time-length = OT HALBZEIT
    PAUSE DAUER
using-uwh-portal = UWHPORTAL VERWENDEN:
starting-sides = STARTSEITEN 
sound-enabled = SOUND
    AKTIVIERT:
whistle-volume = PFEIFE
    LAUTSTÄRKE:
manage-remotes = FERNBEDIENUNGEN VERWALTEN
whistle-enabled = PFEIFE 
    AKTIVIERT:
above-water-volume = ÜBER WASSER
    LAUTSTÄRKE:
auto-sound-start-play = AUTO SOUND
    STARTSPIEL:
buzzer-sound = SUMMERTON 
    SOUND:
underwater-volume = UNTERWASSER
    LAUTSTÄRKE:
auto-sound-stop-play = AUTO SOUND
    STOPP SPIEL:
remotes = FERNBEDIENUNGEN
default = STANDARD
sound = SOUND: { $sound_text }

waiting = WARTEN
add = HINZUFÜGEN
half-length = HALBZEITDAUER
length-of-half-during-regular-play = Die Länge einer Halbzeit während des regulären Spiels
half-time-lenght = HALBZEITPAUSE
length-of-half-time-period = Die Länge der Halbzeitpause
nom-break = NOMINALE PAUSE
system-will-keep-game-times-spaced = Das System versucht, die Spielzeiten gleichmäßig zu verteilen, wobei die
    Gesamtzeit von einem Start zum nächsten 2 * [Halbzeitdauer] + [Halbzeitpause
    Dauer] + [Nominale Pause Zwischen Spielen] beträgt (Beispiel: Wenn Spiele eine [Halbzeitdauer]
    = 15m, [Halbzeitpause Dauer] = 3m und [Nominale Pause Zwischen Spielen] =
    12m haben, beträgt die Zeit vom Start eines Spiels bis zum nächsten 45m. Jede
    genommene Auszeit oder andere Uhrenstopps verringern die 12m Zeit, bis
    der Mindestzeit zwischen den Spielen erreicht ist).
min-break = MINIMALE PAUSE
min-time-btwn-games = Wenn ein Spiel länger als geplant dauert, ist dies die minimale Zeit zwischen
            den Spielen, die das System zuteilt. Wenn die Spiele in Rückstand geraten, wird das System
            automatisch versuchen, nachfolgenden Spielen aufzuholen, wobei diese Mindestzeit
            zwischen den Spielen immer respektiert wird.
pre-ot-break-abreviated = VOR OT PAUSE
pre-sd-brk = Wenn eine Verlängerung aktiviert und benötigt wird, ist dies die Länge der Pause zwischen
            der zweiten Halbzeit und der ersten Halbzeit der Verlängerung
ot-half-len = OT HALBZEIT
time-during-ot = Die Länge einer Halbzeit während der Verlängerung
ot-half-tm-len = OT HALBZEIT
len-of-overtime-halftime = Die Länge der Verlängerung Halbzeit
pre-sd-break = VOR SD PAUSE
pre-sd-len = Die Länge der Pause zwischen der vorhergehenden Spielperiode und Sudden Death

help = HILFE: 

# Confirmation
game-configuration-can-not-be-changed = Die Spielkonfiguration kann während eines laufenden Spiels nicht geändert werden.
    
    Was möchten Sie tun?
apply-this-game-number-change = Wie möchten Sie diese Spielnummeränderung anwenden?
UWHScores-enabled = Wenn UWHScores aktiviert ist, müssen alle Felder ausgefüllt werden.
go-back-to-editor = ZURÜCK ZUM EDITOR
discard-changes = ÄNDERUNGEN VERWERFEN
end-current-game-and-apply-changes = AKTUELLES SPIEL BEENDEN UND ÄNDERUNGEN ÜBERNEHMEN
end-current-game-and-apply-change = AKTUELLES SPIEL BEENDEN UND ÄNDERUNG ÜBERNEHMEN
keep-current-game-and-apply-change = AKTUELLES SPIEL BEIBEHALTEN UND ÄNDERUNG ÜBERNEHMEN
ok = OK
confirm-score = Ist dieser Punktestand korrekt?
    Bestätigen Sie mit dem Hauptschiedsrichter.
    
    Schwarz: { $score_black }        Weiß: { $score_white }
yes = JA
no = NEIN

# Fouls
equal = GLEICH

# Game Info
settings = EINSTELLUNGEN 
none = Keine
game-number-error = Fehler ({ $game_number })
next-game-number-error = Fehler ({ $next_game_number })
last-game-next-game = Letztes Spiel: { $prev_game },
    Nächstes Spiel: { $next_game }
black-team-white-team = Schwarzes Team: { $black_team }
    Weißes Team: { $white_team }
game-length-ot-allowed = Halbzeitdauer: { $half_length }
         Halbzeitpausendauer: { $half_time_length }
         Verlängerung Erlaubt: { $overtime }
overtime-details = Vor-Verlängerung Pause Dauer: { $pre_overtime }
             Verlängerung Halbzeitdauer: { $overtime_len }
             Verlängerung Halbzeitpausen Dauer: { $overtime_half_time_len }
sd-allowed = Sudden Death Erlaubt: { $sd }
pre-sd = Vor-Sudden-Death Pause Dauer: { $pre_sd_len }
team-to-len = Team Auszeitdauer: { $to_len }
time-btwn-games = Nominale Zeit Zwischen Spielen: { $time_btwn }
min-brk-btwn-games = Minimale Zeit Zwischen Spielen: { $min_brk_time }

# List Selecters
select-tournament = TURNIER AUSWÄHLEN
select-court = FELD AUSWÄHLEN
select-game = SPIEL AUSWÄHLEN

# Main View
add-warning = WARNUNG HINZUFÜGEN
add-foul = FOUL HINZUFÜGEN
start-now = STARTEN
end-timeout = AUSZEIT BEENDEN
warnings = WARNUNGEN
penalties = STRAFEN
dark-score-line-1 = WEISSES
dark-score-line-2 = TOR
light-score-line-1 = SCHWARZES
light-score-line-2 = TOR

# Penalties
black-penalties = SCHWARZE STRAFEN
white-penalties = WEISSE STRAFEN

# Score edit
final-score = Bitte geben Sie den endgültigen Punktestand ein

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = ENDE
end-timeout-line-2 = AUSZEIT
switch-to = WECHSELN ZU
ref = SCHIRI
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = STRAFE
penalty-shot-line-2 = SCHUSS
pen-shot = STRAFSCHUSS
## Penalty string
served = Abgesessen
dismissed = DSMS
## Config String
error = Fehler ({ $number })
none = Keine
two-games = Letztes Spiel: { $prev_game },  Nächstes Spiel: { $next_game }
one-game = Spiel: { $game }
teams = { -dark-team-name } Team: { $dark_team }
    { -light-team-name } Team: {

 $light_team }
game-config = Halbzeitdauer: { $half_len },  Halbzeitpausendauer: { $half_time_len }
    Sudden Death Erlaubt: { $sd_allowed },  Verlängerung Erlaubt: { $ot_allowed }
team-timeouts-per-half = Team Auszeiten Erlaubt Pro Halbzeit: { $team_timeouts }
team-timeouts-per-game = Team Auszeiten Erlaubt Pro Spiel: { $team_timeouts }
stop-clock-last-2 = Uhr in den letzten 2 Minuten stoppen: { $stop_clock }
ref-list = Hauptschiedsrichter: { $chief_ref }
    Zeitnehmer: { $timer }
    Wasser Schiri 1: { $water_ref_1 }
    Wasser Schiri 2: { $water_ref_2 }
    Wasser Schiri 3: { $water_ref_3 }
unknown = Unbekannt
## Game time button
next-game = NÄCHSTES SPIEL
first-half = ERSTE HALBZEIT
half-time = HALBZEITPAUSE
second-half = ZWEITE HALBZEIT
pre-ot-break-full = VOR OVERTIME PAUSE
overtime-first-half = ERSTE HALBZEIT DER VERLÄNGERUNG
overtime-half-time = VERLÄNGERUNG HALBZEIT
overtime-second-half = ZWEITE HALBZEIT DER VERLÄNGERUNG
pre-sudden-death-break = VOR SUDDEN DEATH PAUSE
sudden-death = SUDDEN DEATH
ot-first-half = OT ERSTE HALBZEIT
ot-half-time = OT HALBZEIT
ot-2nd-half = OT ZWEITE HALBZEIT
white-timeout-short = WHT AUSZEIT
white-timeout-full = WEISSE AUSZEIT
black-timeout-short = SCHWARZ AUSZEIT
black-timeout-full = SCHWARZE AUSZEIT
ref-timeout-short = SCHIRI AUSZEIT
penalty-shot-short = STRAFSCHUSS
## Make penalty dropdown
infraction = VERSTOSS
## Make warning container
team-warning-abreviation = T

# Time edit
game-time = SPIELZEIT
timeout = AUSZEIT
Note-Game-time-is-paused = Hinweis: Die Spielzeit ist auf diesem Bildschirm angehalten

# Warning Fouls Summary
fouls = FOULS
edit-warnings = WARNUNGEN BEARBEITEN
edit-fouls = FOULS BEARBEITEN

# Warnings
black-warnings = SCHWARZE WARNUNGEN
white-warnings = WEISSE WARNUNGEN

# Message
player-number = SPIELER
    NUMMER:
game-number = SPIEL
    NUMMER:
num-tos-per-half = ANZAHL AUSZEITEN
    PRO HALBZEIT:
num-tos-per-game = ANZAHL AUSZEITEN
    PRO SPIEL:

# Sound Controller - mod
off = AUS
low = NIEDRIG
medium = MITTEL
high = HOCH
max = MAX

# Config
hockey6v6 = Hockey6V6
hockey3v3 = Hockey3V3
rugby = Rugby