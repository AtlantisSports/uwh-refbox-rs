# Definitions for the translation file to use
-dark-team-name = Negro
dark-team-name-caps = NEGRO
-light-team-name = Blanco
light-team-name-caps = BLANCO

# Multipage
done = HECHO
restart-to-apply = REINICIAR PARA APLICAR
cancel = CANCELAR
delete = ELIMINAR
back = ATRÁS
apply = APLICAR
save = GUARDAR
user-options = OPCIONES DE USUARIO
new = NUEVO

# Penalty Edit
total-dismissal = T
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
timeout-length = DURACIÓN DEL
    TIEMPO DE ESPERA:
team-timeout-count = NÚMERO DE
    TIEMPOS DE ESPERA:

# Warning Add
team-warning = AVISO
    DE EQUIPO
team-warning-line-1 = AVISO
team-warning-line-2 = DE EQUIPO

# Configuration
none-selected = Ninguno seleccionado
loading = Cargando...
game-select = JUEGO:
game-options = OPCIONES DE JUEGO
app-options = OPCIONES DE APP
display-options = OPCIONES DE PANTALLA
open-new-display = ABRIR NUEVA PANTALLA
front-display-layout = DISPLAY LAYOUT
layout-default = DEFAULT
layout-classic = CLASSIC
layout-big-time = BIG TIME
layout-corners = CORNERS
layout-scores-only = SCORES ONLY
sound-options = OPCIONES DE SONIDO
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
sound-settings = AJUSTES DE SONIDO
beep-test-edit-levels = EDITAR NIVELES
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
show-behind-schedule-time = MOSTRAR RETRASO
delay = RETRASO
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
using-portal = USANDO { $portal }PORTAL:
starting-sides = LADOS INICIALES
sound-enabled = SONIDO
    HABILITADO:
whistle-volume = VOLUMEN
    DEL SILBATO:
manage-remotes = GESTIONAR REMOTOS
update-audio-output = ACTUALIZAR
    SALIDA DE AUDIO
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
alarm-button = BOTÓN DE
    ALARMA:
alarm = ALARMA
hold-to-test = MANTÉN PARA PROBAR
or-press-spacebar = O Presiona la Barra
or-hold-spacebar = O Mantén la Barra
game-info = INFORMACIÓN
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
game-block = BLOQUE DE JUEGO
game-block-full = BLOQUE DE JUEGO:
game-block-help = Tiempo desde el inicio de un partido hasta el inicio del siguiente
game-block-too-short = Demasiado corto para el partido más la pausa mínima
game-block-tight = Ajustado — los tiempos muertos podrían retrasar los partidos fuera de su franja
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
language = IDIOMA
this-language = ESPAÑOL
### Check
portal-login-code = Código de inicio de sesión
### Check
portal-login-instructions = Por favor, vaya a { $portal } Portal >> Gestión de Eventos >> Gestión de Árbitros, haga clic en el botón + para añadir un nuevo Refbox, e introduzca este ID de Refbox:
    { $id }

    El { $portal } Portal proporcionará un código de confirmación que deberá introducir a la izquierda utilizando el teclado numérico.
    Presione HECHO una vez que haya introducido el código.

help = AYUDA:

# Confirmation
game-configuration-can-not-be-changed = La configuración del juego no se puede cambiar mientras un juego está en progreso.

    ¿Qué te gustaría hacer?
apply-this-game-number-change = ¿Cómo te gustaría aplicar este cambio de número de juego?
apply-switch-to-manual = Cambiar al modo manual borrará el calendario cargado y restablecerá el tiempo antes del próximo juego. Un juego está en progreso.
portal-enabled = Cuando { $portal }PORTAL está habilitado, todos los campos deben ser completados.
mode-switch-portal-tenant = Cambiar el modo de { $from_mode } a { $to_mode } desactivará el enlace a { $from_portal }PORTAL y deberás volver a conectarte a { $to_portal }PORTAL.
### Check
uwhportal-token-invalid-code = Código inválido.
    Por favor, inténtelo de nuevo.
### Check
uwhportal-token-no-pending-link = No se encontró ningún enlace pendiente.
    Por favor, inténtelo de nuevo.
go-back-to-editor = VOLVER AL EDITOR
discard-changes = DESCARTAR CAMBIOS
end-current-game-and-apply-changes = TERMINAR EL JUEGO ACTUAL Y APLICAR CAMBIOS
end-current-game-and-apply-change = TERMINAR EL JUEGO ACTUAL Y APLICAR CAMBIO
keep-current-game-and-apply-change = MANTENER EL JUEGO ACTUAL Y APLICAR CAMBIO
ok = OK
confirm-score = ¿Es correcto este puntaje?
    Confirma con el árbitro principal.

    Negro: { $score_black }        Blanco: { $score_white }

    { confirmation-count-down }
yes = SI
no = NO

# Fouls
equal = IGUAL

# Game Info
refresh = RECARGAR
refreshing = RECARGANDO...
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
game-block-info = Bloque de Juego: { $game_block }
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
### Check
confirmation-count-down = Nota: La puntuación no modificada se confirmará automáticamente en { $countdown }

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = TERMINAR
end-timeout-line-2 = TIEMPO MUERTO
cancel-timeout = { cancel } { timeout }
cancel-timeout-line-1 = { cancel }
cancel-timeout-line-2 = { timeout }
cancel-ref-timeout = { cancel } { ref } { timeout }
cancel-ref-timeout-line-1 = { cancel } { ref }
cancel-ref-timeout-line-2 = { timeout }
cancel-pen-shot = { cancel } { pen-shot }
cancel-pen-shot-line-1 = { cancel }
cancel-pen-shot-line-2 = { pen-shot }
switch-to = CAMBIAR A
ref = ÁRBITRO
ref-timeout-line-1 = { timeout }
ref-timeout-line-2 = { ref }
dark-timeout-line-1 = { timeout }
dark-timeout-line-2 = E. { dark-team-name-caps }
light-timeout-line-1 = { timeout }
light-timeout-line-2 = E. { light-team-name-caps }
revive-hold-line-1 = MANTENER PARA
revive-hold-line-2 = RESTAURAR
revive-deciding-line-2 = RESTAURADO
penalty-shot-line-1 = TIRO
penalty-shot-line-2 = PENAL
pen-shot = TIRO PENAL
## Penalty string
served = Servido
penalty = #{$player_number} - {$time ->
        [pending] Pendiente
        [served] Servido
        [total-dismissal] Descartado
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
infraction = Infracción: {$infraction}
## Config String
error = Error ({ $number })
two-games = Último partido: { $prev_game }, Próximo partido: { $next_game }
one-game = Juego: { $game }
teams = { -dark-team-name } Equipo: { $dark_team }
    { -light-team-name } Equipo: { $light_team }
game-config = Duración de la mitad: { $half_len },  Duración del medio tiempo: { $half_time_len }
    Muerte súbita permitida: { $sd_allowed },  Tiempo Extra Permitido: { $ot_allowed }
team-timeouts = Tiempos muertos de equipo: { $value }
team-timeouts-label = TIEMPOS MUERTOS
    DE EQUIPO:
stop-clock-last-2 = Detener reloj en los Últimos 2 minutos: { $stop_clock }
ref-list = Árbitro principal: { $chief_ref }
    Timer: { $timer }
    Árbitro de agua 1: { $water_ref_1 }
    Árbitro de agua 2: { $water_ref_2 }
    Árbitro de agua 3: { $water_ref_3 }
team-ref-list = Árbitros: { $ref_team }
    Cronometrador/Marcador: { $ts_keeper_team }
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
## Make warning container
team-warning-abreviation = A
## Make time editor
zero = = 0

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
beep-test = PRUEBA DE PITIDOS

# Pantalla de prueba de pitidos
beep-test-pre = PRE
beep-test-top-time-label = TIEMPO
beep-test-top-level-label = NIVEL
beep-test-top-lap-label = VUELTA
beep-test-start = INICIAR
beep-test-pause = PAUSAR
beep-test-resume = REANUDAR
beep-test-reset = REINICIAR
beep-test-column-level = NIVEL
beep-test-column-count = CANTIDAD
beep-test-column-duration = DURACIÓN
beep-test-edit-selected = Nivel { $level }
beep-test-edit-time = TIEMPO
beep-test-edit-count = CANTIDAD
beep-test-edit-new = AÑADIR NIVEL
beep-test-edit-remove = ELIMINAR NIVEL

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


# Portal Health Indicator
portal-summary-title = ESTADO DE { $portal } PORTAL
portal-row-token-expired = Sesión del portal expirada — toca para iniciar sesión
portal-row-stuck = Juego { $game } · Error al enviar resultado, toca para corregir
portal-row-pending = Juego { $game } · Resultado no enviado, toca para reintentar
portal-row-attempt-suffix = (intento { $attempts })
portal-row-recent = Juego { $game } · Enviado hace { $mins } min
portal-action-force-submit = Reintentar este resultado
portal-action-discard = Descartar este resultado
portal-action-discard-confirm = TOCA DE NUEVO PARA CONFIRMAR DESCARTE
portal-page-title-attention = Juego { $game } · Error de envío
portal-page-attention-info = El resultado del juego no ha sido aceptado por { $portal } Portal
portal-page-attention-score = Resultado almacenado: Claro { $white } - Oscuro { $black }
portal-page-attention-remediation = Puedes Reintentar si la conexión está verificada, o descartar para borrar el error
portal-advisory-at-game-end = Problema del portal detectado. El resultado se encolará igualmente — busca a un administrador para resolverlo.

# 2 Halves / 1 Period selector (Half Length editor)
two-halves = 2 MITADES
one-period = 1 PERIODO
game-len = DURACIÓN DEL JUEGO
length-of-game-during-regular-play = La duración del juego durante el juego regular

# Single-period (1 Period) info variants — Game Length, no half-time line
game-config-single-half = Duración del juego: { $half_len }
    Muerte súbita permitida: { $sd_allowed },  Tiempo Extra Permitido: { $ot_allowed }
game-length-ot-allowed-single-half = Duración del juego: { $half_length }
         Tiempo extra habilitado: { $overtime }

# Self-update / Updates page
check-version = Comprobar versión
updates-current-version = Versión actual
updates-check-for-updates = Buscar actualizaciones
updates-install = Instalar
updates-do-revert = Revertir
updates-install-note = Al hacer clic en Instalar se descargará e instalará la actualización y se reiniciará la refbox
updates-revert-note = Al hacer clic en Revertir se restaurará la versión anterior y se reiniciará la refbox
updates-unknown = Desconocido
updates-checking = Comprobando…
updates-up-to-date = Está actualizado.
updates-available = Actualización disponible: {$version}
updates-downloading = Descargando…
updates-verifying = Comprobando la descarga…
updates-installing = Instalando…
updates-restarting = Reiniciando…
updates-confirm-revert = ¿Volver a la versión anterior ({$version})?
updates-rolled-back = Se ha vuelto a la versión anterior porque la actualización no se inició correctamente, inténtalo de nuevo.
updates-revert = Volver a la versión anterior ({$version})
updates-error-no-internet = No se pudo conectar con el servidor de actualizaciones, comprueba tu conexión a internet
updates-error-bad-download = La actualización descargada no era válida y no se instaló.
updates-error-rate-limited = El servidor de actualizaciones está ocupado, inténtalo de nuevo dentro de un rato.
updates-error-no-space = No hay suficiente espacio libre para instalar la actualización.
updates-error-not-writable = No se pudo guardar la actualización (permiso denegado).

# Game-info table labels
gi-prior-game = Último partido
gi-team-light = { -light-team-name }
gi-team-dark = { -dark-team-name }
gi-current-game = Partido actual
gi-next-game = Próximo Juego
gi-game-block = Bloque de Juego
gi-half-length = Duración de una mitad
gi-half-time-length = Duración del descanso de medio tiempo
gi-game-length = Duración del juego
gi-timeouts = Tiempos muertos
gi-timeout-duration = Duración del tiempo muerto
gi-overtime = Tiempo extra
gi-sudden-death = Muerte súbita
gi-pre-overtime-break = Descanso previo al tiempo extra
gi-pre-sudden-death-break = Descanso previo a la muerte súbita
gi-overtime-half-length = Duración de la mitad del tiempo extra
gi-overtime-half-time-length = Duración del medio tiempo extra
gi-minimum-game-break = Tiempo mínimo entre partidos
gi-stop-clock-last-2 = Detener reloj en los Últimos 2 minutos
gi-ref-chief = Árbitro principal
gi-ref-timekeeper = Timer
gi-ref-timekeeper-helper = Asistente de marcador
gi-ref-water-1 = Árbitro de agua 1
gi-ref-water-2 = Árbitro de agua 2
gi-ref-water-3 = Árbitro de agua 3
gi-ref-water-referees = Árbitros de agua
gi-ref-deck-referees = Árbitros de borde
gi-unknown = ???
