# Definitionen für die Übersetzungsdatei
-dark-team-name = Schwarz
dark-team-name-caps = SCHWARZ

-light-team-name = Weiß
light-team-name-caps = WEISS

# Mehrere Seiten
done = FERTIG
restart-to-apply = NEU STARTEN ZUM ANWENDEN
cancel = ABBRECHEN
delete = LÖSCHEN
back = ZURÜCK
apply = ANWENDEN
save = SPEICHERN
user-options = BENUTZEROPTIONEN
new = NEU

# Strafzeit-Bearbeitung
total-dismissal = PV
penalty-kind = {$kind ->
    [thirty-seconds] 30s
    [one-minute] 1m
    [two-minutes] 2m
    [four-minutes] 4m
    [five-minutes] 5m
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# Auszeit-Bearbeitung
timeout-length = AUSZEIT
    DAUER:
team-timeout-count = AUSZEIT
    ANZAHL:

# Verwarnung hinzufügen
team-warning = MANNSCHAFT
    VERWARNUNG
team-warning-line-1 = MANNSCHAFT
team-warning-line-2 = VERWARNUNG

# Konfiguration
none-selected = Nichts ausgewählt
loading = Wird geladen...
game-select = SPIEL:
game-options = SPIELOPTIONEN
app-options = APP-OPTIONEN
display-options = ANZEIGEOPTIONEN
open-new-display = NEUE ANZEIGE ÖFFNEN
front-display-layout = DISPLAY LAYOUT
layout-default = DEFAULT
layout-classic = CLASSIC
layout-big-time = BIG TIME
layout-corners = CORNERS
layout-scores-only = SCORES ONLY
sound-options = TONOPTIONEN
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
sound-settings = TONEINSTELLUNGEN
beep-test-edit-levels = STUFEN BEARBEITEN
app-mode = APP-
    MODUS
hide-time-for-last-15-seconds = ZEIT IN DEN
    LETZTEN 15 SEK AUSBLENDEN
player-display-brightness = HELLIGKEIT
    SPIELERANZEIGE
confirm-score-at-game-end = SPIELSTAND BEI
    SPIELENDE BESTÄTIGEN
track-cap-number-of-scorer = BADEKAPPENNUMMER
    DES TORSCHÜTZEN ERFASSEN
event = VERANSTALTUNG:
track-fouls-and-warnings = FOULS UND
    VERWARNUNGEN ERFASSEN
show-behind-schedule-time = VERZÖGERUNG ANZEIGEN
delay = VERZÖGERUNG
court = FELD:
single-half = EINZELNE
    HALBZEIT:
half-length-full = HALBZEITDAUER:
game-length = SPIELDAUER:
overtime-allowed = VERLÄNGERUNG
    ERLAUBT:
sudden-death-allowed = PLÖTZLICHER TOD
    ERLAUBT:
half-time-length = HALBZEITPAUSE
    DAUER:
pre-ot-break-length = PAUSE VOR
    VERLÄNGERUNG:
pre-sd-break-length = PAUSE VOR
    PLÖTZL. TOD:
nominal-break-between-games = NOMINALE PAUSE
    ZWISCHEN SPIELEN:
ot-half-length = VERLÄNGERUNGS-
    HALBZEITDAUER:
timeouts-counted-per = AUSZEITEN
    GEZÄHLT PRO:
game = SPIEL
half = HALBZEIT
minimum-brk-btwn-games = MIN. PAUSE
    ZWISCHEN SPIELEN:
ot-half-time-length = VERLÄNGERUNGS-
    PAUSENDAUER
using-portal = { $portal }PORTAL VERWENDEN:
starting-sides = STARTSEITEN
sound-enabled = TON
    AKTIVIERT:
whistle-volume = PFEIFEN-
    LAUTSTÄRKE:
manage-remotes = FERNBEDIENUNGEN VERWALTEN
whistle-enabled = PFEIFE
    AKTIVIERT:
above-water-volume = LAUTSTÄRKE
    ÜBER WASSER:
auto-sound-start-play = AUTO-TON
    SPIELSTART:
buzzer-sound = SUMMER-
    TON:
underwater-volume = LAUTSTÄRKE
    UNTER WASSER:
auto-sound-stop-play = AUTO-TON
    SPIELENDE:
alarm-button = ALARM-
    TASTE:
alarm = ALARM
hold-to-test = ZUM TESTEN HALTEN
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
half-length = HALBZ. DAUER
length-of-half-during-regular-play = Die Dauer einer Halbzeit während des regulären Spiels
half-time-lenght = HALBZEITPAUSE DAUER
length-of-half-time-period = Die Dauer der Halbzeitpause
nom-break = NOM. PAUSE
game-block = SPIELBLOCK
game-block-help = Zeit vom Beginn eines Spiels bis zum Beginn des nächsten
game-block-too-short = Zu kurz für das Spiel plus die Mindestpause
game-block-tight = Knapp — Auszeiten könnten die Spiele über ihren Zeitslot hinausschieben
system-will-keep-game-times-spaced = Das System versucht, die Spielstartzeiten gleichmäßig zu verteilen. Die Gesamtzeit von einem Start zum nächsten beträgt 2 × [Halbzeitdauer] + [Halbzeitpausendauer] + [Nominale Spielpause] (Beispiel: bei [Halbzeitdauer] = 15 Min, [Halbzeitpausendauer] = 3 Min und [Nominale Spielpause] = 12 Min beträgt die Zeit von Spielstart zu Spielstart 45 Min. Auszeiten oder andere Spielunterbrechungen verkürzen die 12 Min bis zur minimalen Pause zwischen Spielen).
min-break = MIN. PAUSE
min-time-btwn-games = Wenn ein Spiel länger dauert als geplant, ist dies die minimale Pause zwischen Spielen, die das System einplant. Bei Rückstand holt das System bei nachfolgenden Spielen auf und hält dabei stets diese Mindestpause ein.
pre-ot-break-abreviated = PAUSE VOR VERLÄNGERUNG
pre-sd-brk = Wenn Verlängerung aktiviert und erforderlich ist, ist dies die Dauer der Pause zwischen der zweiten Halbzeit und der ersten Halbzeit der Verlängerung
ot-half-len = VERL. HALBZ. DAUER
time-during-ot = Die Dauer einer Halbzeit während der Verlängerung
ot-half-tm-len = VERL. PAUSE DAUER
len-of-overtime-halftime = Die Dauer der Verlängerungspause
pre-sd-break = PAUSE VOR PLÖTZL. TOD
pre-sd-len = Die Dauer der Pause zwischen dem vorherigen Spielabschnitt und dem Plötzlichen Tod
language = SPRACHE
this-language = DEUTSCH
portal-login-code = CODE
portal-login-instructions = Gehen Sie zu { $portal } Portal >> Veranstaltungsverwaltung >> Schiedsrichterverwaltung, klicken Sie auf die + Schaltfläche, um eine neue Refbox hinzuzufügen, und geben Sie diese Refbox-ID ein:
    { $id }

    Das { $portal } Portal stellt Ihnen dann einen Bestätigungscode bereit, den Sie links über das Nummernfeld eingeben.
    Drücken Sie Fertig, sobald Sie den Code eingegeben haben

help = HILFE:

# Bestätigung
game-configuration-can-not-be-changed = Die Spielkonfiguration kann während eines laufenden Spiels nicht geändert werden.

    Was möchten Sie tun?
apply-this-game-number-change = Wie möchten Sie diese Spielnummernänderung anwenden?
portal-enabled = Wenn { $portal }PORTAL aktiviert ist, müssen alle Felder ausgefüllt werden.
mode-switch-portal-tenant = Wenn Sie den Modus von { $from_mode } zu { $to_mode } wechseln, wird die Verbindung zu { $from_portal }PORTAL getrennt und Sie müssen sich erneut mit { $to_portal }PORTAL verbinden.
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
confirm-score = Ist dieser Spielstand korrekt?
    Mit dem Hauptschiedsrichter bestätigen.

    Schwarz: { $score_black }        Weiß: { $score_white }

    { confirmation-count-down }
yes = JA
no = NEIN

# Fouls
equal = GLEICH

# Spielinfo
refresh = AKTUALISIEREN
refreshing = WIRD AKTUALISIERT...
settings = EINSTELLUNGEN
none = Keine
game-number-error = Fehler ({ $game_number })
next-game-number-error = Fehler ({ $next_game_number })
last-game-next-game = Letztes Spiel: { $prev_game },
    Nächstes Spiel: { $next_game }
black-team-white-team = Schwarze Mannschaft: { $black_team }
    Weiße Mannschaft: { $white_team }
game-length-ot-allowed = Halbzeitdauer: { $half_length }
         Halbzeitpausendauer: { $half_time_length }
         Verlängerung Erlaubt: { $overtime }
overtime-details = Pause vor Verlängerung: { $pre_overtime }
             Halbzeitdauer Verlängerung: { $overtime_len }
             Halbzeitpausendauer Verlängerung: { $overtime_half_time_len }
sd-allowed = Plötzlicher Tod Erlaubt: { $sd }
pre-sd = Pause vor Plötzlichem Tod: { $pre_sd_len }
team-to-len = Mannschafts-Auszeit Dauer: { $to_len }
time-btwn-games = Nominale Spielpause: { $time_btwn }
game-block-info = Spielblock: { $game_block }
min-brk-btwn-games = Minimale Spielpause: { $min_brk_time }


# Listenauswahl
select-event = VERANSTALTUNG AUSWÄHLEN
select-court = FELD AUSWÄHLEN
select-game = SPIEL AUSWÄHLEN

# Hauptansicht
add-warning = VERWARNUNG HINZUFÜGEN
add-foul = FOUL HINZUFÜGEN
start-now = JETZT STARTEN
end-timeout = AUSZEIT BEENDEN
warnings = VERWARNUNGEN
penalties = STRAFZEITEN
dark-score-line-1 = TORE
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = TORE
light-score-line-2 = { light-team-name-caps }

# Strafzeiten
black-penalties = STRAFZEITEN SCHWARZ
white-penalties = STRAFZEITEN WEISS

# Spielstand bearbeiten
final-score = Bitte Endergebnis eingeben
confirmation-count-down = Hinweis: Der unveränderte Spielstand wird in { $countdown } automatisch bestätigt

# Gemeinsame Elemente
## Auszeit-Leiste
end-timeout-line-1 = AUSZEIT
end-timeout-line-2 = BEENDEN
cancel-timeout = { cancel } { timeout }
cancel-timeout-line-1 = { cancel }
cancel-timeout-line-2 = { timeout }
cancel-ref-timeout = { cancel } { ref } { timeout }
cancel-ref-timeout-line-1 = { cancel } { ref }
cancel-ref-timeout-line-2 = { timeout }
cancel-pen-shot = { cancel } { pen-shot }
cancel-pen-shot-line-1 = { cancel }
cancel-pen-shot-line-2 = { pen-shot }
switch-to = WECHSELN ZU
ref = SCHIRI
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = STRAF-
penalty-shot-line-2 = STOSS
pen-shot = STRAFSTOSS
## Strafzeit-Zeichenkette
served = Verbüßt
penalty = #{$player_number} - {$time ->
        [pending] Ausstehend
        [served] Verbüßt
        [total-dismissal] Platzverweis
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
## Konfigurationszeichenkette
error = Fehler ({ $number })
two-games = Letztes Spiel: { $prev_game },  Nächstes Spiel: { $next_game }
one-game = Spiel: { $game }
teams = { -dark-team-name } Mannschaft: { $dark_team }
    { -light-team-name } Mannschaft: { $light_team }
game-config = Halbzeitdauer: { $half_len },  Halbzeitpausendauer: { $half_time_len }
    Plötzlicher Tod Erlaubt: { $sd_allowed },  Verlängerung Erlaubt: { $ot_allowed }
team-timeouts = Mannschafts-Auszeiten: { $value }
team-timeouts-label = MANNSCHAFTS-
    AUSZEITEN:
stop-clock-last-2 = Uhr in den letzten 2 Minuten stoppen: { $stop_clock }
ref-list = Hauptschiedsrichter: { $chief_ref }
    Zeitnehmer: { $timer }
    Wasserschiedsrichter 1: { $water_ref_1 }
    Wasserschiedsrichter 2: { $water_ref_2 }
    Wasserschiedsrichter 3: { $water_ref_3 }
team-ref-list = Schiedsrichter: { $ref_team }
    Zeitnehmer/Anschreiber: { $ts_keeper_team }
unknown = Unbekannt
## Spielzeit-Schaltfläche
next-game = NÄCHSTES SPIEL
first-half = ERSTE HALBZEIT
half-time = HALBZEITPAUSE
second-half = ZWEITE HALBZEIT
pre-ot-break-full = PAUSE VOR VERLÄNGERUNG
overtime-first-half = ERSTE VERLÄNGERUNGSHÄLFTE
overtime-half-time = VERLÄNGERUNGSPAUSE
overtime-second-half = ZWEITE VERLÄNGERUNGSHÄLFTE
pre-sudden-death-break = PAUSE VOR PLÖTZLICHEM TOD
sudden-death = PLÖTZLICHER TOD
ot-first-half = VERL. ERSTE HZ
ot-half-time = VERL. PAUSE
ot-2nd-half = VERL. ZWEITE HZ
white-timeout-short = WEI AUS
white-timeout-full = AUSZEIT WEISS
black-timeout-short = SCH AUS
black-timeout-full = AUSZEIT SCHWARZ
ref-timeout-short = SCHIRI AUS
penalty-shot-short = STRAFSTOSS
## Verwarnungscontainer erstellen
team-warning-abreviation = M
## Zeiteditor erstellen
zero = = 0

# Zeitbearbeitung
game-time = SPIELZEIT
timeout = AUSZEIT
Note-Game-time-is-paused = Hinweis: Die Spielzeit ist auf diesem Bildschirm pausiert

# Fouls- und Verwarnungsübersicht
fouls = FOULS
edit-warnings = VERWARNUNGEN BEARBEITEN
edit-fouls = FOULS BEARBEITEN

# Verwarnungen
black-warnings = VERWARNUNGEN SCHWARZ
white-warnings = VERWARNUNGEN WEISS

# Meldung
player-number = SPIELER-
    NUMMER:
game-number = SPIEL-
    NUMMER:
num-tos-per-half = AUSZEITEN
    PRO HALBZEIT:
num-tos-per-game = AUSZEITEN
    PRO SPIEL:

# Ton-Controller - Modus
off = AUS
low = NIEDRIG
medium = MITTEL
high = HOCH
max = MAX

# Konfiguration
hockey6v6 = HOCKEY6G6
hockey3v3 = HOCKEY3G3
rugby = RUGBY
beep-test = BEEP TEST

# Beep-test screen
beep-test-pre = VOR
beep-test-top-time-label = ZEIT
beep-test-top-level-label = STUFE
beep-test-top-lap-label = RUNDE
beep-test-start = START
beep-test-pause = PAUSE
beep-test-resume = FORTSETZEN
beep-test-reset = ZURÜCKSETZEN
beep-test-column-level = STUFE
beep-test-column-count = ANZAHL
beep-test-column-duration = DAUER
beep-test-edit-selected = Stufe { $level }
beep-test-edit-time = ZEIT
beep-test-edit-count = ANZAHL
beep-test-edit-new = STUFE HINZUFÜGEN
beep-test-edit-remove = STUFE ENTFERNEN

# Vergehen
stick-foul = Schläger-Foul
illegal-advance = Regelwidriges Vorrücken
sub-foul = Auswechsel-Foul
illegal-stoppage = Regelwidrige Spielunterbrechung
out-of-bounds = Außerhalb des Spielfeldes
grabbing-the-wall = Festhalten am Beckenrand
obstruction = Behinderung
delay-of-game = Spielverzögerung
unsportsmanlike = Unsportliches Verhalten
free-arm = Freier Arm
false-start = Fehlstart


# Portal Health Indicator
portal-summary-title = { $portal } PORTAL STATUS
portal-row-token-expired = Portal-Anmeldung abgelaufen — zum erneuten Anmelden tippen
portal-row-stuck = Spiel { $game } Übermittlungsfehler, zum Beheben tippen
portal-row-pending = Spiel { $game } Spielstand nicht gesendet, zum erneuten Versuch tippen
portal-row-attempt-suffix = (Versuch { $attempts })
portal-row-recent = Spiel { $game } · Übermittelt vor { $mins } Min
portal-action-force-submit = Dieses Spielergebnis erneut senden
portal-action-discard = Dieses Spielergebnis verwerfen
portal-action-discard-confirm = ZUM BESTÄTIGEN DES VERWERFENS ERNEUT TIPPEN
portal-page-title-attention = Übermittlungsfehler Spiel { $game }
portal-page-attention-info = Das Spielergebnis wurde vom { $portal } Portal nicht angenommen
portal-page-attention-score = Gespeichertes Spielergebnis: Weiß { $white } - Schwarz { $black }
portal-page-attention-remediation = Sie können erneut versuchen, wenn die Verbindung bestätigt ist, oder verwerfen, um den Fehler zu löschen
portal-advisory-at-game-end = Portal-Problem erkannt. Spielstand wird trotzdem in die Warteschlange gestellt — bitte einen Administrator zur Lösung kontaktieren.

# 2 Halves / 1 Period selector (Half Length editor)
two-halves = 2 HALBZEITEN
one-period = 1 PERIODE
game-len = SPIELDAUER
length-of-game-during-regular-play = Die Spieldauer während des regulären Spiels

# Single-period (1 Period) info variants — Game Length, no half-time line
game-config-single-half = Spieldauer: { $half_len }
    Plötzlicher Tod Erlaubt: { $sd_allowed },  Verlängerung Erlaubt: { $ot_allowed }
game-length-ot-allowed-single-half = Spieldauer: { $half_length }
         Verlängerung Erlaubt: { $overtime }

# Self-update / Updates page
check-version = Version prüfen
updates-current-version = Aktuelle Version
updates-check-for-updates = Nach Updates suchen
updates-install = Installieren
updates-do-revert = Zurücksetzen
updates-install-note = Beim Klick auf „Installieren“ wird das Update heruntergeladen, installiert und die Refbox neu gestartet
updates-revert-note = Beim Klick auf „Zurücksetzen“ wird die vorherige Version wiederhergestellt und die Refbox neu gestartet
updates-unknown = Unbekannt
updates-checking = Wird geprüft…
updates-up-to-date = Auf dem neuesten Stand.
updates-available = Update verfügbar: {$version}
updates-downloading = Wird heruntergeladen…
updates-verifying = Download wird geprüft…
updates-installing = Wird installiert…
updates-restarting = Wird neu gestartet…
updates-confirm-revert = Zur vorherigen Version ({$version}) zurückkehren?
updates-rolled-back = Zur vorherigen Version zurückgesetzt, da das Update nicht korrekt gestartet ist. Bitte versuchen Sie es erneut.
updates-revert = Zur vorherigen Version zurückkehren ({$version})
updates-error-no-internet = Der Update-Server konnte nicht erreicht werden, bitte prüfen Sie Ihre Internetverbindung
updates-error-bad-download = Das heruntergeladene Update war ungültig und wurde nicht installiert.
updates-error-rate-limited = Der Update-Server ist ausgelastet, bitte versuchen Sie es in Kürze erneut.
updates-error-no-space = Nicht genügend freier Speicherplatz, um das Update zu installieren.
updates-error-not-writable = Das Update konnte nicht gespeichert werden (Zugriff verweigert).
