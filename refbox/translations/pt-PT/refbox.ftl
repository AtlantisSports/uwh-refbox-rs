# Definições para o ficheiro de tradução
-dark-team-name = Preta
dark-team-name-caps = PRETA

-light-team-name = Branca
light-team-name-caps = BRANCA

# Várias páginas
done = CONCLUÍDO
restart-to-apply = REINICIAR PARA APLICAR
cancel = CANCELAR
delete = ELIMINAR
back = VOLTAR
apply = APLICAR
save = GUARDAR
user-options = OPÇÕES DO UTILIZADOR
new = NOVO

# Edição de Penalidade
total-dismissal = ED
penalty-kind = {$kind ->
    [thirty-seconds] 30s
    [one-minute] 1min
    [two-minutes] 2min
    [four-minutes] 4min
    [five-minutes] 5min
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# Edição de Tempo de Equipa
timeout-length = DURAÇÃO DO
    TEMPO DE EQUIPA

# Adicionar Aviso
team-warning = AVISO DE
    EQUIPA
team-warning-line-1 = AVISO DE
team-warning-line-2 = EQUIPA

# Configuração
none-selected = Nenhum Selecionado
loading = A carregar...
game-select = Jogo:
game-options = OPÇÕES DE JOGO
app-options = OPÇÕES DA APP
display-options = OPÇÕES DE ECRÃ
open-new-display = ABRIR NOVO ECRÃ
front-display-layout = DISPLAY LAYOUT
layout-default = DEFAULT
layout-classic = CLASSIC
layout-big-time = BIG TIME
layout-corners = CORNERS
layout-scores-only = SCORES ONLY
sound-options = OPÇÕES DE SOM
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
sound-settings = DEFINIÇÕES DE SOM
beep-test-edit-levels = EDITAR NÍVEIS
app-mode = MODO DA
    APP
hide-time-for-last-15-seconds = OCULTAR TEMPO NOS
    ÚLTIMOS 15 SEG
player-display-brightness = BRILHO DO
    ECRÃ DE JOGADORES
confirm-score-at-game-end = CONFIRMAR RESULTADO
    NO FIM DO JOGO
track-cap-number-of-scorer = REGISTAR NÚMERO
    DE TOUCA DO MARCADOR
event = EVENTO:
track-fouls-and-warnings = REGISTAR FALTAS
    E AVISOS
court = CAMPO:
single-half = TEMPO
    ÚNICO:
half-length-full = DURAÇÃO DO TEMPO:
game-length = DURAÇÃO DO JOGO:
overtime-allowed = PRORROGAÇÃO
    PERMITIDA:
sudden-death-allowed = MORTE SÚBITA
    PERMITIDA:
half-time-length = DURAÇÃO DO
    INTERVALO:
pre-ot-break-length = PAUSA PRÉ
    PRORROGAÇÃO:
pre-sd-break-length = PAUSA PRÉ
    MORTE SÚBITA:
nominal-break-between-games = PAUSA NOMINAL
    ENTRE JOGOS:
ot-half-length = DURAÇÃO TEMPO
    PRORROGAÇÃO:
timeouts-counted-per = TEMPOS DE EQUIPA
    CONTADOS POR:
game = JOGO
half = TEMPO
minimum-brk-btwn-games = PAUSA MÍN
    ENTRE JOGOS:
ot-half-time-length = INTERVALO DA
    PRORROGAÇÃO
using-portal = USAR { $portal }PORTAL:
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
half-length = DUR TEMPO
length-of-half-during-regular-play = A duração de um tempo durante o jogo regular
half-time-lenght = DUR INTERVALO
length-of-half-time-period = A duração do período de intervalo
nom-break = PAUSA NOM
system-will-keep-game-times-spaced = O sistema tentará manter os horários de início dos jogos espaçados de forma uniforme, sendo o tempo total de um início ao seguinte igual a 2 × [Duração do Tempo] + [Duração do Intervalo] + [Tempo Nominal Entre Jogos] (exemplo: se [Duração do Tempo] = 15 min, [Duração do Intervalo] = 3 min e [Tempo Nominal Entre Jogos] = 12 min, o tempo de início a início será de 45 min. Os tempos de equipa ou outras paragens reduzirão os 12 min até ser atingido o tempo mínimo entre jogos).
min-break = PAUSA MÍN
min-time-btwn-games = Se um jogo durar mais do que o previsto, este é o tempo mínimo entre jogos que o sistema atribuirá. Se os jogos ficarem atrasados, o sistema tentará recuperar nos jogos seguintes, respeitando sempre este tempo mínimo.
pre-ot-break-abreviated = PAUSA PRÉ PRORR
pre-sd-brk = Se a prorrogação estiver ativada e for necessária, esta é a duração da pausa entre o Segundo Tempo e o Primeiro Tempo da Prorrogação
ot-half-len = DUR TEMPO PRORR
time-during-ot = A duração de um tempo durante a prorrogação
ot-half-tm-len = DUR INT PRORR
len-of-overtime-halftime = A duração do intervalo da prorrogação
pre-sd-break = PAUSA PRÉ MS
pre-sd-len = A duração da pausa entre o período de jogo anterior e a Morte Súbita
language = IDIOMA
this-language = PORTUGUÊS
portal-login-code = CÓDIGO
portal-login-instructions = Aceda ao Portal { $portal } >> Gestão de Eventos >> Gestão de Árbitros, clique no botão + para adicionar um novo Refbox e introduza este ID de Refbox:
    { $id }

    O Portal { $portal } fornecerá então um código de confirmação para introduzir à esquerda através do teclado numérico.
    Prima Concluído depois de ter introduzido o código

help = AJUDA:

# Confirmação
game-configuration-can-not-be-changed = A configuração do jogo não pode ser alterada enquanto um jogo está em curso.

    O que pretende fazer?
apply-this-game-number-change = Como pretende aplicar esta alteração ao número de jogo?
portal-enabled = Quando o { $portal }PORTAL está ativado, todos os campos têm de ser preenchidos.
mode-switch-portal-tenant = Alterar o modo de { $from_mode } para { $to_mode } desativará a ligação ao { $from_portal }PORTAL e terá de se ligar novamente ao { $to_portal }PORTAL.
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
confirm-score = Este resultado está correto?
    Confirme com o árbitro principal.

    Preta: { $score_black }        Branca: { $score_white }

    { confirmation-count-down }
yes = SIM
no = NÃO

# Faltas
equal = IGUAL

# Informação do Jogo
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
game-length-ot-allowed = Duração do Tempo: { $half_length }
         Duração do Intervalo: { $half_time_length }
         Prorrogação Permitida: { $overtime }
overtime-details = Duração da Pausa Pré-Prorrogação: { $pre_overtime }
             Duração do Tempo de Prorrogação: { $overtime_len }
             Duração do Intervalo de Prorrogação: { $overtime_half_time_len }
sd-allowed = Morte Súbita Permitida: { $sd }
pre-sd = Duração da Pausa Pré-Morte Súbita: { $pre_sd_len }
team-to-len = Duração do Tempo de Equipa: { $to_len }
time-btwn-games = Tempo Nominal Entre Jogos: { $time_btwn }
min-brk-btwn-games = Tempo Mínimo Entre Jogos: { $min_brk_time }


# Seletores de Lista
select-event = SELECIONAR EVENTO
select-court = SELECIONAR CAMPO
select-game = SELECIONAR JOGO

# Vista Principal
add-warning = ADICIONAR AVISO
add-foul = ADICIONAR FALTA
start-now = INICIAR AGORA
end-timeout = TERMINAR TEMPO DE EQUIPA
warnings = AVISOS
penalties = PENALIDADES
dark-score-line-1 = RESULTADO
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = RESULTADO
light-score-line-2 = { light-team-name-caps }

# Penalidades
black-penalties = PENALIDADES PRETA
white-penalties = PENALIDADES BRANCA

# Edição de Resultado
final-score = Introduza o resultado final
confirmation-count-down = Nota: O resultado inalterado será confirmado automaticamente em { $countdown }

# Elementos Partilhados
## Faixa de tempo de equipa
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
penalty-shot-line-1 = TIRO DE
penalty-shot-line-2 = PENALIDADE
pen-shot = TIRO PENAL
## Cadeia de penalidade
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
## Cadeia de configuração
error = Erro ({ $number })
two-games = Último Jogo: { $prev_game },  Próximo Jogo: { $next_game }
one-game = Jogo: { $game }
teams = Equipa { -dark-team-name }: { $dark_team }
    Equipa { -light-team-name }: { $light_team }
game-config = Duração do Tempo: { $half_len },  Duração do Intervalo: { $half_time_len }
    Morte Súbita Permitida: { $sd_allowed },  Prorrogação Permitida: { $ot_allowed }
team-timeouts-per-half = Tempos de Equipa Permitidos Por Tempo: { $team_timeouts }
team-timeouts-per-game = Tempos de Equipa Permitidos Por Jogo: { $team_timeouts }
stop-clock-last-2 = Parar Relógio nos Últimos 2 Minutos: { $stop_clock }
ref-list = Árbitro Principal: { $chief_ref }
    Controlador de Tempo: { $timer }
    Árbitro Aquático 1: { $water_ref_1 }
    Árbitro Aquático 2: { $water_ref_2 }
    Árbitro Aquático 3: { $water_ref_3 }
team-ref-list = Árbitros: { $ref_team }
    Controlador de Tempo/Pontuação: { $ts_keeper_team }
unknown = Desconhecido
## Botão de tempo de jogo
next-game = PRÓXIMO JOGO
first-half = PRIMEIRO TEMPO
half-time = INTERVALO
second-half = SEGUNDO TEMPO
pre-ot-break-full = PAUSA PRÉ-PRORROGAÇÃO
overtime-first-half = PRORROGAÇÃO PRIMEIRO TEMPO
overtime-half-time = INTERVALO PRORROGAÇÃO
overtime-second-half = PRORROGAÇÃO SEGUNDO TEMPO
pre-sudden-death-break = PAUSA PRÉ-MORTE SÚBITA
sudden-death = MORTE SÚBITA
ot-first-half = PRORR 1.º TEMPO
ot-half-time = PRORR INTERVALO
ot-2nd-half = PRORR 2.º TEMPO
white-timeout-short = BRA T/E
white-timeout-full = TEMPO DE EQUIPA BRANCA
black-timeout-short = PRE T/E
black-timeout-full = TEMPO DE EQUIPA PRETA
ref-timeout-short = ARB T/E
penalty-shot-short = TIRO PENAL
## Contentor de aviso de equipa
team-warning-abreviation = A
## Editor de tempo
zero = = 0

# Edição de Tempo
game-time = TEMPO DE JOGO
timeout = TEMPO DE EQUIPA
Note-Game-time-is-paused = Nota: O tempo de jogo está pausado neste ecrã

# Resumo de Avisos e Faltas
fouls = FALTAS
edit-warnings = EDITAR AVISOS
edit-fouls = EDITAR FALTAS

# Avisos
black-warnings = AVISOS PRETA
white-warnings = AVISOS BRANCA

# Mensagem
player-number = NÚMERO DA
    TOUCA:
game-number = NÚMERO DO
    JOGO:
num-tos-per-half = N.º T/E
    POR TEMPO:
num-tos-per-game = N.º T/E
    POR JOGO:

# Controlador de Som - modo
off = DESLIGADO
low = BAIXO
medium = MÉDIO
high = ALTO
max = MÁX

# Configuração
hockey6v6 = HÓQUEI 6C6
hockey3v3 = HÓQUEI 3C3
rugby = RÂGUEBI
beep-test = BEEP TEST

# Beep-test screen
beep-test-pre = PRÉ
beep-test-top-time-label = TEMPO
beep-test-top-level-label = NÍVEL
beep-test-top-lap-label = VOLTA
beep-test-start = INICIAR
beep-test-pause = PAUSA
beep-test-resume = RETOMAR
beep-test-reset = REINICIAR
beep-test-column-level = NÍVEL
beep-test-column-count = CONT
beep-test-column-duration = DURAÇÃO
beep-test-edit-selected = Nível { $level }
beep-test-edit-time = TEMPO
beep-test-edit-count = CONT
beep-test-edit-new = ADICIONAR NÍVEL
beep-test-edit-remove = REMOVER NÍVEL

# Infrações
stick-foul = Falta de Taco
illegal-advance = Avanço Ilegal
sub-foul = Falta de Substituição
illegal-stoppage = Paragem Ilegal
out-of-bounds = Fora dos Limites
grabbing-the-wall = Agarrar a Parede
obstruction = Obstrução
delay-of-game = Atraso de Jogo
unsportsmanlike = Conduta Anti-Desportiva
free-arm = Braço Livre
false-start = Saída Falsa


# Portal Health Indicator
portal-summary-title = ESTADO DO PORTAL { $portal }
portal-row-token-expired = Sessão do portal expirou — toque para iniciar sessão novamente
portal-row-stuck = Jogo { $game } Erro no envio do resultado, toque para corrigir
portal-row-pending = Jogo { $game } Resultado não enviado, toque para tentar novamente
portal-row-recent = Jogo { $game } · Enviado há { $mins } min
portal-row-attempt-suffix = (tentativa { $attempts })
portal-action-force-submit = Tentar novamente este resultado
portal-action-discard = Descartar este resultado
portal-action-discard-confirm = TOQUE NOVAMENTE PARA CONFIRMAR DESCARTE
portal-page-title-attention = Erro no envio do Jogo { $game }
portal-page-attention-info = O resultado do jogo não foi aceite no Portal { $portal }
portal-page-attention-score = Resultado guardado: Branca { $white } - Preta { $black }
portal-page-attention-remediation = Pode Tentar Novamente se a ligação estiver verificada, ou descartar para limpar o erro
portal-advisory-at-game-end = Problema detetado no portal. O resultado será mantido em fila — contacte um administrador para resolver.

# 2 Halves / 1 Period selector (Half Length editor)
two-halves = 2 TEMPOS
one-period = 1 PERÍODO
game-len = DURAÇÃO DO JOGO
length-of-game-during-regular-play = A duração total do jogo durante o jogo regular
