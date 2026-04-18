# Definitions for the translation file to use
-dark-team-name = 흑
dark-team-name-caps = 흑팀

-light-team-name = 백
light-team-name-caps = 백팀

# Multipage
done = 완료
restart-to-apply = 재시작하여 적용
cancel = 취소
delete = 삭제
back = 뒤로
new = 새로

# Penalty Edit
total-dismissal = TD
penalty-kind = {$kind ->
    [thirty-seconds] 30초
    [one-minute] 1분
    [two-minutes] 2분
    [four-minutes] 4분
    [five-minutes] 5분
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# Team Timeout Edit
timeout-length = 팀 타임아웃
    길이

# Warning Add
team-warning = 팀
    경고
team-warning-line-1 = 팀
team-warning-line-2 = 경고

# Configuration
none-selected = 선택 안됨
loading = 로딩 중...
game-select = 경기:
game-options = 경기 옵션
app-options = 앱 옵션
display-options = 디스플레이 옵션
sound-options = 소리 옵션
app-mode = 앱
    모드
hide-time-for-last-15-seconds = 마지막 15초
    시간 숨기기
player-display-brightness = 선수 디스플레이
    밝기
confirm-score-at-game-end = 경기 종료 시
    점수 확인
track-cap-number-of-scorer = 득점자 캡
    번호 추적
event = 대회:
track-fouls-and-warnings = 파울 및
    경고 추적
court = 코트:
single-half = 단일
    전반:
half-length-full = 전반 길이:
game-length = 경기 길이:
overtime-allowed = 연장전
    허용:
sudden-death-allowed = 서든데스
    허용:
half-time-length = 하프타임
    길이:
pre-ot-break-length = 연장전 전
    휴식 길이:
pre-sd-break-length = 서든데스 전
    휴식 길이:
nominal-break-between-games = 경기 간
    기준 휴식:
ot-half-length = 연장 전반
    길이:
timeouts-counted-per = 타임아웃
    기준 단위:
game = 경기
half = 전반
minimum-brk-btwn-games = 최소 경기 간
    휴식:
ot-half-time-length = 연장 하프타임
    길이
using-uwh-portal = UWHPORTAL 사용:
starting-sides = 시작 진영
sound-enabled = 소리
    사용:
whistle-volume = 호루라기
    음량:
manage-remotes = 리모컨 관리
whistle-enabled = 호루라기
    사용:
above-water-volume = 수상
    음량:
auto-sound-start-play = 자동 소리
    플레이 시작:
buzzer-sound = 버저
    소리:
underwater-volume = 수중
    음량:
auto-sound-stop-play = 자동 소리
    플레이 중지:
alarm-button = 알람
    버튼:
alarm = 알람
hold-to-test = 길게 눌러 테스트
or-press-spacebar = 또는 스페이스바 누르기
or-hold-spacebar = 또는 스페이스바 길게 누르기
game-info = 경기 정보
remotes = 리모컨
default = 기본값
sound = 소리: { $sound_text }
brightness = { $brightness ->
        *[Low] 낮음
        [Medium] 중간
        [High] 높음
        [Outdoor] 야외
    }

waiting = 대기 중
add = 추가
half-length = 전반 길이
length-of-half-during-regular-play = 정규 경기 중 전반의 길이
half-time-lenght = 하프타임 길이
length-of-half-time-period = 하프타임 기간의 길이
nom-break = 기준 휴식
system-will-keep-game-times-spaced = 시스템은 경기 시작 시간을 균등하게 유지하려 합니다. 한 경기 시작부터 다음 경기 시작까지의 총 시간은 2 × [전반 길이] + [하프타임 길이] + [경기 간 기준 시간]입니다 (예: [전반 길이] = 15분, [하프타임 길이] = 3분, [경기 간 기준 시간] = 12분이면 한 경기 시작부터 다음 경기까지 45분. 타임아웃이나 기타 시계 정지 시 최소 경기 간 시간에 도달할 때까지 12분이 줄어듭니다).
min-break = 최소 휴식
min-time-btwn-games = 경기가 예정보다 길어질 경우, 시스템이 할당하는 경기 간 최소 시간입니다. 경기가 지연되면 시스템이 이후 경기에서 자동으로 따라잡으며 항상 이 최소 경기 간 시간을 준수합니다.
pre-ot-break-abreviated = 연장전 전 휴식
pre-sd-brk = 연장전이 허용되고 필요한 경우, 이것은 후반과 연장전 전반 사이의 휴식 길이입니다
ot-half-len = 연장 전반 길이
time-during-ot = 연장전 중 전반의 길이
ot-half-tm-len = 연장 하프타임 길이
len-of-overtime-halftime = 연장전 하프타임의 길이
pre-sd-break = 서든데스 전 휴식
pre-sd-len = 이전 경기 기간과 서든데스 사이의 휴식 길이
language = 언어
this-language = 한국어
portal-login-code = 코드
portal-login-instructions = UWH Portal >> 대회 관리 >> 심판 관리로 이동하여 + 버튼을 클릭해 새 Refbox를 추가하고 이 Refbox ID를 입력하세요:
    { $id }

    그러면 UWH Portal에서 확인 코드를 제공합니다. 왼쪽 숫자 패드로 코드를 입력하세요.
    코드를 입력한 후 완료를 누르세요

help = 도움말:

# Confirmation
game-configuration-can-not-be-changed = 경기 진행 중에는 경기 설정을 변경할 수 없습니다.

    어떻게 하시겠습니까?
apply-this-game-number-change = 이 경기 번호 변경을 어떻게 적용하시겠습니까?
UWHPortal-enabled = UWHPortal이 활성화된 경우 모든 필드를 입력해야 합니다.
uwhportal-token-invalid-code = 잘못된 코드가 입력되었습니다.
    다시 시도하세요.
uwhportal-token-no-pending-link = Portal이 연결을 기다리지 않습니다.
    다시 시도하세요.
go-back-to-editor = 편집기로 돌아가기
discard-changes = 변경 사항 버리기
end-current-game-and-apply-changes = 현재 경기 종료 및 변경 사항 적용
end-current-game-and-apply-change = 현재 경기 종료 및 변경 사항 적용
keep-current-game-and-apply-change = 현재 경기 유지 및 변경 사항 적용
ok = 확인
confirm-score = 이 점수가 맞습니까?
    수석 심판과 확인하세요.

    흑팀: { $score_black }        백팀: { $score_white }

    { confirmation-count-down }
yes = 예
no = 아니요

# Fouls
equal = 동등

# Game Info
refresh = 새로고침
refreshing = 새로고침 중...
settings = 설정
none = 없음
game-number-error = 오류 ({ $game_number })
next-game-number-error = 오류 ({ $next_game_number })
last-game-next-game = 이전 경기: { $prev_game },
    다음 경기: { $next_game }
black-team-white-team = 흑팀: { $black_team }
    백팀: { $white_team }
game-length-ot-allowed = 전반 길이: { $half_length }
         하프타임 길이: { $half_time_length }
         연장전 허용: { $overtime }
overtime-details = 연장전 전 휴식 길이: { $pre_overtime }
             연장 전반 길이: { $overtime_len }
             연장 하프타임 길이: { $overtime_half_time_len }
sd-allowed = 서든데스 허용: { $sd }
pre-sd = 서든데스 전 휴식 길이: { $pre_sd_len }
team-to-len = 팀 타임아웃 길이: { $to_len }
time-btwn-games = 경기 간 기준 시간: { $time_btwn }
min-brk-btwn-games = 경기 간 최소 시간: { $min_brk_time }


# List Selecters
select-event = 대회 선택
select-court = 코트 선택
select-game = 경기 선택

# Main View
add-warning = 경고 추가
add-foul = 파울 추가
start-now = 지금 시작
end-timeout = 타임아웃 종료
warnings = 경고
penalties = 페널티
dark-score-line-1 = 점수
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = 점수
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = 흑팀 페널티
white-penalties = 백팀 페널티

# Score edit
final-score = 최종 점수를 입력하세요
confirmation-count-down = 참고: 변경되지 않은 점수는 { $countdown } 후 자동으로 확인됩니다

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = 종료
end-timeout-line-2 = { timeout }
switch-to = 전환
ref = 심판
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = 페널티
penalty-shot-line-2 = 슛
pen-shot = 페널티 슛
## Penalty string
served = 집행됨
penalty = #{$player_number} - {$time ->
        [pending] 대기 중
        [served] 집행됨
        [total-dismissal] 퇴장됨
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
infraction = 위반: {$infraction}
## Config String
error = 오류 ({ $number })
two-games = 이전 경기: { $prev_game },  다음 경기: { $next_game }
one-game = 경기: { $game }
teams = { -dark-team-name }팀: { $dark_team }
    { -light-team-name }팀: { $light_team }
game-config = 전반 길이: { $half_len },  하프타임 길이: { $half_time_len }
    서든데스 허용: { $sd_allowed },  연장전 허용: { $ot_allowed }
team-timeouts-per-half = 전반당 팀 타임아웃 허용 횟수: { $team_timeouts }
team-timeouts-per-game = 경기당 팀 타임아웃 허용 횟수: { $team_timeouts }
stop-clock-last-2 = 마지막 2분 시계 정지: { $stop_clock }
ref-list = 수석 심판: { $chief_ref }
    타이머: { $timer }
    수중 심판 1: { $water_ref_1 }
    수중 심판 2: { $water_ref_2 }
    수중 심판 3: { $water_ref_3 }
team-ref-list = 심판: { $ref_team }
    타임키퍼/스코어키퍼: { $ts_keeper_team }
unknown = 알 수 없음
## Game time button
next-game = 다음 경기
first-half = 전반전
half-time = 하프타임
second-half = 후반전
pre-ot-break-full = 연장전 전 휴식
overtime-first-half = 연장 전반
overtime-half-time = 연장 하프타임
overtime-second-half = 연장 후반
pre-sudden-death-break = 서든데스 전 휴식
sudden-death = 서든데스
ot-first-half = 연장 1전반
ot-half-time = 연장 하프타임
ot-2nd-half = 연장 2후반
white-timeout-short = 백 타임아웃
white-timeout-full = 백팀 타임아웃
black-timeout-short = 흑 타임아웃
black-timeout-full = 흑팀 타임아웃
ref-timeout-short = 심판 타임아웃
penalty-shot-short = 페널티 슛
## Make warning container
team-warning-abreviation = 팀
## Make time editor
zero = 영

# Time edit
game-time = 경기 시간
timeout = 타임아웃
Note-Game-time-is-paused = 참고: 이 화면에 있는 동안 경기 시간이 일시 정지됩니다

# Warning Fouls Summary
fouls = 파울
edit-warnings = 경고 편집
edit-fouls = 파울 편집

# Warnings
black-warnings = 흑팀 경고
white-warnings = 백팀 경고

# Message
player-number = 선수
    번호:
game-number = 경기
    번호:
num-tos-per-half = 전반당 팀
    타임아웃 수:
num-tos-per-game = 경기당 팀
    타임아웃 수:

# Sound Controller - mod
off = 끄기
low = 낮음
medium = 중간
high = 높음
max = 최대

# Config
hockey6v6 = 하키 6대6
hockey3v3 = 하키 3대3
rugby = 럭비

# Infractions
stick-foul = 스틱 파울
illegal-advance = 불법 전진
sub-foul = 교체 파울
illegal-stoppage = 불법 정지
out-of-bounds = 아웃 오브 바운즈
grabbing-the-wall = 벽 잡기
obstruction = 방해
delay-of-game = 경기 지연
unsportsmanlike = 비스포츠맨십
free-arm = 프리 암
false-start = 부정 출발
