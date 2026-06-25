# 번역 파일 정의
# 주의: 이 번역 파일의 수중하키 전용 용어 중 다수는 [BEST-GUESS] 태그가 붙은 추정 번역입니다.
# 한국 수중하키 협회(KUA) 또는 원어민 검토자의 검토가 완료될 때까지 이 용어들은 미검증 상태입니다.
-dark-team-name = 검정
dark-team-name-caps = 검정 팀

-light-team-name = 흰
light-team-name-caps = 흰 팀

# 다중 페이지
done = 완료
restart-to-apply = 재시작하여 적용
cancel = 취소
delete = 삭제
back = 뒤로
apply = 적용
save = 저장
user-options = 사용자 옵션
new = 새로

# 페널티 편집
total-dismissal = 퇴장
penalty-kind = {$kind ->
    [thirty-seconds] 30초
    [one-minute] 1분
    [two-minutes] 2분
    [four-minutes] 4분
    [five-minutes] 5분
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# 팀 타임아웃 편집
timeout-length = 팀 타임아웃
    시간:
team-timeout-count = 팀 타임아웃
    횟수:

# 경고 추가
team-warning = 팀
    경고
team-warning-line-1 = 팀
team-warning-line-2 = 경고

# 설정
none-selected = 선택 없음
loading = 불러오는 중...
game-select = 경기:
game-options = 경기 옵션
app-options = 앱 옵션
display-options = 화면 옵션
open-new-display = 새 화면 열기
front-display-layout = DISPLAY LAYOUT
layout-default = DEFAULT
layout-classic = CLASSIC
layout-big-time = BIG TIME
layout-corners = CORNERS
layout-scores-only = SCORES ONLY
sound-options = 소리 옵션
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
sound-settings = 소리 설정
beep-test-edit-levels = 레벨 편집
app-mode = 앱
    모드
player-display-brightness = 선수 화면
    밝기
confirm-score-at-game-end = 경기 종료 시
    점수 확인
track-cap-number-of-scorer = 득점자 모자
    번호 추적
event = 대회:
track-fouls-and-warnings = 반칙 및
    경고 추적
show-behind-schedule-time = 지연 표시
show-countdown-for-last-10-seconds = 마지막 10초
    카운트다운 표시
audible-countdown-for-last-10-seconds = 마지막 10초 음성
    카운트다운
delay = 지연
court = 코트:
single-half = 단일
    전반:
half-length-full = 전반 시간:
game-length = 경기 시간:
overtime-allowed = 연장전
    허용:
sudden-death-allowed = 서든 데스
    허용:
half-time-length = 하프타임
    시간:
pre-ot-break-length = 연장전 전
    휴식 시간:
pre-sd-break-length = 서든 데스 전
    휴식 시간:
nominal-break-between-games = 경기 간
    기준 휴식:
ot-half-length = 연장 전반
    시간:
timeouts-counted-per = 타임아웃
    기준 단위:
game = 경기
half = 전반
minimum-brk-btwn-games = 최소 경기 간
    휴식:
ot-half-time-length = 연장 하프타임
    시간
using-portal = { $portal }PORTAL 사용:
starting-sides = 시작 진영
sound-enabled = 소리
    사용:
whistle-volume = 호루라기
    음량:
manage-remotes = 리모컨 관리
update-audio-output = 출력 새로고침
whistle-enabled = 호루라기
    사용:
above-water-volume = 수상
    음량:
auto-sound-start-play = 자동 소리
    경기 시작:
buzzer-sound = 버저
    소리:
underwater-volume = 수중
    음량:
auto-sound-stop-play = 자동 소리
    경기 종료:
alarm-button = 알람
    버튼:
alarm = 알람
hold-to-test = 길게 눌러 테스트
or-press-spacebar = 또는 스페이스바 누르기
or-hold-spacebar = 또는 스페이스바 길게 누르기
game-info = 정보
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
half-length = 전반 시간
length-of-half-during-regular-play = 정규 경기 중 전반의 시간
half-time-lenght = 하프타임 시간
length-of-half-time-period = 하프타임 기간의 시간
nom-break = 기준 휴식
game-block = 게임 블록
game-block-full = 게임 블록:
game-block-help = 한 경기 시작부터 다음 경기 시작까지의 시간
game-block-too-short = 경기와 최소 휴식을 담기에 너무 짧습니다
game-block-tight = 빡빡함 — 팀 타임아웃으로 경기가 슬롯을 초과할 수 있습니다
system-will-keep-game-times-spaced = 시스템은 경기 시작 시간을 균등하게 유지하려 합니다. 한 경기 시작부터 다음 경기 시작까지의 총 시간은 2 × [전반 시간] + [하프타임 시간] + [경기 간 기준 시간]입니다 (예: [전반 시간] = 15분, [하프타임 시간] = 3분, [경기 간 기준 시간] = 12분이면 한 경기 시작부터 다음 경기까지 45분. 타임아웃이나 기타 시계 정지 시 최소 경기 간 시간에 도달할 때까지 12분이 줄어듭니다).
min-break = 최소 휴식
min-time-btwn-games = 경기가 예정보다 길어질 경우, 시스템이 할당하는 경기 간 최소 시간입니다. 경기가 지연되면 시스템이 이후 경기에서 자동으로 따라잡으며 항상 이 최소 경기 간 시간을 준수합니다.
pre-ot-break-abreviated = 연장전 전 휴식
pre-sd-brk = 연장전이 허용되고 필요한 경우, 이것은 후반과 연장 전반 사이의 휴식 시간입니다
ot-half-len = 연장 전반 시간
time-during-ot = 연장전 중 전반의 시간
ot-half-tm-len = 연장 하프타임 시간
len-of-overtime-halftime = 연장전 하프타임의 시간
pre-sd-break = 서든 데스 전 휴식
pre-sd-len = 이전 경기 기간과 서든 데스 사이의 휴식 시간
language = 언어
this-language = 한국어
portal-login-code = 코드
portal-login-instructions = { $portal } Portal >> 대회 관리 >> 심판 관리로 이동하여 + 버튼을 클릭해 새 Refbox를 추가하고 이 Refbox ID를 입력하세요:
    { $id }

    그러면 { $portal } Portal에서 확인 코드를 제공합니다. 왼쪽 숫자 패드로 코드를 입력하세요.
    코드를 입력한 후 완료를 누르세요

help = 도움말:

# 확인
game-configuration-can-not-be-changed = 경기 진행 중에는 경기 설정을 변경할 수 없습니다.

    어떻게 하시겠습니까?
apply-this-game-number-change = 이 경기 번호 변경을 어떻게 적용하시겠습니까?
apply-switch-to-manual = 수동 모드로 전환하면 불러온 일정이 지워지고 다음 경기 전 시간이 초기화됩니다. 경기가 진행 중입니다.
portal-enabled = { $portal }PORTAL이 활성화된 경우 모든 필드를 입력해야 합니다.
mode-switch-portal-tenant = 모드를 { $from_mode }에서 { $to_mode }로 변경하면 { $from_portal }PORTAL 연결이 끊어지며 { $to_portal }PORTAL에 다시 연결해야 합니다.
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
    주심과 확인하세요.

    검정 팀: { $score_black }        흰 팀: { $score_white }

    { confirmation-count-down }
yes = 예
no = 아니요

# 반칙
equal = 동등

# 경기 정보
refresh = 새로고침
refreshing = 새로고침 중...
settings = 설정
none = 없음
game-number-error = 오류 ({ $game_number })
next-game-number-error = 오류 ({ $next_game_number })
last-game-next-game = 이전 경기: { $prev_game },
    다음 경기: { $next_game }
black-team-white-team = 검정 팀: { $black_team }
    흰 팀: { $white_team }
game-length-ot-allowed = 전반 시간: { $half_length }
         하프타임 시간: { $half_time_length }
         연장전 허용: { $overtime }
overtime-details = 연장전 전 휴식 시간: { $pre_overtime }
             연장 전반 시간: { $overtime_len }
             연장 하프타임 시간: { $overtime_half_time_len }
sd-allowed = 서든 데스 허용: { $sd }
pre-sd = 서든 데스 전 휴식 시간: { $pre_sd_len }
team-to-len = 팀 타임아웃 시간: { $to_len }
time-btwn-games = 경기 간 기준 시간: { $time_btwn }
game-block-info = 게임 블록: { $game_block }
min-brk-btwn-games = 경기 간 최소 시간: { $min_brk_time }


# 목록 선택기
select-event = 대회 선택
select-court = 코트 선택
select-game = 경기 선택

# 메인 화면
add-warning = 경고 추가
add-foul = 반칙 추가
start-now = 지금 시작
end-timeout = 타임아웃 종료
warnings = 경고
penalties = 페널티
dark-score-line-1 = 점수
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = 점수
light-score-line-2 = { light-team-name-caps }

# 페널티
black-penalties = 검정 팀 페널티
white-penalties = 흰 팀 페널티

# 점수 편집
final-score = 최종 점수를 입력하세요
confirmation-count-down = 참고: 변경되지 않은 점수는 { $countdown } 후 자동으로 확인됩니다

# 공유 요소
## 타임아웃 리본
end-timeout-line-1 = 종료
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
switch-to = 전환
ref = 심판
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
revive-hold-line-1 = 길게 눌러
revive-hold-line-2 = 복원
revive-deciding-line-2 = 복원됨
penalty-shot-line-1 = 페널티
penalty-shot-line-2 = 샷
pen-shot = 페널티 샷
## 페널티 문자열
served = 집행됨
penalty = #{$player_number} - {$time ->
        [pending] 대기 중
        [served] 집행됨
        [total-dismissal] 완전 퇴장
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
## 설정 문자열
error = 오류 ({ $number })
two-games = 이전 경기: { $prev_game },  다음 경기: { $next_game }
one-game = 경기: { $game }
teams = { -dark-team-name } 팀: { $dark_team }
    { -light-team-name } 팀: { $light_team }
game-config = 전반 시간: { $half_len },  하프타임 시간: { $half_time_len }
    서든 데스 허용: { $sd_allowed },  연장전 허용: { $ot_allowed }
team-timeouts = 팀 타임아웃: { $value }
team-timeouts-label = 팀
    타임아웃:
stop-clock-last-2 = 마지막 2분 시계 정지: { $stop_clock }
ref-list = 주심: { $chief_ref }
    계시원: { $timer }
    수중 심판 1: { $water_ref_1 }
    수중 심판 2: { $water_ref_2 }
    수중 심판 3: { $water_ref_3 }
team-ref-list = 심판: { $ref_team }
    기록원/계시원: { $ts_keeper_team }
unknown = 알 수 없음
## 경기 시간 버튼
next-game = 다음 경기
first-half = 전반
half-time = 하프타임
second-half = 후반
pre-ot-break-full = 연장전 전 휴식
overtime-first-half = 연장 전반
overtime-half-time = 연장 하프타임
overtime-second-half = 연장 후반
pre-sudden-death-break = 서든 데스 전 휴식
sudden-death = 서든 데스
ot-first-half = 연장 1전반
ot-half-time = 연장 하프타임
ot-2nd-half = 연장 2후반
white-timeout-short = 흰 타임아웃
white-timeout-full = 흰 팀 타임아웃
black-timeout-short = 검정 타임아웃
black-timeout-full = 검정 팀 타임아웃
ref-timeout-short = 심판 타임아웃
penalty-shot-short = 페널티 샷
## 경고 컨테이너 생성
team-warning-abreviation = 팀
## 시간 편집기 생성
zero = = 0

# 시간 편집
game-time = 경기 시간
timeout = 타임아웃
Note-Game-time-is-paused = 참고: 이 화면에 있는 동안 경기 시간이 일시 정지됩니다

# 경고 및 반칙 요약
fouls = 반칙
edit-warnings = 경고 편집
edit-fouls = 반칙 편집

# 경고
black-warnings = 검정 팀 경고
white-warnings = 흰 팀 경고

# 메시지
player-number = 선수
    번호:
game-number = 경기
    번호:
num-tos-per-half = 전반당 팀
    타임아웃 수:
num-tos-per-game = 경기당 팀
    타임아웃 수:

# 소리 컨트롤러 - 모드
off = 끄기
low = 낮음
medium = 중간
high = 높음
max = 최대

# 설정
hockey6v6 = 하키 6대6
hockey3v3 = 하키 3대3
rugby = 럭비
beep-test = 비프 테스트

# Beep-test screen
beep-test-pre = 준비
beep-test-top-time-label = 시간
beep-test-top-level-label = 레벨
beep-test-top-lap-label = 랩
beep-test-start = 시작
beep-test-pause = 일시정지
beep-test-resume = 재개
beep-test-reset = 리셋
beep-test-column-level = 레벨
beep-test-column-count = 카운트
beep-test-column-duration = 시간
beep-test-edit-selected = 레벨 { $level }
beep-test-edit-time = 시간
beep-test-edit-count = 카운트
beep-test-edit-new = 레벨 추가
beep-test-edit-remove = 레벨 제거

# 위반 항목
stick-foul = 스틱 반칙
illegal-advance = 불법 전진
sub-foul = 교체 반칙
illegal-stoppage = 불법 정지
out-of-bounds = 경계 밖
grabbing-the-wall = 벽 잡기
obstruction = 방해
delay-of-game = 경기 지연
unsportsmanlike = 비신사적 행위
free-arm = 자유 팔 반칙
false-start = 부정 출발


# Portal Health Indicator
portal-summary-title = { $portal } PORTAL 상태
portal-retry-all = 모두 재시도
portal-row-token-expired = Portal 로그인 만료됨 — 탭하여 다시 로그인
portal-row-stuck = 경기 { $game } 점수 전송 오류, 탭하여 수정
portal-row-pending = 경기 { $game } 점수 전송 안 됨, 탭하여 재시도
portal-row-stats-pending = 경기 { $game } 통계 전송 안 됨, 탭하여 재시도
portal-row-attempt-suffix = (시도 { $attempts }회)
portal-row-recent = 경기 { $game } · { $mins }분 전 전송됨
portal-action-force-submit = 이 경기 결과 재시도
portal-action-discard = 이 경기 결과 버리기
portal-action-discard-confirm = 버리기 확인을 위해 다시 탭하세요
portal-page-title-attention = 경기 { $game } 전송 오류
portal-page-attention-info = 경기 결과가 { $portal } Portal에서 수락되지 않았습니다
portal-page-attention-score = 저장된 경기 결과: 흰 { $white } - 검정 { $black }
portal-page-attention-remediation = 연결이 확인되면 재시도하거나, 오류를 지우려면 버리기를 선택하세요
portal-advisory-at-game-end = Portal 문제가 감지되었습니다. 점수는 계속 대기열에 있습니다 — 관리자에게 문의하세요.

# 2 Halves / 1 Period selector (Half Length editor)
two-halves = 2 하프
one-period = 1 피리어드
game-len = 경기 시간
length-of-game-during-regular-play = 정규 경기 중 전체 경기 시간

# Single-period (1 Period) info variants — Game Length, no half-time line
game-config-single-half = 경기 시간: { $half_len }
    서든 데스 허용: { $sd_allowed },  연장전 허용: { $ot_allowed }
game-length-ot-allowed-single-half = 경기 시간: { $half_length }
         연장전 허용: { $overtime }

# Self-update / Updates page
check-version = 버전 확인
updates-current-version = 현재 버전
updates-check-for-updates = 업데이트 확인
updates-install = 설치
updates-do-revert = 되돌리기
updates-install-note = 설치를 클릭하면 업데이트를 다운로드하여 설치하고 refbox를 다시 시작합니다
updates-revert-note = 되돌리기를 클릭하면 이전 버전을 복원하고 refbox를 다시 시작합니다
updates-unknown = 알 수 없음
updates-checking = 확인 중…
updates-up-to-date = 최신 상태입니다.
updates-available = 업데이트 사용 가능: {$version}
updates-downloading = 다운로드 중…
updates-verifying = 다운로드 확인 중…
updates-installing = 설치 중…
updates-restarting = 다시 시작 중…
updates-confirm-revert = 이전 버전 ({$version})으로 되돌리시겠습니까?
updates-rolled-back = 업데이트가 올바르게 시작되지 않아 이전 버전으로 되돌렸습니다. 다시 시도해 주세요.
updates-revert = 이전 버전으로 되돌리기 ({$version})
updates-error-no-internet = 업데이트 서버에 연결할 수 없습니다. 인터넷 연결을 확인하세요
updates-error-bad-download = 다운로드한 업데이트가 유효하지 않아 설치되지 않았습니다.
updates-error-rate-limited = 업데이트 서버가 사용 중입니다. 잠시 후 다시 시도하세요.
updates-error-no-space = 업데이트를 설치할 여유 공간이 부족합니다.
updates-error-not-writable = 업데이트를 저장할 수 없습니다 (권한이 거부됨).

# Game-info table labels
gi-prior-game = 이전 경기
gi-team-light = { -light-team-name }
gi-team-dark = { -dark-team-name }
gi-current-game = 현재 경기
gi-next-game = 다음 경기
gi-game-block = 게임 블록
gi-half-length = 전반 시간
gi-half-time-length = 하프타임 시간
gi-game-length = 경기 시간
gi-timeouts = 타임아웃
gi-timeout-duration = 타임아웃 시간
gi-overtime = 연장전
gi-sudden-death = 서든 데스
gi-pre-overtime-break = 연장전 전 휴식 시간
gi-pre-sudden-death-break = 서든 데스 전 휴식 시간
gi-overtime-half-length = 연장 전반 시간
gi-overtime-half-time-length = 연장 하프타임 시간
gi-minimum-game-break = 경기 간 최소 시간
gi-stop-clock-last-2 = 마지막 2분 시계 정지
gi-ref-chief = 주심
gi-ref-timekeeper = 계시원
gi-ref-timekeeper-helper = 계시 보조원
gi-ref-water-1 = 수중 심판 1
gi-ref-water-2 = 수중 심판 2
gi-ref-water-3 = 수중 심판 3
gi-ref-water-referees = 수중 심판
gi-ref-deck-referees = 데크 심판
gi-unknown = ???
