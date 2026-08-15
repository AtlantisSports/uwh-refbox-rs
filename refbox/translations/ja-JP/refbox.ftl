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
    時間:
team-timeout-count = チームタイムアウト
    回数:

# 警告追加
team-warning-line-1 = チーム
team-warning-line-2 = 警告
team-score-line-1 = チーム
team-score-line-2 = 得点

# 設定
none-selected = 未選択
none-provided = 未入力
loading = 読込中...
game-select = 試合:
game-options = 試合オプション
app-options = アプリオプション
display-options = 表示オプション
open-new-display = 新しい表示を開く
front-display-layout = DISPLAY LAYOUT
layout-default = DEFAULT
layout-classic = CLASSIC
layout-big-time = BIG TIME
layout-corners = CORNERS
layout-scores-only = SCORES ONLY
sound-options = サウンドオプション
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
sound-settings = サウンド設定
beep-test-edit-levels = レベル編集
app-mode = アプリ
    モード
player-display-brightness = 選手表示
    明るさ
confirm-score-at-game-end = 試合終了時
    得点確認
track-cap-number = キャップ番号
    記録
force-keypad-numbers = 数字キーパッドを
    常に使用
event = 大会:
track-fouls-and-warnings = 反則・
    警告の記録
show-behind-schedule-time = 遅延を表示
show-countdown-for-last-10-seconds = 残り10秒の
    カウントダウン表示
audible-countdown-for-last-10-seconds = 残り10秒の音声
    カウントダウン
delay = 遅延
court = コート:
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
manual-games = 手動試合:
source-portal = { $portal } PORTAL
source-custom = カスタム
access-token = アクセストークン:
access-token-connected = 接続済み
access-token-tap-to-connect = タップして接続
access-token-checking = 確認中...
custom-site = サイト:
custom-site-url-title = サイトURL
custom-site-placeholder = https://your-site/api/1234-A
custom-site-invalid =
    このアドレスは使用できません。次の形式にしてください：
    https://your-site/api/1234-A
starting-sides = 開始サイド
sound-enabled = サウンド
    有効:
whistle-volume = 笛
    音量:
manage-remotes = リモコン管理
update-audio-output = 出力を更新
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
test = テスト
or-press-spacebar = またはスペースキーを押す
or-hold-spacebar = またはスペースキーを長押し
game-info = 情報
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
game-block = ゲームブロック
game-block-full = ゲームブロック:
game-block-help = ある試合の開始から次の試合の開始までの時間
game-block-too-short = 試合と最短休憩を収めるには短すぎます
game-block-tight = タイト — チームタイムアウトにより試合が枠を超える可能性があります
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
portal-login-code = コード
portal-login-instructions = { $portal }ポータル >> 大会管理 >> 審判管理 へ進み、＋ボタンをクリックして新しいRefboxを追加し、このRefbox IDを入力してください:
    { $id }

    { $portal }ポータルから確認コードが発行されますので、左の数字パッドで入力してください。
    コードを入力したら完了を押してください
custom-login-instructions = この Refbox ID をご利用のサイトに伝えてください:
    { $id }

    次に、サイトから提供された確認コードをテンキーで入力し、完了 を押してください


# 確認
game-configuration-can-not-be-changed = 試合進行中は試合設定を変更できません。

    どうしますか？
apply-this-game-number-change = この試合番号の変更をどのように適用しますか？
apply-switch-to-manual = 手動モードに切り替えると、読み込まれたスケジュールが消去され、次の試合前の時間がリセットされます。試合が進行中です。
portal-enabled = { $portal }Portalが有効な場合、すべての項目を入力する必要があります。
mode-switch-portal-tenant = アプリのモードを変更すると、{ $from_portal } Portalへのリンクが無効になり、{ $to_portal } Portalに再接続する必要があります。
mode-switch-custom-site = アプリのモードを変更するには再起動が必要です。カスタムサイトの接続は維持されます。
source-locked-game = 試合進行中は試合データの取得元を変更できません。
source-switch-clears-selection = { $source } に切り替えると、選択した大会、コート、試合が消去されます。
link-locked-game = 試合進行中は試合データの取得元に接続できません。
source-locked-queue = 送信待ちの試合結果があります。先に送信するか破棄してください。
uwhportal-token-invalid-code = 無効なコードが入力されました。
    もう一度試してください。
uwhportal-token-no-pending-link = 接続は通信を待っていません。
    もう一度試してください。
uwhportal-token-unusable-key = サイトから送られたアクセスキーは、この Refbox では使用できません。
    サイトにキーをもう一度要求してください。
go-back-to-editor = 編集画面に戻る
discard-changes = 変更を破棄
end-current-game-and-apply-changes = 現在の試合を終了して変更を適用
end-current-game-and-apply-change = 現在の試合を終了して変更を適用
keep-current-game-and-apply-change = 現在の試合を続けて変更を適用
ok = OK
switch-and-clear-selection = 切り替えて選択を消去
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
cancel-timeout = { cancel } { timeout }
cancel-timeout-line-1 = { cancel }
cancel-timeout-line-2 = { timeout }
cancel-ref-timeout = { cancel } { ref } { timeout }
cancel-ref-timeout-line-1 = { cancel } { ref }
cancel-ref-timeout-line-2 = { timeout }
cancel-pen-shot = { cancel } { pen-shot }
cancel-pen-shot-line-1 = { cancel }
cancel-pen-shot-line-2 = { pen-shot }
switch-to = 切替
ref = 審判
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
revive-hold-line-1 = 長押しで
revive-hold-line-2 = 復元
revive-deciding-line-2 = 復元しました
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
team-timeouts-label = チーム
    タイムアウト:
unknown = 不明
select-infraction = 選択してください
## 試合時間ボタン
next-game = 次の試合
schedule-end = 終了
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
beep-test-top-time-label = 時間
beep-test-top-start-in-label = 開始まで
beep-test-top-level-label = レベル
beep-test-top-lap-label = 周回
beep-test-start = スタート
beep-test-pause = 一時停止
beep-test-resume = 再開
beep-test-reset = リセット
beep-test-edit-time = 時間
beep-test-edit-count = 回数
beep-test-preset-ref = 審判
beep-test-preset-heading = プリセット

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
portal-summary-title = 接続状態
portal-retry-all = すべて再試行
portal-row-token-expired = アクセストークンの有効期限が切れました — タップして再ログイン
portal-row-startup-failed = 接続できません — 結果はアップロードされません
portal-row-stuck = 試合 { $game } のスコア送信エラー、タップして修正
portal-row-pending = 試合 { $game } のスコアが未送信、タップして再試行
portal-row-stats-pending = 試合 { $game } の統計が未送信、タップして再試行
portal-row-recent = 試合 { $game } · { $mins } 分前に送信済み
portal-row-attempt-suffix = (試行 { $attempts })
portal-action-force-submit = この試合結果を再送信
portal-action-discard = この試合結果を破棄
portal-action-discard-confirm = もう一度タップして破棄を確定
portal-page-title-attention = 試合 { $game } の送信エラー
portal-page-attention-info = 試合結果が受理されていません
portal-page-attention-score = 保存された試合結果: 白 { $white } - 黒 { $black }
portal-page-attention-remediation = 接続が確認できれば再送信、またはエラーをクリアするには破棄してください
portal-advisory-at-game-end = 接続の問題を検出しました。スコアはキューに残ります — 管理者に解決を依頼してください。

# 2 Halves / 1 Period selector (Half Length editor)
two-halves = 2ハーフ
one-period = 1ピリオド
game-len = 試合時間
length-of-game-during-regular-play = 通常試合中の試合全体の長さ

# Self-update / Updates page
check-version = バージョン確認
updates-current-version = 現在のバージョン
updates-check-for-updates = 更新を確認
updates-install = インストール
updates-do-revert = 元に戻す
updates-install-note = インストールをクリックすると、更新をダウンロードしてインストールし、refbox を再起動します
updates-revert-note = 元に戻すをクリックすると、以前のバージョンを復元し、refbox を再起動します
updates-unknown = 不明
updates-checking = 確認中…
updates-up-to-date = 最新です。
updates-available = 更新があります: {$version}
updates-downloading = ダウンロード中…
updates-verifying = ダウンロードを確認中…
updates-installing = インストール中…
updates-restarting = 再起動中…
updates-confirm-revert = 以前のバージョン ({$version}) に戻しますか？
updates-rolled-back = 更新が正しく起動しなかったため、以前のバージョンに戻しました。もう一度お試しください。
updates-revert = 以前のバージョンに戻す ({$version})
updates-error-no-internet = 更新サーバーに接続できませんでした。インターネット接続を確認してください
updates-error-bad-download = ダウンロードした更新が無効だったため、インストールされませんでした。
updates-error-rate-limited = 更新サーバーが混雑しています。しばらくしてからもう一度お試しください。
updates-error-no-space = 更新をインストールするための空き容量が足りません。
updates-error-not-writable = 更新を保存できませんでした（アクセスが拒否されました）。

# Game-info table labels
gi-prior-game = 前の試合
gi-team-light = { -light-team-name }
gi-team-dark = { -dark-team-name }
gi-current-game = 現在の試合
gi-next-game = 次の試合
gi-game-block = ゲームブロック
gi-half-length = ハーフ時間
gi-half-time-length = ハーフタイム時間
gi-game-length = 試合時間
gi-timeouts = タイムアウト
gi-timeout-duration = タイムアウト時間
gi-overtime = 延長戦
gi-sudden-death = サドンデス
gi-pre-overtime-break = 延長前休憩時間
gi-pre-sudden-death-break = サドンデス前休憩時間
gi-overtime-half-length = 延長ハーフ時間
gi-overtime-half-time-length = 延長ハーフタイム時間
gi-minimum-game-break = 試合間最小時間
gi-stop-clock-last-2 = 残り2分でクロック停止
gi-ref-chief = 主審
gi-ref-timekeeper = タイムキーパー
gi-ref-timekeeper-helper = タイムキーパー補佐
gi-ref-water-1 = 水中審判1
gi-ref-water-2 = 水中審判2
gi-ref-water-3 = 水中審判3
gi-ref-water-referees = 水中審判
gi-ref-deck-referees = デッキ審判
gi-unknown = ???

shut-down = シャットダウン
restart-pi = Pi を再起動
restart-refbox = Refbox を再起動
