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
    TEMPO DE EQUIPA:
team-timeout-count = NÚMERO DE
    TEMPOS DE EQUIPA:

# Adicionar Aviso
team-warning-line-1 = AVISO DE
team-warning-line-2 = EQUIPA
team-score-line-1 = RESULTADO DE
team-score-line-2 = EQUIPA

# Configuração
none-selected = Nenhum Selecionado
loading = A carregar...
game-select = JOGO:
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
player-display-brightness = BRILHO DO
    ECRÃ DE JOGADORES
confirm-score-at-game-end = CONFIRMAR RESULTADO
    NO FIM DO JOGO
track-cap-number-of-scorer = REGISTAR NÚMERO
    DE TOUCA DO MARCADOR
event = EVENTO:
track-fouls-and-warnings = REGISTAR FALTAS
    E AVISOS
show-behind-schedule-time = MOSTRAR ATRASO
show-countdown-for-last-10-seconds = MOSTRAR CONTAGEM
    DECRESCENTE 10 S
audible-countdown-for-last-10-seconds = CONTAGEM DECRESCENTE
    SONORA 10 S
delay = ATRASO
court = CAMPO:
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
manual-games = JOGOS MANUAIS:
source-portal = { $portal }PORTAL
source-custom = PERSONALIZADO
access-token = TOKEN DE ACESSO:
starting-sides = LADOS INICIAIS
sound-enabled = SOM
    ATIVADO:
whistle-volume = VOLUME DO
    APITO:
manage-remotes = GERIR CONTROLOS REMOTOS
update-audio-output = ATUALIZAR SAÍDA
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
test = TESTAR
or-press-spacebar = Ou Prima a Barra de Espaços
or-hold-spacebar = Ou Mantenha a Barra de Espaços
game-info = INFORMAÇÕES
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
game-block = BLOCO DE JOGO
game-block-full = BLOCO DE JOGO:
game-block-help = Tempo desde o início de um jogo até ao início do seguinte
game-block-too-short = Demasiado curto para o jogo mais a pausa mínima
game-block-tight = Apertado — os tempos mortos podem fazer os jogos ultrapassar o seu slot
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
portal-login-code = CÓDIGO
portal-login-instructions = Aceda ao Portal { $portal } >> Gestão de Eventos >> Gestão de Árbitros, clique no botão + para adicionar um novo Refbox e introduza este ID de Refbox:
    { $id }

    O Portal { $portal } fornecerá então um código de confirmação para introduzir à esquerda através do teclado numérico.
    Prima Concluído depois de ter introduzido o código


# Confirmação
game-configuration-can-not-be-changed = A configuração do jogo não pode ser alterada enquanto um jogo está em curso.

    O que pretende fazer?
apply-this-game-number-change = Como pretende aplicar esta alteração ao número de jogo?
apply-switch-to-manual = Mudar para modo manual irá limpar o calendário carregado e repor o tempo antes do próximo jogo. Um jogo está em curso.
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
cancel-timeout = { cancel } { timeout }
cancel-timeout-line-1 = { cancel }
cancel-timeout-line-2 = { timeout }
cancel-ref-timeout = { cancel } { ref } { timeout }
cancel-ref-timeout-line-1 = { cancel } { ref }
cancel-ref-timeout-line-2 = { timeout }
cancel-pen-shot = { cancel } { pen-shot }
cancel-pen-shot-line-1 = { cancel }
cancel-pen-shot-line-2 = { pen-shot }
switch-to = MUDAR PARA
ref = ÁRBITRO
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
revive-hold-line-1 = SEGURE PARA
revive-hold-line-2 = RESTAURAR
revive-deciding-line-2 = RESTAURADO
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
team-timeouts-label = TEMPOS DE
    EQUIPA:
unknown = Desconhecido
select-infraction = Selecione uma opção
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
beep-test-top-time-label = TEMPO
beep-test-top-level-label = NÍVEL
beep-test-top-lap-label = VOLTA
beep-test-start = INICIAR
beep-test-pause = PAUSA
beep-test-resume = RETOMAR
beep-test-reset = REINICIAR
beep-test-edit-selected = Nível { $level }
beep-test-edit-time = TEMPO
beep-test-edit-count = CONT
beep-test-edit-new = ADICIONAR NÍVEL
beep-test-edit-remove = REMOVER NÍVEL
beep-test-preset-ref = ÁRB

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
portal-retry-all = REPETIR TUDO
portal-row-token-expired = Sessão do portal expirou — toque para iniciar sessão novamente
portal-row-stuck = Jogo { $game } Erro no envio do resultado, toque para corrigir
portal-row-pending = Jogo { $game } Resultado não enviado, toque para tentar novamente
portal-row-stats-pending = Jogo { $game } Estatísticas não enviadas, toque para tentar novamente
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

# Self-update / Updates page
check-version = Verificar versão
updates-current-version = Versão atual
updates-check-for-updates = Procurar atualizações
updates-install = Instalar
updates-do-revert = Reverter
updates-install-note = Clicar em Instalar irá transferir e instalar a atualização e reiniciar a refbox
updates-revert-note = Clicar em Reverter irá restaurar a versão anterior e reiniciar a refbox
updates-unknown = Desconhecido
updates-checking = A verificar…
updates-up-to-date = Está atualizado.
updates-available = Atualização disponível: {$version}
updates-downloading = A transferir…
updates-verifying = A verificar a transferência…
updates-installing = A instalar…
updates-restarting = A reiniciar…
updates-confirm-revert = Voltar à versão anterior ({$version})?
updates-rolled-back = Revertido para a versão anterior porque a atualização não arrancou corretamente, tente novamente.
updates-revert = Voltar à versão anterior ({$version})
updates-error-no-internet = Não foi possível contactar o servidor de atualizações, verifique a sua ligação à internet
updates-error-bad-download = A atualização transferida não era válida e não foi instalada.
updates-error-rate-limited = O servidor de atualizações está ocupado, tente novamente daqui a pouco.
updates-error-no-space = Espaço livre insuficiente para instalar a atualização.
updates-error-not-writable = Não foi possível guardar a atualização (permissão negada).

# Game-info table labels
gi-prior-game = Último Jogo
gi-team-light = { -light-team-name }
gi-team-dark = { -dark-team-name }
gi-current-game = Jogo atual
gi-next-game = Próximo Jogo
gi-game-block = Bloco de Jogo
gi-half-length = Duração do Tempo
gi-half-time-length = Duração do Intervalo
gi-game-length = Duração do Jogo
gi-timeouts = Tempos
gi-timeout-duration = Duração do Tempo
gi-overtime = Prorrogação
gi-sudden-death = Morte Súbita
gi-pre-overtime-break = Pausa Pré-Prorrogação
gi-pre-sudden-death-break = Pausa Pré-Morte Súbita
gi-overtime-half-length = Duração do Tempo de Prorrogação
gi-overtime-half-time-length = Duração do Intervalo de Prorrogação
gi-minimum-game-break = Tempo Mínimo Entre Jogos
gi-stop-clock-last-2 = Parar Relógio nos Últimos 2 Minutos
gi-ref-chief = Árbitro Principal
gi-ref-timekeeper = Controlador de Tempo
gi-ref-timekeeper-helper = Assistente de Tempo
gi-ref-water-1 = Árbitro Aquático 1
gi-ref-water-2 = Árbitro Aquático 2
gi-ref-water-3 = Árbitro Aquático 3
gi-ref-water-referees = Árbitros Aquáticos
gi-ref-deck-referees = Árbitros de Bordo
gi-unknown = ???

shut-down = DESLIGAR
restart-pi = REINICIAR PI
restart-refbox = REINICIAR REFBOX
