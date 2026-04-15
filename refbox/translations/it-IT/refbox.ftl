# Definitions for the translation file to use
-dark-team-name = Neri
dark-team-name-caps = NERI

-light-team-name = Bianchi
light-team-name-caps = BIANCHI

# Multipage
done = FATTO
cancel = ANNULLA
delete = ELIMINA
back = INDIETRO
new = NUOVO

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
timeout-length = DURATA
    TIMEOUT

# Warning Add
team-warning = AMMONIZIONE
    SQUADRA
team-warning-line-1 = AMMONIZIONE
team-warning-line-2 = SQUADRA

# Configuration
none-selected = Nessuno Selezionato
loading = Caricamento...
game-select = Partita:
game-options = OPZIONI PARTITA
app-options = OPZIONI APP
display-options = OPZIONI DISPLAY
sound-options = OPZIONI AUDIO
app-mode = MODALITÀ
    APP
hide-time-for-last-15-seconds = NASCONDI TEMPO
    ULTIMI 15 SEC
player-display-brightness = LUMINOSITÀ
    DISPLAY GIOCATORI
confirm-score-at-game-end = CONFERMA PUNTEGGIO
    A FINE PARTITA
track-cap-number-of-scorer = TRACCIA NUMERO
    BERRETTO MARCATORE
event = EVENTO:
track-fouls-and-warnings = TRACCIA FALLI
    E AMMONIZIONI
court = CAMPO:
single-half = SINGOLO
    TEMPO:
half-length-full = DURATA TEMPO:
game-length = DURATA PARTITA:
overtime-allowed = OVERTIME
    CONSENTITO:
sudden-death-allowed = SUDDEN DEATH
    CONSENTITO:
half-time-length = DURATA
    INTERVALLO:
pre-ot-break-length = PAUSA PRE
    OVERTIME:
pre-sd-break-length = PAUSA PRE
    SUDDEN DEATH:
nominal-break-between-games = PAUSA NOMINALE
    FRA PARTITE:
ot-half-length = DURATA TEMPO
    OVERTIME:
timeouts-counted-per = TIMEOUT
    CONTATI PER:
game = PARTITA
half = TEMPO
minimum-brk-btwn-games = PAUSA MIN
    FRA PARTITE:
ot-half-time-length = INTERVALLO
    OVERTIME
using-uwh-portal = USA UWHPORTAL:
starting-sides = LATI DI PARTENZA
sound-enabled = AUDIO
    ATTIVATO:
whistle-volume = VOLUME
    FISCHIO:
manage-remotes = GESTISCI TELECOMANDI
whistle-enabled = FISCHIO
    ATTIVATO:
above-water-volume = VOLUME
    SOPRA ACQUA:
auto-sound-start-play = AUDIO AUTO
    INIZIO GIOCO:
buzzer-sound = SUONO
    BUZZER:
underwater-volume = VOLUME
    SOTT'ACQUA:
auto-sound-stop-play = AUDIO AUTO
    FINE GIOCO:
alarm-button = PULSANTE
    ALLARME:
alarm = ALLARME
hold-to-test = TIENI PER TESTARE
or-press-spacebar = O Premi Barra Spazio
or-hold-spacebar = O Tieni Barra Spazio
game-info = INFO PARTITA
remotes = TELECOMANDI
default = PREDEFINITO
sound = AUDIO: { $sound_text }
brightness = { $brightness ->
        *[Low] BASSO
        [Medium] MEDIO
        [High] ALTO
        [Outdoor] ESTERNO
    }

waiting = IN ATTESA
add = AGGIUNGI
half-length = DUR TEMPO
length-of-half-during-regular-play = La durata di un tempo durante il gioco regolare
half-time-lenght = DUR INTERVALLO
length-of-half-time-period = La durata del periodo di intervallo
nom-break = PAUSA NOM
system-will-keep-game-times-spaced = Il sistema cercherà di mantenere i tempi di inizio partita equamente distanziati, con il tempo totale da un inizio all'altro pari a 2 × [Durata Tempo] + [Durata Intervallo] + [Tempo Nominale fra Partite] (esempio: se [Durata Tempo] = 15 min, [Durata Intervallo] = 3 min e [Tempo Nominale fra Partite] = 12 min, il tempo da inizio a inizio sarà 45 min. Eventuali timeout o altre interruzioni ridurranno i 12 min fino al raggiungimento del tempo minimo fra partite).
min-break = PAUSA MIN
min-time-btwn-games = Se una partita dura più del previsto, questo è il tempo minimo fra le partite che il sistema assegnerà. Se le partite accumulano ritardo, il sistema cercherà di recuperare nelle partite successive, rispettando sempre questo tempo minimo.
pre-ot-break-abreviated = PAUSA PRE OT
pre-sd-brk = Se l'overtime è abilitato e necessario, questa è la durata della pausa fra il Secondo Tempo e il Primo Tempo degli Overtime
ot-half-len = DUR TEMPO OT
time-during-ot = La durata di un tempo durante l'overtime
ot-half-tm-len = DUR INT OT
len-of-overtime-halftime = La durata dell'intervallo degli overtime
pre-sd-break = PAUSA PRE SD
pre-sd-len = La durata della pausa fra il periodo di gioco precedente e il Sudden Death
language = LINGUA
this-language = ITALIANO
portal-login-code = CODICE
portal-login-instructions = Vai su UWH Portal >> Gestione Evento >> Gestione Arbitri, clicca sul pulsante + per aggiungere un nuovo Refbox e inserisci questo ID Refbox:
    { $id }

    Il Portale UWH fornirà quindi un codice di conferma da inserire a sinistra usando il tastierino numerico.
    Premi Fine una volta inserito il codice

help = AIUTO:

# Confirmation
game-configuration-can-not-be-changed = La configurazione della partita non può essere modificata mentre una partita è in corso.

    Cosa vuoi fare?
apply-this-game-number-change = Come vuoi applicare questa modifica al numero di partita?
UWHPortal-enabled = Quando UWHPortal è abilitato, tutti i campi devono essere compilati.
uwhportal-token-invalid-code = Codice inserito non valido.
    Riprova.
uwhportal-token-no-pending-link = Il portale non si aspetta una connessione.
    Riprova.
go-back-to-editor = TORNA ALL'EDITOR
discard-changes = SCARTA MODIFICHE
end-current-game-and-apply-changes = TERMINA PARTITA E APPLICA MODIFICHE
end-current-game-and-apply-change = TERMINA PARTITA E APPLICA MODIFICA
keep-current-game-and-apply-change = MANTIENI PARTITA E APPLICA MODIFICA
ok = OK
confirm-score = Il punteggio è corretto?
    Conferma con l'arbitro principale.

    Neri: { $score_black }        Bianchi: { $score_white }

    { confirmation-count-down }
yes = SÌ
no = NO

# Fouls
equal = PARI

# Game Info
refresh = AGGIORNA
refreshing = AGGIORNAMENTO...
settings = IMPOSTAZIONI
none = Nessuno
game-number-error = Errore ({ $game_number })
next-game-number-error = Errore ({ $next_game_number })
last-game-next-game = Ultima Partita: { $prev_game },
    Prossima Partita: { $next_game }
black-team-white-team = Squadra Neri: { $black_team }
    Squadra Bianchi: { $white_team }
game-length-ot-allowed = Durata Tempo: { $half_length }
         Durata Intervallo: { $half_time_length }
         Overtime Consentito: { $overtime }
overtime-details = Durata Pausa Pre-Overtime: { $pre_overtime }
             Durata Tempo Overtime: { $overtime_len }
             Durata Intervallo Overtime: { $overtime_half_time_len }
sd-allowed = Sudden Death Consentito: { $sd }
pre-sd = Durata Pausa Pre-Sudden Death: { $pre_sd_len }
team-to-len = Durata Timeout Squadra: { $to_len }
time-btwn-games = Tempo Nominale fra Partite: { $time_btwn }
min-brk-btwn-games = Tempo Minimo fra Partite: { $min_brk_time }


# List Selecters
select-event = SELEZIONA EVENTO
select-court = SELEZIONA CAMPO
select-game = SELEZIONA PARTITA

# Main View
add-warning = AGGIUNGI AMMONIZIONE
add-foul = AGGIUNGI FALLO
start-now = INIZIA ORA
end-timeout = FINE TIMEOUT
warnings = AMMONIZIONI
penalties = PENALITÀ
dark-score-line-1 = SEGNA
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = SEGNA
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = PENALITÀ NERI
white-penalties = PENALITÀ BIANCHI

# Score edit
final-score = Inserisci il punteggio finale
confirmation-count-down = Nota: Il punteggio invariato sarà confermato automaticamente tra { $countdown }

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = FINE
end-timeout-line-2 = { timeout }
switch-to = PASSA A
ref = ARBITRO
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = TIRO
penalty-shot-line-2 = PENALITÀ
pen-shot = TIRO PEN
## Penalty string
served = Scontata
penalty = #{$player_number} - {$time ->
        [pending] In attesa
        [served] Scontata
        [total-dismissal] Espulso
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
infraction = Infrazione: {$infraction}
## Config String
error = Errore ({ $number })
two-games = Ultima Partita: { $prev_game },  Prossima Partita: { $next_game }
one-game = Partita: { $game }
teams = Squadra { -dark-team-name }: { $dark_team }
    Squadra { -light-team-name }: { $light_team }
game-config = Durata Tempo: { $half_len },  Durata Intervallo: { $half_time_len }
    Sudden Death Consentito: { $sd_allowed },  Overtime Consentito: { $ot_allowed }
team-timeouts-per-half = Timeout Squadra Consentiti per Tempo: { $team_timeouts }
team-timeouts-per-game = Timeout Squadra Consentiti per Partita: { $team_timeouts }
stop-clock-last-2 = Ferma Orologio negli Ultimi 2 Minuti: { $stop_clock }
ref-list = Arbitro Principale: { $chief_ref }
    Cronometrista: { $timer }
    Arbitro Acqua 1: { $water_ref_1 }
    Arbitro Acqua 2: { $water_ref_2 }
    Arbitro Acqua 3: { $water_ref_3 }
team-ref-list = Arbitri: { $ref_team }
    Cronometrista/Segnapunti: { $ts_keeper_team }
unknown = Sconosciuto
## Game time button
next-game = PROSSIMA PARTITA
first-half = PRIMO TEMPO
half-time = INTERVALLO
second-half = SECONDO TEMPO
pre-ot-break-full = PAUSA PRE OVERTIME
overtime-first-half = PRIMO TEMPO OT
overtime-half-time = INTERVALLO OT
overtime-second-half = SECONDO TEMPO OT
pre-sudden-death-break = PAUSA PRE SUDDEN DEATH
sudden-death = SUDDEN DEATH
ot-first-half = PRIMO TEMPO OT
ot-half-time = INTERVALLO OT
ot-2nd-half = SECONDO TEMPO OT
white-timeout-short = TMP BCH
white-timeout-full = TIMEOUT BIANCHI
black-timeout-short = TMP NER
black-timeout-full = TIMEOUT NERI
ref-timeout-short = TMP ARB
penalty-shot-short = TIRO PEN
## Make warning container
team-warning-abreviation = S
## Make time editor
zero = ZERO

# Time edit
game-time = TEMPO DI GIOCO
timeout = TIMEOUT
Note-Game-time-is-paused = Nota: Il tempo di gioco è in pausa su questa schermata

# Warning Fouls Summary
fouls = FALLI
edit-warnings = MODIFICA AMMONIZIONI
edit-fouls = MODIFICA FALLI

# Warnings
black-warnings = AMMONIZIONI NERI
white-warnings = AMMONIZIONI BIANCHI

# Message
player-number = NUMERO
    GIOCATORE:
game-number = NUMERO
    PARTITA:
num-tos-per-half = TIMEOUT PER
    TEMPO:
num-tos-per-game = TIMEOUT PER
    PARTITA:

# Sound Controller - mod
off = OFF
low = BASSO
medium = MEDIO
high = ALTO
max = MAX

# Config
hockey6v6 = HOCKEY 6V6
hockey3v3 = HOCKEY 3V3
rugby = RUGBY

# Infractions
stick-foul = Fallo col Bastone
illegal-advance = Avanzata Irregolare
sub-foul = Fallo di Cambio
illegal-stoppage = Stop Irregolare
out-of-bounds = Fuori Campo
grabbing-the-wall = Aggancio alla Parete
obstruction = Ostruzione
delay-of-game = Ritardo di Gioco
unsportsmanlike = Comportamento non Sportivo
free-arm = Braccio Libero
false-start = Falsa Partenza
