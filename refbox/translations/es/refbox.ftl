# Definitions for the translation file to use
-dark-team-name = Negro
dark-team-name-caps = NEGRO
-light-team-name = Blanco
light-team-name-caps = BLANCO

# Multipage
done = HECHO
cancel = CANCELAR
delete = ELIMINAR
back = ATRÁS
new = NUEVO

# Penalty Edit
total-dismissal = T

# Team Timeout Edit
timeout-length = DURACIÓN DEL
    TIEMPO DE ESPERA

# Warning Add
team-warning = AVISO
    DE EQUIPO
team-warning-line-1 = AVISO
team-warning-line-2 = DE EQUIPO

# Configuration
none-selected = Ninguno seleccionado
loading = Cargando...
game-select = Juego:
game-options = OPCIONES DE JUEGO
app-options = OPCIONES DE APP
display-options = OPCIONES DE PANTALLA
sound-options = OPCIONES DE SONIDO
app-mode = MODO DE
    LA APLICACIÓN
hide-time-for-last-15-seconds = OCULTAR TIEMPO PARA
    LOS ÚLTIMOS 15 SEGUNDOS
player-display-brightness = BRILLO DE LA
    PANTALLA DEL JUGADOR
confirm-score-at-game-end = CONFIRMAR PUNTUACIÓN
    AL FINAL DEL JUEGO
track-cap-number-of-scorer = REGISTRAR NÚMERO
    DEL ANOTADOR
track-fouls-and-warnings = REGISTRAR FALTAS
    Y ADVERTENCIAS
event = EVENTO:
court = CANCHA:
single-half = UNA SOLA MITAD:
half-length-full = DURACIÓN DE
    UNA MITAD:
game-length = DURACIÓN DEL
    JUEGO:
overtime-allowed = TIEMPO EXTRA (T/E)
    PERMITIDO:
sudden-death-allowed = MUERTE SÚBITA (M/S)
    PERMITIDA:
half-time-length = DURACIÓN DEL
    MEDIO TIEMPO:
pre-ot-break-length = DURACIÓN
    PREVIA AL T/E:
pre-sd-break-length = DURACIÓN PREVIA
    A LA M/S:
nominal-break-between-games = DESCANSO
    ENTRE JUEGOS:
ot-half-length = DURACIÓN DE LA
    MITAD DEL T/M:
timeouts-counted-per = TIEMPOS MUERTOS
    CONTADOS POR:
game = JUEGO
half = MITAD
minimum-brk-btwn-games = DESCANSO MÍNIMO
    ENTRE JUEGOS:
ot-half-time-length = DUR. MEDIO
    TIEMPO DEL T/E
using-uwh-portal = USANDO UWHPORTAL:
starting-sides = LADOS INICIALES
sound-enabled = SONIDO
    HABILITADO:
whistle-volume = VOLUMEN
    DEL SILBATO:
manage-remotes = GESTIONAR REMOTOS
whistle-enabled = SILBATO
    HABILITADO:
above-water-volume = VOLUMEN
    SOBRE EL AGUA:
auto-sound-start-play = INICIO AUTOMÁTICO
    DE SONIDO AL JUGAR:
buzzer-sound = SONIDO
    DEL ALTAVOZ:
underwater-volume = VOLUMEN
    BAJO EL AGUA:
auto-sound-stop-play = SONIDO AUTOMÁTICO
    AL PARAR:
remotes = REMOTOS
default = POR DEFECTO
sound = SONIDO: { $sound_text }
brightness = { $brightness ->
        *[Low] BAJO
        [Medium] MEDIO
        [High] ALTO
        [Outdoor] EXTERIOR
    }

waiting = EN ESPERA
add = AÑADIR
half-length = DURACIÓN DE
    UNA MITAD
length-of-half-during-regular-play = La duración de una mitad durante el juego regular
half-time-lenght = DURACIÓN DEL
    MEDIO TIEMPO
length-of-half-time-period = La duración del período de medio tiempo
nom-break = DESCANSO NOMINAL
system-will-keep-game-times-spaced = El sistema intentará mantener los tiempos de inicio de los juegos espaciados uniformemente, con el tiempo total de un juego al siguiente siendo 2 * [Duración de la Mitad] + [Duración del Medio Tiempo] + [Tiempo Nominal Entre Juegos] (ejemplo: si los juegos tienen [Duración de la Mitad] = 15m, [Duración del Medio Tiempo] = 3m, y [Tiempo Nominal Entre Juegos] = 12m, el tiempo desde el inicio de un juego al siguiente será de 45m. Cualquier tiempo de espera tomado, u otras paradas de reloj, reducirán el tiempo de 12m hasta alcanzar el tiempo mínimo entre juegos).
min-break = DESCANSO MÍNIMO
min-time-btwn-games = Si un juego dura más de lo programado, este es el tiempo mínimo entre juegos que el sistema asignará. Si los juegos se retrasan, el sistema intentará automáticamente reajustarse después de los juegos subsecuentes, siempre respetando este tiempo mínimo entre juegos.
pre-ot-break-abreviated = DESCANSO PREVIO
    A LA PRORROGA
pre-sd-brk = Si se habilita el tiempo extra y es necesario, esta es la duración del descanso entre la segunda y la primera mitad del tiempo extra
ot-half-len = DURACIÓN DE LA
    MITAD DEL T/E
time-during-ot = La duración de una mitad durante el tiempo extra
ot-half-tm-len = DURACIÓN DEL MEDIO
    TIEMPO DEL T/E
len-of-overtime-halftime = La duración del medio tiempo durante el tiempo extra
pre-sd-break = DESCANSO PREVIO
    A MUERTE SÚBITA
pre-sd-len = La duración del descanso entre el período de juego precedente y la muerte súbita
help = AYUDA:

# Confirmation
game-configuration-can-not-be-changed = La configuración del juego no se puede cambiar mientras un juego está en progreso.

    ¿Qué te gustaría hacer?
apply-this-game-number-change = ¿Cómo te gustaría aplicar este cambio de número de juego?
UWHPortal-enabled = Cuando UWHPortal está habilitado, todos los campos deben ser completados.
go-back-to-editor = VOLVER AL EDITOR
discard-changes = DESCARTAR CAMBIOS
end-current-game-and-apply-changes = TERMINAR EL JUEGO ACTUAL Y APLICAR CAMBIOS
end-current-game-and-apply-change = TERMINAR EL JUEGO ACTUAL Y APLICAR CAMBIO
keep-current-game-and-apply-change = MANTENER EL JUEGO ACTUAL Y APLICAR CAMBIO
ok = OK
confirm-score = ¿Es correcto este puntaje?
    Confirma con el árbitro principal.

    Negro: { $score_black }        Blanco: { $score_white }
yes = SI
no = NO

# Fouls
equal = IGUAL

# Game Info
settings = CONFIGURACIÓN
none = Ninguno
game-number-error = Error ({ $game_number })
next-game-number-error = Error ({ $next_game_number })
last-game-next-game = Último partido: { $prev_game },
    Próximo Juego: { $next_game }
black-team-white-team = Equipo Negro: { $black_team }
    Equipo Blanco: { $white_team }
game-length-ot-allowed = Duración de una mitad: { $half_length }
         Duración del medio tiempo: { $half_time_length }
         Tiempo extra habilitado: { $overtime }
overtime-details = Duración del descanso previo al tiempo extra: { $pre_overtime }
             Duración de la mitad del tiempo extra: { $overtime_len }
             Duración del medio tiempo extra: { $overtime_half_time_len }
sd-allowed = Muerte súbita habilitada: { $sd }
pre-sd = Duración del descanso previo a la muerte súbita: { $pre_sd_len }
team-to-len = Duración del tiempo muerto de equipo: { $to_len }
time-btwn-games = Tiempo nominal entre partidos: { $time_btwn }
min-brk-btwn-games = Tiempo mínimo entre partidos: { $min_brk_time }


# List Selecters
select-event = SELECCIONAR EVENTO
select-court = SELECCIONAR CANCHA
select-game = SELECCIONAR PARTIDO

# Main View
add-warning = AÑADIR AVISO
add-foul = AÑADIR FALTA
start-now = EMPEZAR
end-timeout = TERMINAR TIEMPO MUERTO
warnings = AVISOS
penalties = EXPULSIONES
dark-score-line-1 = PUNTUACIÓN
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = PUNTUACIÓN
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = EXPULSIONES E. NEGRO
white-penalties = EXPULSIONES E. BLANCO

# Score edit
final-score = Por favor ingrese la puntuación final

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = TERMINAR
end-timeout-line-2 = TIEMPO MUERTO
switch-to = CAMBIAR A
ref = ÁRBITRO
ref-timeout-line-1 = { timeout }
ref-timeout-line-2 = { ref }
dark-timeout-line-1 = { timeout }
dark-timeout-line-2 = E. { dark-team-name-caps }
light-timeout-line-1 = { timeout }
light-timeout-line-2 = E. { light-team-name-caps }
penalty-shot-line-1 = TIRO
penalty-shot-line-2 = PENAL
pen-shot = TIRO PENAL
## Penalty string
served = Servido
total-dismissal = DESCARTADO
penalty = #{$player_number} - {$time ->
        [pending] Pendiente
        [served] Servido
        [total-dismissal] Descartado
       *[number] {$time}
    } {$time ->
        [total-dismissal] {""}
       *[other] ({$kind ->
            [thirty-seconds] 30s
            [one-minute] 1m
            [two-minutes] 2m
            [four-minutes] 4m
            [five-minutes] 5m
           *[other] {$kind}
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
infraction = Infracción: {$infraction}
## Config String
error = Error ({ $number })
none = Ninguno
two-games = Último partido: { $prev_game }, Próximo partido: { $next_game }
one-game = Juego: { $game }
teams = { -dark-team-name } Equipo: { $dark_team }
    { -light-team-name } Equipo: { $light_team }
game-config = Duración de la mitad: { $half_len },  Duración del medio tiempo: { $half_time_len }
    Muerte súbita permitida: { $sd_allowed },  Tiempo Extra Permitido: { $ot_allowed }
team-timeouts-per-half = Tiempos muertos de equipo permitidos por mitad: { $team_timeouts }
team-timeouts-per-game = Tiempos muertos de equipo permitidos por juego: { $team_timeouts }
stop-clock-last-2 = Detener reloj en los Últimos 2 minutos: { $stop_clock }
ref-list = Árbitro principal: { $chief_ref }
    Timer: { $timer }
    Árbitro de agua 1: { $water_ref_1 }
    Árbitro de agua 2: { $water_ref_2 }
    Árbitro de agua 3: { $water_ref_3 }
unknown = Desconocido
## Game time button
next-game = PRÓXIMO JUEGO
first-half = 1a MITAD
half-time = MEDIO TIEMPO
second-half = SEGUNDA MITAD
pre-ot-break-full = DESCANSO PREVIO AL TIEMPO EXTRA
overtime-first-half = PRIMERA MITAD DEL TIEMPO EXTRA
overtime-half-time = MEDIO TIEMPO DEL TIEMPO EXTRA
overtime-second-half = SEGUNDA MITAD DEL TIEMPO EXTRA
pre-sudden-death-break = DESCANSO PREVIO A MUERTE SÚBITA
sudden-death = MUERTE SÚBITA
ot-first-half = PRIMERA MITAD DEL TIEMPO EXTRA
ot-half-time = MEDIO TIEMPO DEL TIEMPO EXTRA
ot-2nd-half = SEGUNDA MITAD DEL TIEMPO EXTRA
white-timeout-short = T/M BLANCO
white-timeout-full = TIEMPO MUERTO BLANCO
black-timeout-short = T/M NEGRO
black-timeout-full = TIEMPO MUERTO NEGRO
ref-timeout-short = T/M ÁRBITRO
penalty-shot-short = TIRO PENAL
## Make penalty dropdown
infraction = INFRACCIÓN
## Make warning container
team-warning-abreviation = A

# Time edit
game-time = TIEMPO DE JUEGO
timeout = TIEMPO MUERTO
Note-Game-time-is-paused = Nota: El tiempo de juego está pausado mientras está en esta pantalla

# Warning Fouls Summary
fouls = FALTAS
edit-warnings = EDITAR AVISO
edit-fouls = EDITAR FALTAS

# Warnings
black-warnings = AVISOS E. NEGRO
white-warnings = AVISOS E. BLANCO

# Message
player-number = NÚMERO
    DE JUGADOR:
game-number = NÚMERO
    DE JUEGO:
num-tos-per-half = NÚMERO DE T/Ms
    POR MITAD:
num-tos-per-game = NÚMERO DE T/Ms
    POR JUEGO:

# Sound Controller - mod
off = NO
low = BAJO
medium = MEDIO
high = ALTO
max = MÁX

# Config
hockey6v6 = HOCKEY 6vs6
hockey3v3 = HOCKEY 3vs3
rugby = RUGBY

# Infractions
stick-foul = Infracción con el palo
illegal-advance = Uso de la mano libre
sub-foul = Substitución ilegal
illegal-stoppage = Parada ilegal
out-of-bounds = Pastilla fuera
grabbing-the-wall = agarrarse con barreras
obstruction = Obstruir
delay-of-game = Quemar tiempo
unsportsmanlike = Conducta antideportiva
free-arm = Uso ilegal del brazo libre
false-start = Saque nulo

