# Definitions for the translation file to use
-dark-team-name = 黒
dark-team-name-caps = 黒チーム

-light-team-name = 白
light-team-name-caps = 白チーム

# Multipage
done = 完了
restart-to-apply = 再起動して適用
cancel = キャンセル
delete = 削除
back = 戻る
new = 新規

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
timeout-length = チーム タイムアウト
    時間

# Warning Add
team-warning = チーム
    警告
team-warning-line-1 = チーム
team-warning-line-2 = 警告

# Configuration
none-selected = 未選択
loading = 読込中...
game-select = 試合:
game-options = 試合オプション
app-options = アプリオプション
display-options = 表示オプション
sound-options = サウンドオプション
app-mode = アプリ
    モード
hide-time-for-last-15-seconds = 残り15秒
    時間非表示
player-display-brightness = 選手表示
    明るさ
confirm-score-at-game-end = 試合終了時
    スコア確認
track-cap-number-of-scorer = 得点者の
    番号記録
event = 大会:
track-fouls-and-warnings = ファウル・
    警告の記録
court = コート:
single-half = 単一
    ハーフ:
half-length-full = ハーフ時間:
game-length = 試合時間:
overtime-allowed = 延長戦
    許可:
sudden-death-allowed = サドンデス
    許可:
half-time-length = ハーフタイム
    時間:
pre-ot-break-length = 延長前
    休憩時間:
pre-sd-break-length = SD前
    休憩時間:
nominal-break-between-games = 試合間
    基準休憩:
ot-half-length = 延長ハーフ
    時間:
timeouts-counted-per = タイムアウト
    集計単位:
game = 試合
half = ハーフ
minimum-brk-btwn-games = 試合間
    最小休憩:
ot-half-time-length = 延長ハーフ
    タイム時間
using-uwh-portal = UWHPORTAL使用:
starting-sides = 開始サイド
sound-enabled = サウンド
    有効:
whistle-volume = 笛
    音量:
manage-remotes = リモコン管理
whistle-enabled = 笛
    有効:
above-water-volume = 水上
    音量:
auto-sound-start-play = 自動サウンド
    プレー開始:
buzzer-sound = ブザー
    音:
underwater-volume = 水中
    音量:
auto-sound-stop-play = 自動サウンド
    プレー停止:
alarm-button = アラーム
    ボタン:
alarm = アラーム
hold-to-test = 長押しでテスト
or-press-spacebar = またはスペースキーを押す
or-hold-spacebar = またはスペースキーを長押し
game-info = 試合情報
remotes = リモコン
default = デフォルト
sound = サウンド: { $sound_text }
brightness = { $brightness ->
        *[Low] 低
        [Medium] 中
        [High] 高
        [Outdoor] 屋外
    }

waiting = 待機中
add = 追加
half-length = ハーフ時間
length-of-half-during-regular-play = 通常試合中のハーフの長さ
half-time-lenght = ハーフタイム時間
length-of-half-time-period = ハーフタイムの長さ
nom-break = 基準休憩
system-will-keep-game-times-spaced = システムは試合開始時刻を均等に保つよう努めます。1試合の開始から次の試合の開始までの合計時間は、2 × [ハーフ時間] + [ハーフタイム時間] + [試合間基準時間] となります（例：[ハーフ時間] = 15分、[ハーフタイム時間] = 3分、[試合間基準時間] = 12分の場合、1試合の開始から次の試合まで45分。タイムアウトやその他のクロック停止があると、最小試合間時間に達するまで12分が短縮されます）。
min-break = 最小休憩
min-time-btwn-games = 試合が予定より長引いた場合、システムが確保する試合間の最小時間です。試合が遅れた場合、システムは後続の試合で自動的に追いつきを試みますが、常にこの最小試合間時間を守ります。
pre-ot-break-abreviated = 延長前休憩
pre-sd-brk = 延長戦が許可されており必要な場合、これは第2ハーフと延長第1ハーフの間の休憩時間です
ot-half-len = 延長ハーフ時間
time-during-ot = 延長戦中のハーフの長さ
ot-half-tm-len = 延長ハーフタイム時間
len-of-overtime-halftime = 延長ハーフタイムの長さ
pre-sd-break = SD前休憩
pre-sd-len = 直前のプレー期間とサドンデスの間の休憩時間
language = 言語
this-language = 日本語
portal-login-code = コード
portal-login-instructions = UWHポータル >> 大会管理 >> 審判管理 へ進み、＋ボタンをクリックして新しいRefboxを追加し、このRefbox IDを入力してください:
    { $id }

    UWHポータルから確認コードが発行されますので、左の数字パッドで入力してください。
    コードを入力したら完了を押してください

help = ヘルプ:

# Confirmation
game-configuration-can-not-be-changed = 試合進行中は試合設定を変更できません。

    どうしますか？
apply-this-game-number-change = この試合番号の変更をどのように適用しますか？
UWHPortal-enabled = UWHPortalが有効な場合、すべての項目を入力する必要があります。
uwhportal-token-invalid-code = 無効なコードが入力されました。
    もう一度試してください。
uwhportal-token-no-pending-link = ポータルは接続を待っていません。
    もう一度試してください。
go-back-to-editor = 編集画面に戻る
discard-changes = 変更を破棄
end-current-game-and-apply-changes = 現在の試合を終了して変更を適用
end-current-game-and-apply-change = 現在の試合を終了して変更を適用
keep-current-game-and-apply-change = 現在の試合を続けて変更を適用
ok = OK
confirm-score = このスコアは正しいですか？
    主任審判に確認してください。

    黒: { $score_black }        白: { $score_white }

    { confirmation-count-down }
yes = はい
no = いいえ

# Fouls
equal = 同等

# Game Info
refresh = 更新
refreshing = 更新中...
settings = 設定
none = なし
game-number-error = エラー ({ $game_number })
next-game-number-error = エラー ({ $next_game_number })
last-game-next-game = 前の試合: { $prev_game },
    次の試合: { $next_game }
black-team-white-team = 黒チーム: { $black_team }
    白チーム: { $white_team }
game-length-ot-allowed = ハーフ時間: { $half_length }
         ハーフタイム時間: { $half_time_length }
         延長戦許可: { $overtime }
overtime-details = 延長前休憩時間: { $pre_overtime }
             延長ハーフ時間: { $overtime_len }
             延長ハーフタイム時間: { $overtime_half_time_len }
sd-allowed = サドンデス許可: { $sd }
pre-sd = サドンデス前休憩時間: { $pre_sd_len }
team-to-len = チームタイムアウト時間: { $to_len }
time-btwn-games = 試合間基準時間: { $time_btwn }
min-brk-btwn-games = 試合間最小時間: { $min_brk_time }


# List Selecters
select-event = 大会を選択
select-court = コートを選択
select-game = 試合を選択

# Main View
add-warning = 警告追加
add-foul = ファウル追加
start-now = 今すぐ開始
end-timeout = タイムアウト終了
warnings = 警告
penalties = ペナルティ
dark-score-line-1 = スコア
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = スコア
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = 黒チームペナルティ
white-penalties = 白チームペナルティ

# Score edit
final-score = 最終スコアを入力してください
confirmation-count-down = 注意: 変更されていないスコアは { $countdown } 後に自動的に確定されます

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = 終了
end-timeout-line-2 = { timeout }
switch-to = 切替
ref = 審判
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = ペナルティ
penalty-shot-line-2 = ショット
pen-shot = ペナルティショット
## Penalty string
served = 執行済
penalty = #{$player_number} - {$time ->
        [pending] 保留中
        [served] 執行済
        [total-dismissal] 退場
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
infraction = 反則: {$infraction}
## Config String
error = エラー ({ $number })
two-games = 前の試合: { $prev_game },  次の試合: { $next_game }
one-game = 試合: { $game }
teams = { -dark-team-name }チーム: { $dark_team }
    { -light-team-name }チーム: { $light_team }
game-config = ハーフ時間: { $half_len },  ハーフタイム時間: { $half_time_len }
    サドンデス許可: { $sd_allowed },  延長戦許可: { $ot_allowed }
team-timeouts-per-half = 1ハーフあたりのチームタイムアウト数: { $team_timeouts }
team-timeouts-per-game = 1試合あたりのチームタイムアウト数: { $team_timeouts }
stop-clock-last-2 = 残り2分でクロック停止: { $stop_clock }
ref-list = 主任審判: { $chief_ref }
    計時員: { $timer }
    水中審判1: { $water_ref_1 }
    水中審判2: { $water_ref_2 }
    水中審判3: { $water_ref_3 }
team-ref-list = 審判: { $ref_team }
    タイムキーパー/スコアラー: { $ts_keeper_team }
unknown = 不明
## Game time button
next-game = 次の試合
first-half = 前半
half-time = ハーフタイム
second-half = 後半
pre-ot-break-full = 延長前休憩
overtime-first-half = 延長前半
overtime-half-time = 延長ハーフタイム
overtime-second-half = 延長後半
pre-sudden-death-break = サドンデス前休憩
sudden-death = サドンデス
ot-first-half = 延長第1ハーフ
ot-half-time = 延長ハーフタイム
ot-2nd-half = 延長第2ハーフ
white-timeout-short = 白 T/O
white-timeout-full = 白チームタイムアウト
black-timeout-short = 黒 T/O
black-timeout-full = 黒チームタイムアウト
ref-timeout-short = 審判タイムアウト
penalty-shot-short = ペナルティ
## Make warning container
team-warning-abreviation = チ
## Make time editor
zero = ゼロ

# Time edit
game-time = 試合時間
timeout = タイムアウト
Note-Game-time-is-paused = 注意: この画面中は試合時間が一時停止されています

# Warning Fouls Summary
fouls = ファウル
edit-warnings = 警告を編集
edit-fouls = ファウルを編集

# Warnings
black-warnings = 黒チーム警告
white-warnings = 白チーム警告

# Message
player-number = 選手
    番号:
game-number = 試合
    番号:
num-tos-per-half = ハーフあたりの
    チームT/O数:
num-tos-per-game = 試合あたりの
    チームT/O数:

# Sound Controller - mod
off = オフ
low = 低
medium = 中
high = 高
max = 最大

# Config
hockey6v6 = ホッケー6対6
hockey3v3 = ホッケー3対3
rugby = ラグビー

# Infractions
stick-foul = スティックファウル
illegal-advance = 不正前進
sub-foul = 交代ファウル
illegal-stoppage = 不正停止
out-of-bounds = アウトオブバウンズ
grabbing-the-wall = 壁つかみ
obstruction = 妨害
delay-of-game = 試合遅延
unsportsmanlike = スポーツマンシップ違反
free-arm = フリーアーム
false-start = フライング
