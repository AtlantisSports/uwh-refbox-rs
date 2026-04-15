# Definitions for the translation file to use
-dark-team-name = ดำ
dark-team-name-caps = ดำ

-light-team-name = ขาว
light-team-name-caps = ขาว

# Multipage
done = เสร็จสิ้น
cancel = ยกเลิก
delete = ลบ
back = กลับ
new = ใหม่

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
timeout-length = ระยะเวลา
    หมดเวลาทีม

# Warning Add
team-warning = คำเตือน
    ทีม
team-warning-line-1 = คำเตือน
team-warning-line-2 = ทีม

# Configuration
none-selected = ไม่ได้เลือก
loading = กำลังโหลด...
game-select = เกม:
game-options = ตัวเลือกเกม
app-options = ตัวเลือกแอป
display-options = ตัวเลือกการแสดงผล
sound-options = ตัวเลือกเสียง
app-mode = โหมด
    แอป
hide-time-for-last-15-seconds = ซ่อนเวลา
    15 วินาทีสุดท้าย
player-display-brightness = ความสว่าง
    หน้าจอผู้เล่น
confirm-score-at-game-end = ยืนยันคะแนน
    ท้ายเกม
track-cap-number-of-scorer = บันทึกหมายเลข
    หมวกผู้ทำคะแนน
event = กิจกรรม:
track-fouls-and-warnings = บันทึกฟาวล์
    และคำเตือน
court = สนาม:
single-half = ครึ่ง
    เดียว:
half-length-full = ความยาวครึ่งเวลา:
game-length = ความยาวเกม:
overtime-allowed = อนุญาต
    ต่อเวลา:
sudden-death-allowed = อนุญาต
    ซัดเดนเดธ:
half-time-length = ความยาว
    พักครึ่ง:
pre-ot-break-length = พักก่อน
    ต่อเวลา:
pre-sd-break-length = พักก่อน
    ซัดเดนเดธ:
nominal-break-between-games = พักระหว่างเกม
    ตามกำหนด:
ot-half-length = ครึ่งเวลา
    ต่อเวลา:
timeouts-counted-per = นับหมดเวลา
    ต่อ:
game = เกม
half = ครึ่งเวลา
minimum-brk-btwn-games = พักระหว่างเกม
    ขั้นต่ำ:
ot-half-time-length = พักครึ่ง
    ต่อเวลา
using-uwh-portal = ใช้ UWHPORTAL:
starting-sides = ด้านเริ่มต้น
sound-enabled = เปิดใช้
    เสียง:
whistle-volume = ระดับเสียง
    นกหวีด:
manage-remotes = จัดการรีโมต
whistle-enabled = เปิดใช้
    นกหวีด:
above-water-volume = ระดับเสียง
    เหนือน้ำ:
auto-sound-start-play = เสียงอัตโนมัติ
    เริ่มเกม:
buzzer-sound = เสียง
    บัซเซอร์:
underwater-volume = ระดับเสียง
    ใต้น้ำ:
auto-sound-stop-play = เสียงอัตโนมัติ
    หยุดเกม:
alarm-button = ปุ่ม
    สัญญาณเตือน:
alarm = สัญญาณเตือน
hold-to-test = กดค้างเพื่อทดสอบ
or-press-spacebar = หรือกดสเปซบาร์
or-hold-spacebar = หรือกดค้างสเปซบาร์
game-info = ข้อมูลเกม
remotes = รีโมต
default = ค่าเริ่มต้น
sound = เสียง: { $sound_text }
brightness = { $brightness ->
        *[Low] ต่ำ
        [Medium] กลาง
        [High] สูง
        [Outdoor] กลางแจ้ง
    }

waiting = กำลังรอ
add = เพิ่ม
half-length = ความยาวครึ่ง
length-of-half-during-regular-play = ความยาวของครึ่งเวลาระหว่างการเล่นปกติ
half-time-lenght = ความยาวพักครึ่ง
length-of-half-time-period = ความยาวของช่วงพักครึ่งเวลา
nom-break = พักตามกำหนด
system-will-keep-game-times-spaced = ระบบจะพยายามรักษาเวลาเริ่มเกมให้เท่ากัน โดยเวลารวมจากการเริ่มหนึ่งไปอีกการเริ่มหนึ่งคือ 2 × [ความยาวครึ่ง] + [ความยาวพักครึ่ง] + [เวลาระหว่างเกมตามกำหนด] (ตัวอย่าง: ถ้า [ความยาวครึ่ง] = 15 นาที, [ความยาวพักครึ่ง] = 3 นาที และ [เวลาระหว่างเกมตามกำหนด] = 12 นาที เวลาจากการเริ่มเกมหนึ่งไปอีกเกมหนึ่งจะเป็น 45 นาที การหมดเวลาหรือการหยุดนาฬิกาอื่นๆ จะลดเวลา 12 นาทีลงจนถึงค่าเวลาระหว่างเกมขั้นต่ำ)
min-break = พักขั้นต่ำ
min-time-btwn-games = หากเกมดำเนินนานกว่ากำหนด นี่คือเวลาระหว่างเกมขั้นต่ำที่ระบบจะจัดสรร หากเกมล่าช้า ระบบจะพยายามตามทันในเกมถัดไป โดยเคารพเวลาระหว่างเกมขั้นต่ำนี้เสมอ
pre-ot-break-abreviated = พักก่อนต่อเวลา
pre-sd-brk = หากเปิดใช้การต่อเวลาและจำเป็น นี่คือความยาวของการพักระหว่างครึ่งที่สองและครึ่งแรกของการต่อเวลา
ot-half-len = ครึ่งเวลาต่อเวลา
time-during-ot = ความยาวของครึ่งเวลาระหว่างการต่อเวลา
ot-half-tm-len = พักครึ่งต่อเวลา
len-of-overtime-halftime = ความยาวของพักครึ่งเวลาต่อเวลา
pre-sd-break = พักก่อนซัดเดนเดธ
pre-sd-len = ความยาวของการพักระหว่างช่วงเล่นก่อนหน้าและซัดเดนเดธ
language = ภาษา
this-language = ภาษาไทย
portal-login-code = รหัส
portal-login-instructions = กรุณาไปที่ UWH Portal >> การจัดการกิจกรรม >> การจัดการผู้ตัดสิน คลิกปุ่ม + เพื่อเพิ่ม Refbox ใหม่ และป้อน Refbox ID นี้:
    { $id }

    UWH Portal จะให้รหัสยืนยันสำหรับป้อนทางซ้ายโดยใช้แป้นตัวเลข
    กดเสร็จสิ้นเมื่อป้อนรหัสแล้ว

help = ช่วยเหลือ:

# Confirmation
game-configuration-can-not-be-changed = ไม่สามารถเปลี่ยนการตั้งค่าเกมได้ขณะที่เกมกำลังดำเนินอยู่

    คุณต้องการทำอะไร?
apply-this-game-number-change = คุณต้องการใช้การเปลี่ยนหมายเลขเกมนี้อย่างไร?
UWHPortal-enabled = เมื่อเปิดใช้ UWHPortal ต้องกรอกข้อมูลทุกช่อง
uwhportal-token-invalid-code = รหัสที่ป้อนไม่ถูกต้อง
    โปรดลองอีกครั้ง
uwhportal-token-no-pending-link = พอร์ทัลไม่รอการเชื่อมต่อ
    โปรดลองอีกครั้ง
go-back-to-editor = กลับไปยังตัวแก้ไข
discard-changes = ยกเลิกการเปลี่ยนแปลง
end-current-game-and-apply-changes = สิ้นสุดเกมปัจจุบันและใช้การเปลี่ยนแปลง
end-current-game-and-apply-change = สิ้นสุดเกมปัจจุบันและใช้การเปลี่ยนแปลง
keep-current-game-and-apply-change = เก็บเกมปัจจุบันและใช้การเปลี่ยนแปลง
ok = ตกลง
confirm-score = คะแนนนี้ถูกต้องหรือไม่?
    ยืนยันกับผู้ตัดสินหลัก

    ดำ: { $score_black }        ขาว: { $score_white }

    { confirmation-count-down }
yes = ใช่
no = ไม่

# Fouls
equal = เท่ากัน

# Game Info
refresh = รีเฟรช
refreshing = กำลังรีเฟรช...
settings = การตั้งค่า
none = ไม่มี
game-number-error = ข้อผิดพลาด ({ $game_number })
next-game-number-error = ข้อผิดพลาด ({ $next_game_number })
last-game-next-game = เกมที่แล้ว: { $prev_game },
    เกมถัดไป: { $next_game }
black-team-white-team = ทีมดำ: { $black_team }
    ทีมขาว: { $white_team }
game-length-ot-allowed = ความยาวครึ่ง: { $half_length }
         ความยาวพักครึ่ง: { $half_time_length }
         อนุญาตต่อเวลา: { $overtime }
overtime-details = ความยาวพักก่อนต่อเวลา: { $pre_overtime }
             ความยาวครึ่งต่อเวลา: { $overtime_len }
             ความยาวพักครึ่งต่อเวลา: { $overtime_half_time_len }
sd-allowed = อนุญาตซัดเดนเดธ: { $sd }
pre-sd = ความยาวพักก่อนซัดเดนเดธ: { $pre_sd_len }
team-to-len = ระยะเวลาหมดเวลาทีม: { $to_len }
time-btwn-games = เวลาระหว่างเกมตามกำหนด: { $time_btwn }
min-brk-btwn-games = เวลาระหว่างเกมขั้นต่ำ: { $min_brk_time }


# List Selecters
select-event = เลือกกิจกรรม
select-court = เลือกสนาม
select-game = เลือกเกม

# Main View
add-warning = เพิ่มคำเตือน
add-foul = เพิ่มฟาวล์
start-now = เริ่มเลย
end-timeout = สิ้นสุดหมดเวลา
warnings = คำเตือน
penalties = การลงโทษ
dark-score-line-1 = คะแนน
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = คะแนน
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = การลงโทษดำ
white-penalties = การลงโทษขาว

# Score edit
final-score = กรุณาป้อนคะแนนสุดท้าย
confirmation-count-down = หมายเหตุ: คะแนนที่ไม่เปลี่ยนแปลงจะได้รับการยืนยันอัตโนมัติใน { $countdown }

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = สิ้นสุด
end-timeout-line-2 = { timeout }
switch-to = สลับไป
ref = ผู้ตัดสิน
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = ลูก
penalty-shot-line-2 = โทษ
pen-shot = ลูกโทษ
## Penalty string
served = รับโทษแล้ว
penalty = #{$player_number} - {$time ->
        [pending] รอ
        [served] รับโทษแล้ว
        [total-dismissal] ไล่ออก
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
infraction = ฟาวล์: {$infraction}
## Config String
error = ข้อผิดพลาด ({ $number })
two-games = เกมที่แล้ว: { $prev_game },  เกมถัดไป: { $next_game }
one-game = เกม: { $game }
teams = ทีม { -dark-team-name }: { $dark_team }
    ทีม { -light-team-name }: { $light_team }
game-config = ความยาวครึ่ง: { $half_len },  ความยาวพักครึ่ง: { $half_time_len }
    อนุญาตซัดเดนเดธ: { $sd_allowed },  อนุญาตต่อเวลา: { $ot_allowed }
team-timeouts-per-half = หมดเวลาทีมที่อนุญาตต่อครึ่ง: { $team_timeouts }
team-timeouts-per-game = หมดเวลาทีมที่อนุญาตต่อเกม: { $team_timeouts }
stop-clock-last-2 = หยุดนาฬิกาใน 2 นาทีสุดท้าย: { $stop_clock }
ref-list = ผู้ตัดสินหลัก: { $chief_ref }
    จับเวลา: { $timer }
    ผู้ตัดสินน้ำ 1: { $water_ref_1 }
    ผู้ตัดสินน้ำ 2: { $water_ref_2 }
    ผู้ตัดสินน้ำ 3: { $water_ref_3 }
team-ref-list = ผู้ตัดสิน: { $ref_team }
    ผู้จับเวลา/บันทึกคะแนน: { $ts_keeper_team }
unknown = ไม่ทราบ
## Game time button
next-game = เกมถัดไป
first-half = ครึ่งแรก
half-time = พักครึ่ง
second-half = ครึ่งที่สอง
pre-ot-break-full = พักก่อนต่อเวลา
overtime-first-half = ต่อเวลาครึ่งแรก
overtime-half-time = พักครึ่งต่อเวลา
overtime-second-half = ต่อเวลาครึ่งที่สอง
pre-sudden-death-break = พักก่อนซัดเดนเดธ
sudden-death = ซัดเดนเดธ
ot-first-half = ต่อเวลาครึ่ง 1
ot-half-time = พักครึ่งต่อเวลา
ot-2nd-half = ต่อเวลาครึ่ง 2
white-timeout-short = ขาว หมดเวลา
white-timeout-full = หมดเวลาทีมขาว
black-timeout-short = ดำ หมดเวลา
black-timeout-full = หมดเวลาทีมดำ
ref-timeout-short = ผู้ตัดสิน หมดเวลา
penalty-shot-short = ลูกโทษ
## Make warning container
team-warning-abreviation = ท
## Make time editor
zero = ศูนย์

# Time edit
game-time = เวลาเกม
timeout = หมดเวลา
Note-Game-time-is-paused = หมายเหตุ: เวลาเกมหยุดชั่วคราวขณะอยู่ในหน้านี้

# Warning Fouls Summary
fouls = ฟาวล์
edit-warnings = แก้ไขคำเตือน
edit-fouls = แก้ไขฟาวล์

# Warnings
black-warnings = คำเตือนดำ
white-warnings = คำเตือนขาว

# Message
player-number = หมายเลข
    ผู้เล่น:
game-number = หมายเลข
    เกม:
num-tos-per-half = จำนวนหมดเวลา
    ต่อครึ่ง:
num-tos-per-game = จำนวนหมดเวลา
    ต่อเกม:

# Sound Controller - mod
off = ปิด
low = ต่ำ
medium = กลาง
high = สูง
max = สูงสุด

# Config
hockey6v6 = ฮอกกี้ 6ต่อ6
hockey3v3 = ฮอกกี้ 3ต่อ3
rugby = รักบี้

# Infractions
stick-foul = ฟาวล์ไม้ฮอกกี้
illegal-advance = รุกผิดกติกา
sub-foul = ฟาวล์เปลี่ยนตัว
illegal-stoppage = หยุดผิดกติกา
out-of-bounds = นอกเขต
grabbing-the-wall = จับกำแพง
obstruction = การขัดขวาง
delay-of-game = ประวิงเวลา
unsportsmanlike = ไม่มีน้ำใจนักกีฬา
free-arm = แขนอิสระ
false-start = ออกตัวผิด
