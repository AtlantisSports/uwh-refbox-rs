# Definitions for the translation file to use
-dark-team-name = 黑
dark-team-name-caps = 黑队
-light-team-name = 白
light-team-name-caps = 白队

# Multipage
done = 完成
cancel = 取消
delete = 删除
back = 返回
new = 新建

# Penalty Edit
total-dismissal = TD
penalty-kind = {$kind ->
    [thirty-seconds] 30秒
    [one-minute] 1分
    [two-minutes] 2分
    [four-minutes] 4分
    [five-minutes] 5分
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# Team Timeout Edit
timeout-length = 球队暂停
    时长

# Warning Add
team-warning = 球队
    警告
team-warning-line-1 = 球队
team-warning-line-2 = 警告

# Configuration
none-selected = 未选择
loading = 加载中...
game-select = 比赛:
game-options = 比赛选项
app-options = 应用选项
display-options = 显示选项
sound-options = 声音选项
app-mode = 应用
    模式
hide-time-for-last-15-seconds = 隐藏最后
    15秒时间
player-display-brightness = 球员显示
    亮度
confirm-score-at-game-end = 比赛结束
    确认比分
track-cap-number-of-scorer = 追踪进球
    球员号码
event = 赛事:
track-fouls-and-warnings = 追踪犯规
    和警告
court = 球场:
single-half = 单节
    时长:
half-length-full = 半场时长:
game-length = 比赛时长:
overtime-allowed = 允许
    加时赛:
sudden-death-allowed = 允许
    骤死赛:
half-time-length = 中场
    时长:
pre-ot-break-length = 加时前
    休息时长:
pre-sd-break-length = 骤死前
    休息时长:
nominal-break-between-games = 名义场间
    休息:
ot-half-length = 加时半场
    时长:
timeouts-counted-per = 暂停
    计算单位:
game = 比赛
half = 半场
minimum-brk-btwn-games = 最短场间
    休息:
ot-half-time-length = 加时中场
    时长
using-uwh-portal = 使用UWHPORTAL:
starting-sides = 起始位置
sound-enabled = 声音
    启用:
whistle-volume = 哨声
    音量:
manage-remotes = 管理遥控器
whistle-enabled = 哨声
    启用:
above-water-volume = 水上
    音量:
auto-sound-start-play = 自动声音
    开始播放:
buzzer-sound = 蜂鸣器
    声音:
underwater-volume = 水下
    音量:
auto-sound-stop-play = 自动声音
    停止播放:
alarm-button = 警报
    按钮:
alarm = 警报
hold-to-test = 长按测试
or-press-spacebar = 或按空格键
or-hold-spacebar = 或长按空格键
game-info = 比赛信息
remotes = 遥控器
default = 默认
sound = 声音: { $sound_text }
brightness = { $brightness ->
        *[Low] 低
        [Medium] 中
        [High] 高
        [Outdoor] 室外
    }

waiting = 等待中
add = 添加
half-length = 半场时长
length-of-half-during-regular-play = 常规比赛中半场的时长
half-time-lenght = 中场时长
length-of-half-time-period = 中场休息的时长
nom-break = 名义休息
system-will-keep-game-times-spaced = 系统将尽量保持比赛开始时间均匀分布，从一场比赛开始到下一场比赛开始的总时间为 2 × [半场时长] + [中场时长] + [名义场间时间]（例如：若[半场时长] = 15分钟，[中场时长] = 3分钟，[名义场间时间] = 12分钟，则从一场比赛开始到下一场比赛的时间为45分钟。任何暂停或其他停钟都会缩短这12分钟，直到达到最短场间时间为止）。
min-break = 最短休息
min-time-btwn-games = 若比赛超时，这是系统分配的场间最短时间。若比赛落后，系统将在后续比赛后自动追赶，始终遵守此最短场间时间。
pre-ot-break-abreviated = 加时前休息
pre-sd-brk = 若启用加时赛且需要加时，这是第二半场与加时赛第一半场之间的休息时长
ot-half-len = 加时半场时长
time-during-ot = 加时赛中半场的时长
ot-half-tm-len = 加时中场时长
len-of-overtime-halftime = 加时赛中场的时长
pre-sd-break = 骤死前休息
pre-sd-len = 前一比赛阶段到骤死赛之间的休息时长
language = 语言
this-language = 中文
portal-login-code = 代码
portal-login-instructions = 请前往UWH Portal >> 赛事管理 >> 裁判管理，点击+按钮添加新的Refbox，并输入此Refbox ID：
    { $id }

    UWH Portal将随后提供一个确认码，请使用左侧数字键盘输入。
    输入完成后请按完成

help = 帮助:

# Confirmation
game-configuration-can-not-be-changed = 比赛进行中无法更改比赛设置。

    您想怎么做？
apply-this-game-number-change = 您希望如何应用此比赛编号更改？
UWHPortal-enabled = 启用UWHPortal时，所有字段必须填写。
uwhportal-token-invalid-code = 输入的代码无效。
    请重试。
uwhportal-token-no-pending-link = Portal未等待连接。
    请重试。
go-back-to-editor = 返回编辑器
discard-changes = 放弃更改
end-current-game-and-apply-changes = 结束当前比赛并应用更改
end-current-game-and-apply-change = 结束当前比赛并应用更改
keep-current-game-and-apply-change = 保留当前比赛并应用更改
ok = 确定
confirm-score = 此比分是否正确？
    请与首席裁判确认。

    黑队: { $score_black }        白队: { $score_white }

    { confirmation-count-down }
yes = 是
no = 否

# Fouls
equal = 相等

# Game Info
refresh = 刷新
refreshing = 刷新中...
settings = 设置
none = 无
game-number-error = 错误 ({ $game_number })
next-game-number-error = 错误 ({ $next_game_number })
last-game-next-game = 上场: { $prev_game },
    下场: { $next_game }
black-team-white-team = 黑队: { $black_team }
    白队: { $white_team }
game-length-ot-allowed = 半场时长: { $half_length }
         中场时长: { $half_time_length }
         允许加时赛: { $overtime }
overtime-details = 加时前休息时长: { $pre_overtime }
             加时赛半场时长: { $overtime_len }
             加时赛中场时长: { $overtime_half_time_len }
sd-allowed = 允许骤死赛: { $sd }
pre-sd = 骤死前休息时长: { $pre_sd_len }
team-to-len = 球队暂停时长: { $to_len }
time-btwn-games = 名义场间时间: { $time_btwn }
min-brk-btwn-games = 最短场间时间: { $min_brk_time }


# List Selecters
select-event = 选择赛事
select-court = 选择球场
select-game = 选择比赛

# Main View
add-warning = 添加警告
add-foul = 添加犯规
start-now = 立即开始
end-timeout = 结束暂停
warnings = 警告
penalties = 犯规记录
dark-score-line-1 = 进球
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = 进球
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = 黑队犯规记录
white-penalties = 白队犯规记录

# Score edit
final-score = 请输入最终比分
confirmation-count-down = 注意：未修改的比分将在 { $countdown } 内自动确认

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = 结束
end-timeout-line-2 = { timeout }
switch-to = 切换至
ref = 裁判
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = 罚球
penalty-shot-line-2 = 射门
pen-shot = 罚球射门
## Penalty string
served = 已执行
penalty = #{$player_number} - {$time ->
        [pending] 待执行
        [served] 已执行
        [total-dismissal] 已驱逐
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
infraction = 违规: {$infraction}
## Config String
error = 错误 ({ $number })
two-games = 上场: { $prev_game },  下场: { $next_game }
one-game = 比赛: { $game }
teams = { -dark-team-name }队: { $dark_team }
    { -light-team-name }队: { $light_team }
game-config = 半场时长: { $half_len },  中场时长: { $half_time_len }
    允许骤死赛: { $sd_allowed },  允许加时赛: { $ot_allowed }
team-timeouts-per-half = 每半场允许球队暂停次数: { $team_timeouts }
team-timeouts-per-game = 每场允许球队暂停次数: { $team_timeouts }
stop-clock-last-2 = 最后2分钟停钟: { $stop_clock }
ref-list = 首席裁判: { $chief_ref }
    计时员: { $timer }
    水下裁判1: { $water_ref_1 }
    水下裁判2: { $water_ref_2 }
    水下裁判3: { $water_ref_3 }
team-ref-list = 裁判员: { $ref_team }
    计时员/记分员: { $ts_keeper_team }
unknown = 未知
## Game time button
next-game = 下一场
first-half = 上半场
half-time = 中场休息
second-half = 下半场
pre-ot-break-full = 加时前休息
overtime-first-half = 加时赛上半场
overtime-half-time = 加时赛中场
overtime-second-half = 加时赛下半场
pre-sudden-death-break = 骤死赛前休息
sudden-death = 骤死赛
ot-first-half = 加时上半场
ot-half-time = 加时中场
ot-2nd-half = 加时下半场
white-timeout-short = 白暂停
white-timeout-full = 白队暂停
black-timeout-short = 黑暂停
black-timeout-full = 黑队暂停
ref-timeout-short = 裁判暂停
penalty-shot-short = 罚球
## Make warning container
team-warning-abreviation = 队
## Make time editor
zero = 零

# Time edit
game-time = 比赛时间
timeout = 暂停
Note-Game-time-is-paused = 注意：在此屏幕上时比赛时间已暂停

# Warning Fouls Summary
fouls = 犯规
edit-warnings = 编辑警告
edit-fouls = 编辑犯规

# Warnings
black-warnings = 黑队警告
white-warnings = 白队警告

# Message
player-number = 球员
    号码:
game-number = 比赛
    编号:
num-tos-per-half = 每半场球队
    暂停次数:
num-tos-per-game = 每场球队
    暂停次数:

# Sound Controller - mod
off = 关
low = 低
medium = 中
high = 高
max = 最大

# Config
hockey6v6 = 六对六曲棍球
hockey3v3 = 三对三曲棍球
rugby = 橄榄球

# Infractions
stick-foul = 球杆犯规
illegal-advance = 非法前进
sub-foul = 替换犯规
illegal-stoppage = 非法停球
out-of-bounds = 出界
grabbing-the-wall = 抓墙
obstruction = 阻挡
delay-of-game = 延误比赛
unsportsmanlike = 不体育道德行为
free-arm = 自由臂
false-start = 抢跑
