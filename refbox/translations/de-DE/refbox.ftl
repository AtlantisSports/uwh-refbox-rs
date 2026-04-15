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
timeout-length = AUSZEIT
    DAUER

# Warning Add
team-warning = TEAM
    VERWARNUNG
team-warning-line-1 = TEAM
team-warning-line-2 = VERWARNUNG

# Configuration
none-selected = Nichts Ausgewählt
loading = Wird geladen...
game-select = Spiel:
game-options = SPIELOPTIONEN
app-options = APP-OPTIONEN
display-options = ANZEIGEOPTIONEN
sound-options = TONOPTIONEN
app-mode = APP-
    MODUS
hide-time-for-last-15-seconds = ZEIT FÜR LETZTE
    15 SEK AUSBLENDEN
player-display-brightness = HELLIGKEIT
    SPIELERANZEIGE
confirm-score-at-game-end = PUNKTE BEI
    SPIELENDE BESTÄTIGEN
track-cap-number-of-scorer = TRIKOT-NR
    TORSCHÜTZE ERFASSEN
event = EVENT:
track-fouls-and-warnings = FOULS UND
    VERWARNUNGEN ERFASSEN
court = FELD:
single-half = EINZELNE
    HALBZEIT:
half-length-full = HALBZEITDAUER:
game-length = SPIELDAUER:
overtime-allowed = VERLÄNGERUNG
    ERLAUBT:
sudden-death-allowed = SUDDEN DEATH
    ERLAUBT:
half-time-length = PAUSE
    DAUER:
pre-ot-break-length = PAUSE VOR
    VERLÄNGERUNG:
pre-sd-break-length = PAUSE VOR
    SUDDEN DEATH:
nominal-break-between-games = NOMINALE PAUSE
    ZWISCHEN SPIELEN:
ot-half-length = HALBZEITDAUER
    VERLÄNGERUNG:
timeouts-counted-per = AUSZEITEN
    GEZÄHLT PRO:
game = SPIEL
half = HALBZEIT
minimum-brk-btwn-games = MIN PAUSE
    ZWISCHEN SPIELEN:
ot-half-time-length = PAUSE
    VERLÄNGERUNG
using-uwh-portal = UWHPORTAL VERWENDEN:
starting-sides = STARTSEITEN
sound-enabled = TON
    AKTIVIERT:
whistle-volume = PFEIFE
    LAUTSTÄRKE:
manage-remotes = FERNBEDIENUNGEN VERWALTEN
whistle-enabled = PFEIFE
    AKTIVIERT:
above-water-volume = LAUTSTÄRKE
    ÜBER WASSER:
auto-sound-start-play = AUTO TON
    SPIEL STARTEN:
buzzer-sound = SUMMER
    TON:
underwater-volume = LAUTSTÄRKE
    UNTER WASSER:
auto-sound-stop-play = AUTO TON
    SPIEL STOPPEN:
alarm-button = ALARM-
    TASTE:
alarm = ALARM
hold-to-test = HALTEN ZUM TESTEN
or-press-spacebar = Oder Leertaste drücken
or-hold-spacebar = Oder Leertaste halten
game-info = SPIELINFO
remotes = FERNBEDIENUNGEN
default = STANDARD
sound = TON: { $sound_text }
brightness = { $brightness ->
        *[Low] NIEDRIG
        [Medium] MITTEL
        [High] HOCH
        [Outdoor] AUSSEN
    }

waiting = WARTEN
add = HINZUFÜGEN
half-length = HALBZ DAUER
length-of-half-during-regular-play = Die Dauer einer Halbzeit während des regulären Spiels
half-time-lenght = PAUSE DAUER
length-of-half-time-period = Die Dauer der Halbzeitpause
nom-break = NOM PAUSE
system-will-keep-game-times-spaced = Das System versucht, die Spielstartzeiten gleichmäßig zu verteilen. Die Gesamtzeit von einem Start zum nächsten beträgt 2 × [Halbzeitdauer] + [Pausendauer] + [Nominale Zeit zwischen Spielen] (Beispiel: bei [Halbzeitdauer] = 15 Min, [Pausendauer] = 3 Min und [Nominale Zeit] = 12 Min beträgt die Zeit von Spielstart zu Spielstart 45 Min. Auszeiten oder andere Unterbrechungen verkürzen die 12 Min bis zur minimalen Zeit zwischen Spielen).
min-break = MIN PAUSE
min-time-btwn-games = Wenn ein Spiel länger dauert als geplant, ist dies die minimale Zeit zwischen Spielen, die das System einplant. Bei Rückstand holt das System bei nachfolgenden Spielen auf und hält dabei stets diese Mindestzeit ein.
pre-ot-break-abreviated = PAUSE VOR VERLÄNGERUNG
pre-sd-brk = Wenn Verlängerung aktiviert und nötig ist, dies ist die Pause zwischen der zweiten Halbzeit und der ersten Halbzeit der Verlängerung
ot-half-len = HALBZ VER DAUER
time-during-ot = Die Dauer einer Halbzeit während der Verlängerung
ot-half-tm-len = PAUSE VER DAUER
len-of-overtime-halftime = Die Dauer der Verlängerungspause
pre-sd-break = PAUSE VOR SD
pre-sd-len = Die Dauer der Pause zwischen dem vorherigen Spielabschnitt und Sudden Death
language = SPRACHE
this-language = DEUTSCH
portal-login-code = CODE
portal-login-instructions = Gehen Sie zu UWH Portal >> Veranstaltungsverwaltung >> Schiedsrichterverwaltung, klicken Sie auf die + Schaltfläche, um eine neue Refbox hinzuzufügen, und geben Sie diese Refbox-ID ein:
    { $id }

    Das UWH Portal gibt Ihnen dann einen Bestätigungscode, den Sie links über das Nummernfeld eingeben.
    Drücken Sie Fertig, sobald Sie den Code eingegeben haben

help = HILFE:

# Confirmation
game-configuration-can-not-be-changed = Die Spielkonfiguration kann während eines laufenden Spiels nicht geändert werden.

    Was möchten Sie tun?
apply-this-game-number-change = Wie möchten Sie diese Spielnummernänderung anwenden?
UWHPortal-enabled = Wenn UWHPortal aktiviert ist, müssen alle Felder ausgefüllt werden.
uwhportal-token-invalid-code = Ungültiger Code eingegeben.
    Bitte erneut versuchen.
uwhportal-token-no-pending-link = Portal erwartet keine Verbindung.
    Bitte erneut versuchen.
go-back-to-editor = ZURÜCK ZUM EDITOR
discard-changes = ÄNDERUNGEN VERWERFEN
end-current-game-and-apply-changes = AKTUELLES SPIEL BEENDEN UND ÄNDERUNGEN ÜBERNEHMEN
end-current-game-and-apply-change = AKTUELLES SPIEL BEENDEN UND ÄNDERUNG ÜBERNEHMEN
keep-current-game-and-apply-change = AKTUELLES SPIEL BEHALTEN UND ÄNDERUNG ÜBERNEHMEN
ok = OK
confirm-score = Ist dieser Punktestand korrekt?
    Mit dem Hauptschiedsrichter bestätigen.

    Schwarz: { $score_black }        Weiß: { $score_white }

    { confirmation-count-down }
yes = JA
no = NEIN

# Fouls
equal = GLEICH

# Game Info
refresh = AKTUALISIEREN
refreshing = WIRD AKTUALISIERT...
settings = EINSTELLUNGEN
none = Keiner
game-number-error = Fehler ({ $game_number })
next-game-number-error = Fehler ({ $next_game_number })
last-game-next-game = Letztes Spiel: { $prev_game },
    Nächstes Spiel: { $next_game }
black-team-white-team = Schwarz-Team: { $black_team }
    Weiß-Team: { $white_team }
game-length-ot-allowed = Halbzeitdauer: { $half_length }
         Pausendauer: { $half_time_length }
         Verlängerung Erlaubt: { $overtime }
overtime-details = Pause vor Verlängerung: { $pre_overtime }
             Halbzeitdauer Verlängerung: { $overtime_len }
             Pausendauer Verlängerung: { $overtime_half_time_len }
sd-allowed = Sudden Death Erlaubt: { $sd }
pre-sd = Pause vor Sudden Death: { $pre_sd_len }
team-to-len = Team-Auszeit Dauer: { $to_len }
time-btwn-games = Nominale Zeit zwischen Spielen: { $time_btwn }
min-brk-btwn-games = Minimale Zeit zwischen Spielen: { $min_brk_time }


# List Selecters
select-event = EVENT AUSWÄHLEN
select-court = FELD AUSWÄHLEN
select-game = SPIEL AUSWÄHLEN

# Main View
add-warning = VERWARNUNG HINZUFÜGEN
add-foul = FOUL HINZUFÜGEN
start-now = JETZT STARTEN
end-timeout = AUSZEIT BEENDEN
warnings = VERWARNUNGEN
penalties = STRAFEN
dark-score-line-1 = PUNKTE
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = PUNKTE
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = STRAFEN SCHWARZ
white-penalties = STRAFEN WEISS

# Score edit
final-score = Bitte Endergebnis eingeben
confirmation-count-down = Hinweis: Das unveränderte Ergebnis wird in { $countdown } automatisch bestätigt

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = AUSZEIT
end-timeout-line-2 = BEENDEN
switch-to = WECHSELN ZU
ref = SCHIRI
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = STRAF-
penalty-shot-line-2 = SCHUSS
pen-shot = STRAFSCHUSS
## Penalty string
served = Verbüßt
penalty = #{$player_number} - {$time ->
        [pending] Ausstehend
        [served] Verbüßt
        [total-dismissal] Ausgeschlossen
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
infraction = Verstoß: {$infraction}
## Config String
error = Fehler ({ $number })
two-games = Letztes Spiel: { $prev_game },  Nächstes Spiel: { $next_game }
one-game = Spiel: { $game }
teams = { -dark-team-name }-Team: { $dark_team }
    { -light-team-name }-Team: { $light_team }
game-config = Halbzeitdauer: { $half_len },  Pausendauer: { $half_time_len }
    Sudden Death Erlaubt: { $sd_allowed },  Verlängerung Erlaubt: { $ot_allowed }
team-timeouts-per-half = Team-Auszeiten pro Halbzeit: { $team_timeouts }
team-timeouts-per-game = Team-Auszeiten pro Spiel: { $team_timeouts }
stop-clock-last-2 = Uhr in letzten 2 Minuten stoppen: { $stop_clock }
ref-list = Hauptschiedsrichter: { $chief_ref }
    Zeitnehmer: { $timer }
    Wasserrichter 1: { $water_ref_1 }
    Wasserrichter 2: { $water_ref_2 }
    Wasserrichter 3: { $water_ref_3 }
unknown = Unbekannt
## Game time button
next-game = NÄCHSTES SPIEL
first-half = ERSTE HALBZEIT
half-time = HALBZEIT
second-half = ZWEITE HALBZEIT
pre-ot-break-full = PAUSE VOR VERLÄNGERUNG
overtime-first-half = VERLÄNGERUNG ERSTE HZ
overtime-half-time = VERLÄNGERUNGSPAUSE
overtime-second-half = VERLÄNGERUNG ZWEITE HZ
pre-sudden-death-break = PAUSE VOR SUDDEN DEATH
sudden-death = SUDDEN DEATH
ot-first-half = VERL ERSTE HZ
ot-half-time = VERL PAUSE
ot-2nd-half = VERL ZWEITE HZ
white-timeout-short = WEI AUS
white-timeout-full = AUSZEIT WEISS
black-timeout-short = SCH AUS
black-timeout-full = AUSZEIT SCHWARZ
ref-timeout-short = SCHIRI AUS
penalty-shot-short = STRAFSCH
## Make warning container
team-warning-abreviation = T
## Make time editor
zero = NULL

# Time edit
game-time = SPIELZEIT
timeout = AUSZEIT
Note-Game-time-is-paused = Hinweis: Die Spielzeit ist auf diesem Bildschirm pausiert

# Warning Fouls Summary
fouls = FOULS
edit-warnings = VERWARNUNGEN BEARBEITEN
edit-fouls = FOULS BEARBEITEN

# Warnings
black-warnings = VERWARNUNGEN SCHWARZ
white-warnings = VERWARNUNGEN WEISS

# Message
player-number = SPIELER-
    NUMMER:
game-number = SPIEL-
    NUMMER:
num-tos-per-half = AUSZEITEN
    PRO HALBZEIT:
num-tos-per-game = AUSZEITEN
    PRO SPIEL:

# Sound Controller - mod
off = AUS
low = NIEDRIG
medium = MITTEL
high = HOCH
max = MAX

# Config
hockey6v6 = HOCKEY 6G6
hockey3v3 = HOCKEY 3G3
rugby = RUGBY

# Infractions
stick-foul = Stock-Foul
illegal-advance = Unerlaubtes Vordringen
sub-foul = Wechsel-Foul
illegal-stoppage = Unerlaubtes Stoppen
out-of-bounds = Aus
grabbing-the-wall = Wandgreifen
obstruction = Behinderung
delay-of-game = Spielverzögerung
unsportsmanlike = Unsportliches Verhalten
free-arm = Freier Arm
false-start = Frühstart
