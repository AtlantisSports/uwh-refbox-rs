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
apply = APPLICA
save = SALVA
user-options = OPZIONI UTENTE
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
    SQUADRA:
team-timeout-count = NUMERO DI
    TIME-OUT:

# Aggiunta Ammonizione
team-warning = AMMONIZIONE
    DI SQUADRA
team-warning-line-1 = AMMONIZIONE
team-warning-line-2 = DI SQUADRA

# Configurazione
none-selected = Nessuno Selezionato
loading = Caricamento...
game-select = PARTITA:
game-options = OPZIONI PARTITA
app-options = OPZIONI APP
display-options = OPZIONI DISPLAY
open-new-display = APRI NUOVO DISPLAY
front-display-layout = DISPLAY LAYOUT
layout-default = DEFAULT
layout-classic = CLASSIC
layout-big-time = BIG TIME
layout-corners = CORNERS
layout-scores-only = SCORES ONLY
sound-options = OPZIONI AUDIO
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
sound-settings = IMPOSTAZIONI AUDIO
beep-test-edit-levels = MODIFICA LIVELLI
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
show-behind-schedule-time = MOSTRA RITARDO
delay = RITARDO
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
using-portal = USA { $portal }PORTAL:
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
game-block = BLOCCO PARTITA
game-block-help = Tempo dall'inizio di una partita all'inizio della successiva
game-block-too-short = Troppo breve per contenere la partita più la pausa minima
game-block-tight = Stretto — i timeout potrebbero far sforare le partite dal loro slot
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
portal-login-instructions = Vai su { $portal } Portal >> Gestione Evento >> Gestione Arbitri, clicca sul pulsante + per aggiungere un nuovo Refbox e inserisci questo ID Refbox:
    { $id }

    Il Portale { $portal } fornirà quindi un codice di conferma da inserire a sinistra usando il tastierino numerico.
    Premi Fine una volta inserito il codice

help = AIUTO:

# Conferma
game-configuration-can-not-be-changed = La configurazione della partita non può essere modificata mentre una partita è in corso.

    Cosa vuoi fare?
apply-this-game-number-change = Come vuoi applicare questa modifica al numero di partita?
portal-enabled = Quando { $portal }PORTAL è abilitato, tutti i campi devono essere compilati.
mode-switch-portal-tenant = Cambiare modalità da { $from_mode } a { $to_mode } disabiliterà il collegamento a { $from_portal }PORTAL e sarà necessario ricollegarsi a { $to_portal }PORTAL.
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
game-block-info = Blocco di Gioco: { $game_block }
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
cancel-timeout = { cancel } { timeout }
cancel-timeout-line-1 = { cancel }
cancel-timeout-line-2 = { timeout }
cancel-ref-timeout = { cancel } { ref } { timeout }
cancel-ref-timeout-line-1 = { cancel } { ref }
cancel-ref-timeout-line-2 = { timeout }
cancel-pen-shot = { cancel } { pen-shot }
cancel-pen-shot-line-1 = { cancel }
cancel-pen-shot-line-2 = { pen-shot }
switch-to = PASSA A
ref = ARBITRO
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
revive-hold-line-1 = TIENI PER
revive-hold-line-2 = RIPRISTINA
revive-deciding-line-2 = RIPRISTINATO
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
team-timeouts = Time-out di Squadra: { $value }
team-timeouts-label = TIME-OUT DI
    SQUADRA:
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
zero = = 0

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
beep-test = BEEP TEST

# Beep-test screen
beep-test-pre = PRE
beep-test-top-time-label = TEMPO
beep-test-top-level-label = LIVELLO
beep-test-top-lap-label = GIRO
beep-test-start = AVVIA
beep-test-pause = PAUSA
beep-test-resume = RIPRENDI
beep-test-reset = AZZERA
beep-test-column-level = LIVELLO
beep-test-column-count = NUMERO
beep-test-column-duration = DURATA
beep-test-edit-selected = Livello { $level }
beep-test-edit-time = TEMPO
beep-test-edit-count = NUMERO
beep-test-edit-new = AGGIUNGI LIVELLO
beep-test-edit-remove = RIMUOVI LIVELLO

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
portal-summary-title = STATO PORTALE { $portal }
portal-row-token-expired = Sessione Portale scaduta — tocca per accedere di nuovo
portal-row-stuck = Partita { $game } Errore invio punteggio, tocca per correggere
portal-row-pending = Partita { $game } Punteggio non inviato, tocca per riprovare
portal-row-attempt-suffix = (tentativo { $attempts })
portal-row-recent = Partita { $game } · Inviato { $mins } min fa
portal-action-force-submit = Riprova questo risultato
portal-action-discard = Scarta questo risultato
portal-action-discard-confirm = TOCCA DI NUOVO PER CONFERMARE LO SCARTO
portal-page-title-attention = Errore invio Partita { $game }
portal-page-attention-info = Il risultato della partita non è stato accettato dal Portale { $portal }
portal-page-attention-score = Risultato memorizzato: Bianco { $white } - Nero { $black }
portal-page-attention-remediation = Puoi Riprovare se la connessione è verificata, oppure scartare per annullare l'errore
portal-advisory-at-game-end = Rilevato problema con il Portale. Il punteggio sarà comunque accodato — contatta un amministratore.

# 2 Halves / 1 Period selector (Half Length editor)
two-halves = 2 TEMPI
one-period = 1 PERIODO
game-len = DURATA PARTITA
length-of-game-during-regular-play = La durata della partita durante il gioco regolare

# Single-period (1 Period) info variants — Game Length, no half-time line
game-config-single-half = Durata Partita: { $half_len }
    Morte Improvvisa Consentita: { $sd_allowed },  Tempi Suppl. Consentiti: { $ot_allowed }
game-length-ot-allowed-single-half = Durata Partita: { $half_length }
         Tempi Suppl. Consentiti: { $overtime }

# Self-update / Updates page
check-version = Controlla versione
updates-current-version = Versione attuale
updates-check-for-updates = Controlla aggiornamenti
updates-install = Installa
updates-do-revert = Ripristina
updates-install-note = Cliccando su Installa l’aggiornamento verrà scaricato e installato e la refbox verrà riavviata
updates-revert-note = Cliccando su Ripristina verrà ripristinata la versione precedente e la refbox verrà riavviata
updates-unknown = Sconosciuto
updates-checking = Controllo in corso…
updates-up-to-date = Aggiornato.
updates-available = Aggiornamento disponibile: {$version}
updates-downloading = Download in corso…
updates-verifying = Controllo del download…
updates-installing = Installazione in corso…
updates-restarting = Riavvio in corso…
updates-confirm-revert = Tornare alla versione precedente ({$version})?
updates-rolled-back = Ripristinato alla versione precedente perché l’aggiornamento non si è avviato correttamente, riprova.
updates-revert = Torna alla versione precedente ({$version})
updates-error-no-internet = Impossibile raggiungere il server degli aggiornamenti, controlla la tua connessione internet
updates-error-bad-download = L’aggiornamento scaricato non era valido e non è stato installato.
updates-error-rate-limited = Il server degli aggiornamenti è occupato, riprova tra poco.
updates-error-no-space = Spazio libero insufficiente per installare l’aggiornamento.
updates-error-not-writable = Impossibile salvare l’aggiornamento (permesso negato).
