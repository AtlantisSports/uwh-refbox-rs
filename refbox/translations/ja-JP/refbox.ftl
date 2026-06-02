# 翻訳ファイルの定義
-dark-team-name = 黒
dark-team-name-caps = 黒チーム

-light-team-name = 白
light-team-name-caps = 白チーム

# マルチページ
done = 完了
restart-to-apply = 再起動して適用
cancel = キャンセル
delete = 削除
back = 戻る
apply = 適用
save = 保存
user-options = ユーザー設定
new = 新規

# ペナルティ編集
total-dismissal = 退場
penalty-kind = {$kind ->
    [thirty-seconds] 30秒
    [one-minute] 1分
    [two-minutes] 2分
    [four-minutes] 4分
    [five-minutes] 5分
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# チームタイムアウト編集
timeout-length = チームタイムアウト
    時間

# 警告追加
team-warning = チーム
    警告
team-warning-line-1 = チーム
team-warning-line-2 = 警告

# 設定
none-selected = 未選択
loading = 読込中...
game-select = 試合:
game-options = 試合オプション
app-options = アプリオプション
display-options = 表示オプション
open-new-display = 新しい表示を開く
sound-options = サウンドオプション
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
sound-settings = サウンド設定
beep-test-edit-levels = レベル編集
app-mode = アプリ
    モード
hide-time-for-last-15-seconds = 残り15秒
    時間非表示
player-display-brightness = 選手表示
    明るさ
confirm-score-at-game-end = 試合終了時
    得点確認
track-cap-number-of-scorer = 得点者の
    キャップ番号記録
event = 大会:
track-fouls-and-warnings = 反則・
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
using-portal = { $portal }PORTAL使用:
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
portal-login-instructions = { $portal }ポータル >> 大会管理 >> 審判管理 へ進み、＋ボタンをクリックして新しいRefboxを追加し、このRefbox IDを入力してください:
    { $id }

    { $portal }ポータルから確認コードが発行されますので、左の数字パッドで入力してください。
    コードを入力したら完了を押してください

help = ヘルプ:

# 確認
game-configuration-can-not-be-changed = 試合進行中は試合設定を変更できません。

    どうしますか？
apply-this-game-number-change = この試合番号の変更をどのように適用しますか？
portal-enabled = { $portal }Portalが有効な場合、すべての項目を入力する必要があります。
mode-switch-portal-tenant = モードを{ $from_mode }から{ $to_mode }に変更すると、{ $from_portal }PORTALへのリンクが無効になり、{ $to_portal }PORTALに再接続する必要があります。
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
    主審に確認してください。

    黒: { $score_black }        白: { $score_white }

    { confirmation-count-down }
yes = はい
no = いいえ

# 反則
equal = 同点

# 試合情報
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


# リスト選択
select-event = 大会を選択
select-court = コートを選択
select-game = 試合を選択

# メイン画面
add-warning = 警告追加
add-foul = 反則追加
start-now = 今すぐ開始
end-timeout = タイムアウト終了
warnings = 警告
penalties = 退水
dark-score-line-1 = 得点
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = 得点
light-score-line-2 = { light-team-name-caps }

# 退水記録
black-penalties = 黒チーム退水
white-penalties = 白チーム退水

# 得点編集
final-score = 最終スコアを入力してください
confirmation-count-down = 注意: 変更されていないスコアは { $countdown } 後に自動的に確定されます

# 共通要素
## タイムアウト帯
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
penalty-shot-line-1 = ペナルティー
penalty-shot-line-2 = ショット
pen-shot = ペナルティーショット
## ペナルティ表示
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
## 設定文字列
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
ref-list = 主審: { $chief_ref }
    タイムキーパー: { $timer }
    水中審判1: { $water_ref_1 }
    水中審判2: { $water_ref_2 }
    水中審判3: { $water_ref_3 }
team-ref-list = 審判員: { $ref_team }
    記録員: { $ts_keeper_team }
unknown = 不明
## 試合時間ボタン
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
ref-timeout-short = 審判T/O
penalty-shot-short = PEN SHT
## 警告コンテナ
team-warning-abreviation = 警
## 時間編集
zero = = 0

# 時間編集
game-time = 試合時間
timeout = タイムアウト
Note-Game-time-is-paused = 注意: この画面中は試合時間が一時停止されています

# 警告・反則一覧
fouls = 反則
edit-warnings = 警告を編集
edit-fouls = 反則を編集

# 警告
black-warnings = 黒チーム警告
white-warnings = 白チーム警告

# メッセージ
player-number = 選手
    番号:
game-number = 試合
    番号:
num-tos-per-half = ハーフあたりの
    チームT/O数:
num-tos-per-game = 試合あたりの
    チームT/O数:

# サウンドコントローラー
off = オフ
low = 低
medium = 中
high = 高
max = 最大

# 設定
hockey6v6 = ホッケー6対6
hockey3v3 = ホッケー3対3
rugby = ラグビー
beep-test = ビープテスト

# Beep-test screen
beep-test-pre = 準備
beep-test-top-time-label = 時間
beep-test-top-level-label = レベル
beep-test-top-lap-label = 周回
beep-test-start = スタート
beep-test-pause = 一時停止
beep-test-resume = 再開
beep-test-reset = リセット
beep-test-column-level = レベル
beep-test-column-count = 回数
beep-test-column-duration = 時間
beep-test-edit-selected = レベル { $level }
beep-test-edit-time = 時間
beep-test-edit-count = 回数
beep-test-edit-new = レベル追加
beep-test-edit-remove = レベル削除

# 反則種別
stick-foul = スティック反則
illegal-advance = 違反前進
sub-foul = 交代反則
illegal-stoppage = 違反停止
out-of-bounds = コート外
grabbing-the-wall = 壁つかみ
obstruction = 妨害
delay-of-game = 遅延行為
unsportsmanlike = 非紳士的行為
free-arm = フリーハンド反則
false-start = 不正スタート


# Portal Health Indicator
portal-summary-title = { $portal } PORTAL 状態
portal-row-token-expired = ポータルのログインが期限切れです — タップして再ログイン
portal-row-stuck = 試合 { $game } のスコア送信エラー、タップして修正
portal-row-pending = 試合 { $game } のスコアが未送信、タップして再試行
portal-row-recent = 試合 { $game } · { $mins } 分前に送信済み
portal-row-attempt-suffix = (試行 { $attempts })
portal-action-force-submit = この試合結果を再送信
portal-action-discard = この試合結果を破棄
portal-action-discard-confirm = もう一度タップして破棄を確定
portal-page-title-attention = 試合 { $game } の送信エラー
portal-page-attention-info = 試合結果が { $portal } ポータルに受理されていません
portal-page-attention-score = 保存された試合結果: 白 { $white } - 黒 { $black }
portal-page-attention-remediation = 接続が確認できれば再送信、またはエラーをクリアするには破棄してください
portal-advisory-at-game-end = ポータルの問題を検出しました。スコアはキューに残ります — 管理者に解決を依頼してください。
