# Definizioni per il file di traduzione
-dark-team-name = Nero
dark-team-name-caps = NERO

-light-team-name = Bianco
light-team-name-caps = BIANCO

# Multipage
done = FATTO
restart-to-apply = RIAVVIA PER APPLICARE
cancel = ANNULLA
delete = ELIMINA
back = INDIETRO
new = NUOVO

# Modifica Penalità
total-dismissal = ES.DEF.
penalty-kind = {$kind ->
    [thirty-seconds] 30s
    [one-minute] 1m
    [two-minutes] 2m
    [four-minutes] 4m
    [five-minutes] 5m
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# Modifica Time-out di Squadra
timeout-length = TIME-OUT DI
    SQUADRA

# Aggiunta Ammonizione
team-warning = AMMONIZIONE
    DI SQUADRA
team-warning-line-1 = AMMONIZIONE
team-warning-line-2 = DI SQUADRA

# Configurazione
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
    CALOTTINA MARCATORE
event = EVENTO:
track-fouls-and-warnings = TRACCIA FALLI
    E AMMONIZIONI
court = CAMPO:
single-half = SINGOLO
    TEMPO:
half-length-full = DURATA TEMPO:
game-length = DURATA PARTITA:
overtime-allowed = TEMPI SUPPL.
    CONSENTITI:
sudden-death-allowed = MORTE IMPROVVISA
    CONSENTITA:
half-time-length = DURATA
    INTERVALLO:
pre-ot-break-length = PAUSA PRE
    SUPPLEMENTARI:
pre-sd-break-length = PAUSA PRE
    MORTE IMPROVVISA:
nominal-break-between-games = PAUSA NOMINALE
    FRA PARTITE:
ot-half-length = DURATA TEMPO
    SUPPLEMENTARE:
timeouts-counted-per = TIME-OUT
    CONTATI PER:
game = PARTITA
half = TEMPO
minimum-brk-btwn-games = PAUSA MIN
    FRA PARTITE:
ot-half-time-length = INTERVALLO
    SUPPLEMENTARE
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
    FUORI ACQUA:
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
system-will-keep-game-times-spaced = Il sistema cercherà di mantenere i tempi di inizio partita equamente distanziati, con il tempo totale da un inizio all'altro pari a 2 × [Durata Tempo] + [Durata Intervallo] + [Tempo Nominale fra Partite] (esempio: se [Durata Tempo] = 15 min, [Durata Intervallo] = 3 min e [Tempo Nominale fra Partite] = 12 min, il tempo da inizio a inizio sarà 45 min. Eventuali time-out o altre interruzioni ridurranno i 12 min fino al raggiungimento del tempo minimo fra partite).
min-break = PAUSA MIN
min-time-btwn-games = Se una partita dura più del previsto, questo è il tempo minimo fra le partite che il sistema assegnerà. Se le partite accumulano ritardo, il sistema cercherà di recuperare nelle partite successive, rispettando sempre questo tempo minimo.
pre-ot-break-abreviated = PAUSA PRE SUPPL.
pre-sd-brk = Se i tempi supplementari sono abilitati e necessari, questa è la durata della pausa fra il Secondo Tempo e il Primo Tempo Supplementare
ot-half-len = DUR TEMPO SUPPL.
time-during-ot = La durata di un tempo durante i tempi supplementari
ot-half-tm-len = DUR INT SUPPL.
len-of-overtime-halftime = La durata dell'intervallo supplementare
pre-sd-break = PAUSA PRE MORTE IMM.
pre-sd-len = La durata della pausa fra il periodo di gioco precedente e la Morte Improvvisa
language = LINGUA
this-language = ITALIANO
portal-login-code = CODICE
portal-login-instructions = Vai su UWH Portal >> Gestione Evento >> Gestione Arbitri, clicca sul pulsante + per aggiungere un nuovo Refbox e inserisci questo ID Refbox:
    { $id }

    Il Portale UWH fornirà quindi un codice di conferma da inserire a sinistra usando il tastierino numerico.
    Premi Fine una volta inserito il codice

help = AIUTO:

# Conferma
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
    Conferma con il capo arbitro.

    Nero: { $score_black }        Bianco: { $score_white }

    { confirmation-count-down }
yes = SÌ
no = NO

# Falli
equal = PARI

# Info Partita
refresh = AGGIORNA
refreshing = AGGIORNAMENTO...
settings = IMPOSTAZIONI
none = Nessuno
game-number-error = Errore ({ $game_number })
next-game-number-error = Errore ({ $next_game_number })
last-game-next-game = Ultima Partita: { $prev_game },
    Prossima Partita: { $next_game }
black-team-white-team = Squadra Nera: { $black_team }
    Squadra Bianca: { $white_team }
game-length-ot-allowed = Durata Tempo: { $half_length }
         Durata Intervallo: { $half_time_length }
         Tempi Suppl. Consentiti: { $overtime }
overtime-details = Durata Pausa Pre-Supplementari: { $pre_overtime }
             Durata Tempo Supplementare: { $overtime_len }
             Durata Intervallo Supplementare: { $overtime_half_time_len }
sd-allowed = Morte Improvvisa Consentita: { $sd }
pre-sd = Durata Pausa Pre-Morte Improvvisa: { $pre_sd_len }
team-to-len = Durata Time-out di Squadra: { $to_len }
time-btwn-games = Tempo Nominale fra Partite: { $time_btwn }
min-brk-btwn-games = Tempo Minimo fra Partite: { $min_brk_time }


# Selettori Lista
select-event = SELEZIONA EVENTO
select-court = SELEZIONA CAMPO
select-game = SELEZIONA PARTITA

# Vista Principale
add-warning = AGGIUNGI AMMONIZIONE
add-foul = AGGIUNGI FALLO
start-now = INIZIA ORA
end-timeout = FINE TIME-OUT
warnings = AMMONIZIONI
penalties = PENALITÀ
dark-score-line-1 = PUNTEGGIO
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = PUNTEGGIO
light-score-line-2 = { light-team-name-caps }

# Penalità
black-penalties = PENALITÀ NERI
white-penalties = PENALITÀ BIANCHI

# Modifica Punteggio
final-score = Inserisci il punteggio finale
confirmation-count-down = Nota: Il punteggio invariato sarà confermato automaticamente tra { $countdown }

# Elementi Condivisi
## Nastro time-out
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
penalty-shot-line-1 = TIRO DI
penalty-shot-line-2 = RIGORE
pen-shot = TIRO RIG.
## Stringa penalità
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
## Stringa configurazione
error = Errore ({ $number })
two-games = Ultima Partita: { $prev_game },  Prossima Partita: { $next_game }
one-game = Partita: { $game }
teams = Squadra { -dark-team-name }: { $dark_team }
    Squadra { -light-team-name }: { $light_team }
game-config = Durata Tempo: { $half_len },  Durata Intervallo: { $half_time_len }
    Morte Improvvisa Consentita: { $sd_allowed },  Tempi Suppl. Consentiti: { $ot_allowed }
team-timeouts-per-half = Time-out di Squadra Consentiti per Tempo: { $team_timeouts }
team-timeouts-per-game = Time-out di Squadra Consentiti per Partita: { $team_timeouts }
stop-clock-last-2 = Ferma Orologio negli Ultimi 2 Minuti: { $stop_clock }
ref-list = Capo Arbitro: { $chief_ref }
    Cronometrista: { $timer }
    Arbitro di Vasca 1: { $water_ref_1 }
    Arbitro di Vasca 2: { $water_ref_2 }
    Arbitro di Vasca 3: { $water_ref_3 }
team-ref-list = Arbitri: { $ref_team }
    Cronometrista/Segnapunti: { $ts_keeper_team }
unknown = Sconosciuto
## Pulsante tempo di gioco
next-game = PROSSIMA PARTITA
first-half = PRIMO TEMPO
half-time = INTERVALLO
second-half = SECONDO TEMPO
pre-ot-break-full = PAUSA PRE SUPPLEMENTARI
overtime-first-half = PRIMO TEMPO SUPPLEMENTARE
overtime-half-time = INTERVALLO SUPPLEMENTARE
overtime-second-half = SECONDO TEMPO SUPPLEMENTARE
pre-sudden-death-break = PAUSA PRE MORTE IMPROVVISA
sudden-death = MORTE IMPROVVISA
ot-first-half = 1° TEMPO SUPPL.
ot-half-time = INT. SUPPL.
ot-2nd-half = 2° TEMPO SUPPL.
white-timeout-short = T/O BIA
white-timeout-full = TIME-OUT BIANCHI
black-timeout-short = T/O NER
black-timeout-full = TIME-OUT NERI
ref-timeout-short = T/O ARB
penalty-shot-short = TIRO RIG.
## Contenitore ammonizione di squadra
team-warning-abreviation = S
## Editor tempo
zero = ZERO

# Modifica Tempo
game-time = TEMPO DI GIOCO
timeout = TIME-OUT
Note-Game-time-is-paused = Nota: Il tempo di gioco è in pausa su questa schermata

# Riepilogo Falli e Ammonizioni
fouls = FALLI
edit-warnings = MODIFICA AMMONIZIONI
edit-fouls = MODIFICA FALLI

# Ammonizioni
black-warnings = AMMONIZIONI NERI
white-warnings = AMMONIZIONI BIANCHI

# Messaggio
player-number = NUMERO
    GIOCATORE:
game-number = NUMERO
    PARTITA:
num-tos-per-half = N. TIME-OUT
    PER TEMPO:
num-tos-per-game = N. TIME-OUT
    PER PARTITA:

# Controllore Audio - mod
off = OFF
low = BASSO
medium = MEDIO
high = ALTO
max = MAX

# Configurazione
hockey6v6 = HOCKEY6V6
hockey3v3 = HOCKEY3V3
rugby = RUGBY

# Infrazioni
stick-foul = Fallo di Mazzetta
illegal-advance = Avanzamento Illegale
sub-foul = Fallo di Sostituzione
illegal-stoppage = Arresto Irregolare
out-of-bounds = Fuori dal Campo
grabbing-the-wall = Aggrapparsi alla Parete
obstruction = Ostruzione
delay-of-game = Perdita di Tempo
unsportsmanlike = Comportamento Antisportivo
free-arm = Braccio Libero
false-start = Falsa Partenza


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
portal-page-title-attention = Game { $game } submission error
portal-page-title-token-expired = Portal login expired
portal-page-body-token-expired = The UWH Portal login has expired. Queued scores cannot be sent until you log in again. Tap GO TO LOGIN to re-authenticate.
portal-page-attention-info = The game result has not been accepted on UWH Portal
portal-page-attention-score = Stored game result: Light { $white } - Dark { $black }
portal-page-attention-remediation = You can Retry if connection is verified, or discard to clear the error
portal-advisory-at-game-end = Portal issue detected. Score will still be queued — find an admin to resolve.
