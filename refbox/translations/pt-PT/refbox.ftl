# Definitions for the translation file to use
-dark-team-name = Preto
dark-team-name-caps = PRETO

-light-team-name = Branco
light-team-name-caps = BRANCO

# Multipage
done = CONCLUÍDO
restart-to-apply = REINICIAR PARA APLICAR
cancel = CANCELAR
delete = ELIMINAR
back = VOLTAR
new = NOVO

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
timeout-length = DURAÇÃO
    TEMPO MORTO

# Warning Add
team-warning = AVISO DE
    EQUIPA
team-warning-line-1 = AVISO DE
team-warning-line-2 = EQUIPA

# Configuration
none-selected = Nenhum Selecionado
loading = A carregar...
game-select = Jogo:
game-options = OPÇÕES DE JOGO
app-options = OPÇÕES DA APP
display-options = OPÇÕES DE ECRÃ
sound-options = OPÇÕES DE SOM
app-mode = MODO DA
    APP
hide-time-for-last-15-seconds = OCULTAR TEMPO NOS
    ÚLTIMOS 15 SEG
player-display-brightness = BRILHO DO
    ECRÃ DE JOGADORES
confirm-score-at-game-end = CONFIRMAR PONTUAÇÃO
    NO FIM DO JOGO
track-cap-number-of-scorer = REGISTAR NÚMERO
    DE TOUCA DO MARCADOR
event = EVENTO:
track-fouls-and-warnings = REGISTAR FALTAS
    E AVISOS
court = CAMPO:
single-half = PERÍODO
    ÚNICO:
half-length-full = DURAÇÃO DO PERÍODO:
game-length = DURAÇÃO DO JOGO:
overtime-allowed = PROLONGAMENTO
    PERMITIDO:
sudden-death-allowed = MORTE SÚBITA
    PERMITIDA:
half-time-length = DURAÇÃO DO
    INTERVALO:
pre-ot-break-length = PAUSA PRÉ
    PROLONGAMENTO:
pre-sd-break-length = PAUSA PRÉ
    MORTE SÚBITA:
nominal-break-between-games = PAUSA NOMINAL
    ENTRE JOGOS:
ot-half-length = DURAÇÃO PERÍODO
    PROLONGAMENTO:
timeouts-counted-per = TEMPOS MORTOS
    CONTADOS POR:
game = JOGO
half = PERÍODO
minimum-brk-btwn-games = PAUSA MÍN
    ENTRE JOGOS:
ot-half-time-length = INTERVALO DO
    PROLONGAMENTO
using-uwh-portal = USAR UWHPORTAL:
starting-sides = LADOS INICIAIS
sound-enabled = SOM
    ATIVADO:
whistle-volume = VOLUME DO
    APITO:
manage-remotes = GERIR CONTROLOS REMOTOS
whistle-enabled = APITO
    ATIVADO:
above-water-volume = VOLUME
    ACIMA DA ÁGUA:
auto-sound-start-play = SOM AUTO
    INICIAR JOGO:
buzzer-sound = SOM DA
    BUZINA:
underwater-volume = VOLUME
    DEBAIXO DE ÁGUA:
auto-sound-stop-play = SOM AUTO
    PARAR JOGO:
alarm-button = BOTÃO DE
    ALARME:
alarm = ALARME
hold-to-test = MANTER PRESSIONADO PARA TESTAR
or-press-spacebar = Ou Prima a Barra de Espaços
or-hold-spacebar = Ou Mantenha a Barra de Espaços
game-info = INFO DO JOGO
remotes = CONTROLOS REMOTOS
default = PREDEFINIÇÃO
sound = SOM: { $sound_text }
brightness = { $brightness ->
        *[Low] BAIXO
        [Medium] MÉDIO
        [High] ALTO
        [Outdoor] EXTERIOR
    }

waiting = A AGUARDAR
add = ADICIONAR
half-length = DUR PERÍODO
length-of-half-during-regular-play = A duração de um período durante o jogo regular
half-time-lenght = DUR INTERVALO
length-of-half-time-period = A duração do período de intervalo
nom-break = PAUSA NOM
system-will-keep-game-times-spaced = O sistema tentará manter os horários de início dos jogos espaçados de forma uniforme, sendo o tempo total de um início ao seguinte igual a 2 × [Duração do Período] + [Duração do Intervalo] + [Tempo Nominal Entre Jogos] (exemplo: se [Duração do Período] = 15 min, [Duração do Intervalo] = 3 min e [Tempo Nominal Entre Jogos] = 12 min, o tempo de início a início será de 45 min. Os tempos mortos ou outras paragens reduzirão os 12 min até ser atingido o tempo mínimo entre jogos).
min-break = PAUSA MÍN
min-time-btwn-games = Se um jogo durar mais do que o previsto, este é o tempo mínimo entre jogos que o sistema atribuirá. Se os jogos ficarem atrasados, o sistema tentará recuperar nos jogos seguintes, respeitando sempre este tempo mínimo.
pre-ot-break-abreviated = PAUSA PRÉ PROL
pre-sd-brk = Se o prolongamento estiver ativado e for necessário, esta é a duração da pausa entre o Segundo Período e o Primeiro Período do Prolongamento
ot-half-len = DUR PERÍODO PROL
time-during-ot = A duração de um período durante o prolongamento
ot-half-tm-len = DUR INT PROL
len-of-overtime-halftime = A duração do intervalo do prolongamento
pre-sd-break = PAUSA PRÉ MD
pre-sd-len = A duração da pausa entre o período de jogo anterior e a Morte Súbita
language = IDIOMA
this-language = PORTUGUÊS
portal-login-code = CÓDIGO
portal-login-instructions = Aceda ao Portal UWH >> Gestão de Eventos >> Gestão de Árbitros, clique no botão + para adicionar um novo Refbox e introduza este ID de Refbox:
    { $id }

    O Portal UWH fornecerá então um código de confirmação para introduzir à esquerda através do teclado numérico.
    Prima Concluído depois de ter introduzido o código

help = AJUDA:

# Confirmation
game-configuration-can-not-be-changed = A configuração do jogo não pode ser alterada enquanto um jogo está em curso.

    O que pretende fazer?
apply-this-game-number-change = Como pretende aplicar esta alteração ao número de jogo?
UWHPortal-enabled = Quando o UWHPortal está ativado, todos os campos têm de ser preenchidos.
uwhportal-token-invalid-code = Código introduzido inválido.
    Tente novamente.
uwhportal-token-no-pending-link = O portal não está à espera de uma ligação.
    Tente novamente.
go-back-to-editor = VOLTAR AO EDITOR
discard-changes = DESCARTAR ALTERAÇÕES
end-current-game-and-apply-changes = TERMINAR JOGO ATUAL E APLICAR ALTERAÇÕES
end-current-game-and-apply-change = TERMINAR JOGO ATUAL E APLICAR ALTERAÇÃO
keep-current-game-and-apply-change = MANTER JOGO ATUAL E APLICAR ALTERAÇÃO
ok = OK
confirm-score = Esta pontuação está correta?
    Confirme com o árbitro principal.

    Preto: { $score_black }        Branco: { $score_white }

    { confirmation-count-down }
yes = SIM
no = NÃO

# Fouls
equal = IGUAL

# Game Info
refresh = ATUALIZAR
refreshing = A ATUALIZAR...
settings = DEFINIÇÕES
none = Nenhum
game-number-error = Erro ({ $game_number })
next-game-number-error = Erro ({ $next_game_number })
last-game-next-game = Último Jogo: { $prev_game },
    Próximo Jogo: { $next_game }
black-team-white-team = Equipa Preta: { $black_team }
    Equipa Branca: { $white_team }
game-length-ot-allowed = Duração do Período: { $half_length }
         Duração do Intervalo: { $half_time_length }
         Prolongamento Permitido: { $overtime }
overtime-details = Duração da Pausa Pré-Prolongamento: { $pre_overtime }
             Duração do Período de Prolongamento: { $overtime_len }
             Duração do Intervalo de Prolongamento: { $overtime_half_time_len }
sd-allowed = Morte Súbita Permitida: { $sd }
pre-sd = Duração da Pausa Pré-Morte Súbita: { $pre_sd_len }
team-to-len = Duração do Tempo Morto de Equipa: { $to_len }
time-btwn-games = Tempo Nominal Entre Jogos: { $time_btwn }
min-brk-btwn-games = Tempo Mínimo Entre Jogos: { $min_brk_time }


# List Selecters
select-event = SELECIONAR EVENTO
select-court = SELECIONAR CAMPO
select-game = SELECIONAR JOGO

# Main View
add-warning = ADICIONAR AVISO
add-foul = ADICIONAR FALTA
start-now = INICIAR AGORA
end-timeout = TERMINAR TEMPO MORTO
warnings = AVISOS
penalties = PENALIDADES
dark-score-line-1 = PONTUAÇÃO
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = PONTUAÇÃO
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = PENALIDADES PRETO
white-penalties = PENALIDADES BRANCO

# Score edit
final-score = Introduza a pontuação final
confirmation-count-down = Nota: A pontuação inalterada será confirmada automaticamente em { $countdown }

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = TERMINAR
end-timeout-line-2 = { timeout }
switch-to = MUDAR PARA
ref = ÁRBITRO
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = GRANDE
penalty-shot-line-2 = PENALIDADE
pen-shot = GR PENAL
## Penalty string
served = Cumprida
penalty = #{$player_number} - {$time ->
        [pending] Pendente
        [served] Cumprida
        [total-dismissal] Expulso
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
infraction = Infração: {$infraction}
## Config String
error = Erro ({ $number })
two-games = Último Jogo: { $prev_game },  Próximo Jogo: { $next_game }
one-game = Jogo: { $game }
teams = Equipa { -dark-team-name }: { $dark_team }
    Equipa { -light-team-name }: { $light_team }
game-config = Duração do Período: { $half_len },  Duração do Intervalo: { $half_time_len }
    Morte Súbita Permitida: { $sd_allowed },  Prolongamento Permitido: { $ot_allowed }
team-timeouts-per-half = Tempos Mortos de Equipa Permitidos Por Período: { $team_timeouts }
team-timeouts-per-game = Tempos Mortos de Equipa Permitidos Por Jogo: { $team_timeouts }
stop-clock-last-2 = Parar Relógio nos Últimos 2 Minutos: { $stop_clock }
ref-list = Árbitro Principal: { $chief_ref }
    Cronometrista: { $timer }
    Árbitro de Água 1: { $water_ref_1 }
    Árbitro de Água 2: { $water_ref_2 }
    Árbitro de Água 3: { $water_ref_3 }
team-ref-list = Árbitros: { $ref_team }
    Cronometrista/Marcador: { $ts_keeper_team }
unknown = Desconhecido
## Game time button
next-game = PRÓXIMO JOGO
first-half = PRIMEIRO PERÍODO
half-time = INTERVALO
second-half = SEGUNDO PERÍODO
pre-ot-break-full = PAUSA PRÉ-PROLONGAMENTO
overtime-first-half = PROLONGAMENTO PRIMEIRO PERÍODO
overtime-half-time = INTERVALO PROLONGAMENTO
overtime-second-half = PROLONGAMENTO SEGUNDO PERÍODO
pre-sudden-death-break = PAUSA PRÉ-MORTE SÚBITA
sudden-death = MORTE SÚBITA
ot-first-half = PROL 1.º PERÍODO
ot-half-time = PROL INTERVALO
ot-2nd-half = PROL 2.º PERÍODO
white-timeout-short = BRA T/M
white-timeout-full = TEMPO MORTO BRANCO
black-timeout-short = PRE T/M
black-timeout-full = TEMPO MORTO PRETO
ref-timeout-short = ARB T/M
penalty-shot-short = GR PENAL
## Make warning container
team-warning-abreviation = E
## Make time editor
zero = ZERO

# Time edit
game-time = TEMPO DE JOGO
timeout = TEMPO MORTO
Note-Game-time-is-paused = Nota: O tempo de jogo está pausado neste ecrã

# Warning Fouls Summary
fouls = FALTAS
edit-warnings = EDITAR AVISOS
edit-fouls = EDITAR FALTAS

# Warnings
black-warnings = AVISOS PRETO
white-warnings = AVISOS BRANCO

# Message
player-number = NÚMERO DO
    JOGADOR:
game-number = NÚMERO DO
    JOGO:
num-tos-per-half = N.º T/M
    POR PERÍODO:
num-tos-per-game = N.º T/M
    POR JOGO:

# Sound Controller - mod
off = DESLIGADO
low = BAIXO
medium = MÉDIO
high = ALTO
max = MÁX

# Config
hockey6v6 = HÓQUEI 6C6
hockey3v3 = HÓQUEI 3C3
rugby = RÂGUEBI

# Infractions
stick-foul = Falta com Stick
illegal-advance = Avanço Ilegal
sub-foul = Falta de Substituição
illegal-stoppage = Paragem Ilegal
out-of-bounds = Fora dos Limites
grabbing-the-wall = Agarrar a Parede
obstruction = Obstrução
delay-of-game = Demora de Jogo
unsportsmanlike = Conduta Antidesportiva
free-arm = Braço Livre
false-start = Saída Falsa
