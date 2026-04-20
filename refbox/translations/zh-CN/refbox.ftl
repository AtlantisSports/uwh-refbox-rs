# 翻译文件定义
-dark-team-name = 黑队
dark-team-name-caps = 黑队

-light-team-name = 白队
light-team-name-caps = 白队

# 多页面
done = 完成
restart-to-apply = 重启以应用
cancel = 取消
delete = 删除
back = 返回
apply = 应用
user-options = 用户选项
new = 新建

# 罚时编辑
total-dismissal = 取消
penalty-kind = {$kind ->
    [thirty-seconds] 30秒
    [one-minute] 1分
    [two-minutes] 2分
    [four-minutes] 4分
    [five-minutes] 5分
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# 队伍暂停编辑
timeout-length = 队伍暂停
    时长

# 警告添加
team-warning = 队伍
    警告
team-warning-line-1 = 队伍
team-warning-line-2 = 警告

# 设置
none-selected = 未选择
loading = 加载中...
game-select = 比赛：
game-options = 比赛选项
app-options = 应用选项
display-options = 显示选项
sound-options = 声音选项
app-mode = 应用
    模式
hide-time-for-last-15-seconds = 最后15秒
    隐藏时间
player-display-brightness = 球员显示
    亮度
confirm-score-at-game-end = 比赛结束时
    确认比分
track-cap-number-of-scorer = 追踪进球
    泳帽号码
event = 赛事：
track-fouls-and-warnings = 追踪犯规
    和警告
court = 球场：
single-half = 单节
    比赛：
half-length-full = 半场时长：
game-length = 比赛时长：
overtime-allowed = 允许
    加时赛：
sudden-death-allowed = 允许
    突然死亡：
half-time-length = 中场休息
    时长：
pre-ot-break-length = 加时前
    休息时长：
pre-sd-break-length = 突然死亡前
    休息时长：
nominal-break-between-games = 场间名义
    休息时间：
ot-half-length = 加时半场
    时长：
timeouts-counted-per = 暂停
    计算单位：
game = 比赛
half = 半场
minimum-brk-btwn-games = 场间最短
    休息时间：
ot-half-time-length = 加时中场
    时长
using-uwh-portal = 使用UWHPORTAL：
starting-sides = 起始位置
sound-enabled = 声音
    启用：
whistle-volume = 哨声
    音量：
manage-remotes = 管理遥控器
whistle-enabled = 哨声
    启用：
above-water-volume = 水上
    音量：
auto-sound-start-play = 自动声音
    开始比赛：
buzzer-sound = 蜂鸣器
    声音：
underwater-volume = 水下
    音量：
auto-sound-stop-play = 自动声音
    结束比赛：
alarm-button = 警报
    按钮：
alarm = 警报
hold-to-test = 长按测试
or-press-spacebar = 或按空格键
or-hold-spacebar = 或长按空格键
game-info = 比赛信息
remotes = 遥控器
default = 默认
sound = 声音：{ $sound_text }
brightness = { $brightness ->
        *[Low] 低
        [Medium] 中
        [High] 高
        [Outdoor] 室外
    }

waiting = 等待中
add = 添加
half-length = 半场时长
length-of-half-during-regular-play = 常规比赛中每半场的时长
half-time-lenght = 中场时长
length-of-half-time-period = 中场休息阶段的时长
nom-break = 名义休息
system-will-keep-game-times-spaced = 系统将尽量保持比赛开始时间均匀分布，从一场比赛开始到下一场比赛开始的总时间为 2 × [半场时长] + [中场时长] + [名义场间时间]（例如：若[半场时长] = 15分钟、[中场时长] = 3分钟、[名义场间时间] = 12分钟，则从一场比赛开始到下一场的时间为45分钟。任何暂停或其他停钟操作都会缩短这12分钟，直到达到最短场间时间为止）。
min-break = 最短休息
min-time-btwn-games = 若比赛超出预定时长，这是系统分配的场间最短时间。若比赛落后于计划，系统将在后续比赛中自动追赶，始终遵守此最短场间时间。
pre-ot-break-abreviated = 加时前休息
pre-sd-brk = 若启用加时赛且需要加时，这是下半场与加时赛上半场之间的休息时长
ot-half-len = 加时半场时长
time-during-ot = 加时赛中每半场的时长
ot-half-tm-len = 加时中场时长
len-of-overtime-halftime = 加时赛中场休息的时长
pre-sd-break = 突然死亡前休息
pre-sd-len = 前一比赛阶段到突然死亡赛之间的休息时长
language = 语言
this-language = 中文（简体）
portal-login-code = 代码
portal-login-instructions = 请前往UWH Portal >> 赛事管理 >> 裁判管理，点击+按钮添加新的Refbox，并输入此Refbox ID：
    { $id }

    UWH Portal随后将提供一个确认码，请使用左侧数字键盘输入。
    输入完成后请按"完成"

help = 帮助：

# 确认
game-configuration-can-not-be-changed = 比赛进行中无法更改比赛配置。

    您希望如何处理？
apply-this-game-number-change = 您希望如何应用此比赛编号更改？
UWHPortal-enabled = 启用UWHPortal时，所有字段必须填写。
uwhportal-token-invalid-code = 输入的代码无效。
    请重试。
uwhportal-token-no-pending-link = Portal未等待连接。
    请重试。
go-back-to-editor = 返回编辑器
discard-changes = 放弃更改
end-current-game-and-apply-changes = 结束当前比赛并应用所有更改
end-current-game-and-apply-change = 结束当前比赛并应用更改
keep-current-game-and-apply-change = 保留当前比赛并应用更改
ok = 确定
confirm-score = 此比分是否正确？
    请与主裁判确认。

    黑队：{ $score_black }        白队：{ $score_white }

    { confirmation-count-down }
yes = 是
no = 否

# 犯规
equal = 相等

# 比赛信息
refresh = 刷新
refreshing = 刷新中...
settings = 设置
none = 无
game-number-error = 错误（{ $game_number }）
next-game-number-error = 错误（{ $next_game_number }）
last-game-next-game = 上场：{ $prev_game }，
    下场：{ $next_game }
black-team-white-team = 黑队：{ $black_team }
    白队：{ $white_team }
game-length-ot-allowed = 半场时长：{ $half_length }
         中场时长：{ $half_time_length }
         允许加时赛：{ $overtime }
overtime-details = 加时前休息时长：{ $pre_overtime }
             加时赛半场时长：{ $overtime_len }
             加时赛中场时长：{ $overtime_half_time_len }
sd-allowed = 允许突然死亡：{ $sd }
pre-sd = 突然死亡前休息时长：{ $pre_sd_len }
team-to-len = 队伍暂停时长：{ $to_len }
time-btwn-games = 名义场间时间：{ $time_btwn }
min-brk-btwn-games = 最短场间时间：{ $min_brk_time }


# 列表选择
select-event = 选择赛事
select-court = 选择球场
select-game = 选择比赛

# 主视图
add-warning = 添加警告
add-foul = 添加犯规
start-now = 立即开始
end-timeout = 结束暂停
warnings = 警告
penalties = 罚时记录
dark-score-line-1 = 比分
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = 比分
light-score-line-2 = { light-team-name-caps }

# 罚时
black-penalties = 黑队罚时记录
white-penalties = 白队罚时记录

# 比分编辑
final-score = 请输入最终比分
confirmation-count-down = 注意：未修改的比分将在 { $countdown } 后自动确认

# 共享元素
## 暂停提示条
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
## 罚时字符串
served = 已执行
penalty = #{$player_number} - {$time ->
        [pending] 待执行
        [served] 已执行
        [total-dismissal] 已取消资格
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
infraction = 犯规类型：{$infraction}
## 配置字符串
error = 错误（{ $number }）
two-games = 上场：{ $prev_game }，  下场：{ $next_game }
one-game = 比赛：{ $game }
teams = { -dark-team-name }：{ $dark_team }
    { -light-team-name }：{ $light_team }
game-config = 半场时长：{ $half_len }，  中场时长：{ $half_time_len }
    允许突然死亡：{ $sd_allowed }，  允许加时赛：{ $ot_allowed }
team-timeouts-per-half = 每半场允许队伍暂停次数：{ $team_timeouts }
team-timeouts-per-game = 每场允许队伍暂停次数：{ $team_timeouts }
stop-clock-last-2 = 最后2分钟停钟：{ $stop_clock }
ref-list = 主裁判：{ $chief_ref }
    计时员：{ $timer }
    水下裁判1：{ $water_ref_1 }
    水下裁判2：{ $water_ref_2 }
    水下裁判3：{ $water_ref_3 }
team-ref-list = 裁判员：{ $ref_team }
    计时/记分员：{ $ts_keeper_team }
unknown = 未知
## 比赛时间按钮
next-game = 下一场
first-half = 上半场
half-time = 中场休息
second-half = 下半场
pre-ot-break-full = 加时前休息
overtime-first-half = 加时赛上半场
overtime-half-time = 加时赛中场
overtime-second-half = 加时赛下半场
pre-sudden-death-break = 突然死亡前休息
sudden-death = 突然死亡
ot-first-half = 加时上半场
ot-half-time = 加时中场
ot-2nd-half = 加时下半场
white-timeout-short = 白暂停
white-timeout-full = 白队暂停
black-timeout-short = 黑暂停
black-timeout-full = 黑队暂停
ref-timeout-short = 裁判暂停
penalty-shot-short = 罚球
## 警告容器
team-warning-abreviation = 队
## 时间编辑器
zero = 零

# 时间编辑
game-time = 比赛时间
timeout = 暂停
Note-Game-time-is-paused = 注意：在此界面时比赛时间已暂停

# 警告与犯规汇总
fouls = 犯规
edit-warnings = 编辑警告
edit-fouls = 编辑犯规

# 警告
black-warnings = 黑队警告
white-warnings = 白队警告

# 消息
player-number = 球员
    号码：
game-number = 比赛
    编号：
num-tos-per-half = 每半场队伍
    暂停次数：
num-tos-per-game = 每场队伍
    暂停次数：

# 声音控制器 - 模式
off = 关
low = 低
medium = 中
high = 高
max = 最大

# 配置
hockey6v6 = 六对六水下曲棍球
hockey3v3 = 三对三水下曲棍球
rugby = 橄榄球

# 违规类型
stick-foul = 球棍犯规
illegal-advance = 非法推进
sub-foul = 换人犯规
illegal-stoppage = 非法停球
out-of-bounds = 出界
grabbing-the-wall = 抓墙
obstruction = 阻挡
delay-of-game = 延误比赛
unsportsmanlike = 不体育行为
free-arm = 自由臂犯规
false-start = 抢跑
