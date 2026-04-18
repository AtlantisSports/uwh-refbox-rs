# Definitions for the translation file to use
-dark-team-name = Hitam
dark-team-name-caps = HITAM

-light-team-name = Putih
light-team-name-caps = PUTIH

# Multipage
done = SELESAI
restart-to-apply = MULAI ULANG
cancel = BATAL
delete = HAPUS
back = KEMBALI
new = BARU

# Penalty Edit
total-dismissal = TD
penalty-kind = {$kind ->
    [thirty-seconds] 30d
    [one-minute] 1m
    [two-minutes] 2m
    [four-minutes] 4m
    [five-minutes] 5m
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# Team Timeout Edit
timeout-length = DURASI
    WAKTU JEDA

# Warning Add
team-warning = PERINGATAN
    TIM
team-warning-line-1 = PERINGATAN
team-warning-line-2 = TIM

# Configuration
none-selected = Tidak Ada Dipilih
loading = Memuat...
game-select = Pertandingan:
game-options = OPSI PERTANDINGAN
app-options = OPSI APLIKASI
display-options = OPSI TAMPILAN
sound-options = OPSI SUARA
app-mode = MODE
    APLIKASI
hide-time-for-last-15-seconds = SEMBUNYIKAN WAKTU
    15 DETIK TERAKHIR
player-display-brightness = KECERAHAN
    TAMPILAN PEMAIN
confirm-score-at-game-end = KONFIRMASI SKOR
    DI AKHIR PERTANDINGAN
track-cap-number-of-scorer = LACAK NOMOR
    TOPI PENCETAK GOL
event = ACARA:
track-fouls-and-warnings = LACAK PELANGGARAN
    DAN PERINGATAN
court = LAPANGAN:
single-half = BABAK
    TUNGGAL:
half-length-full = DURASI BABAK:
game-length = DURASI PERTANDINGAN:
overtime-allowed = OVERTIME
    DIIZINKAN:
sudden-death-allowed = SUDDEN DEATH
    DIIZINKAN:
half-time-length = DURASI
    JEDA:
pre-ot-break-length = DURASI JEDA
    SEBELUM OT:
pre-sd-break-length = DURASI JEDA
    SEBELUM SD:
nominal-break-between-games = JEDA NOMINAL
    ANTAR PERTANDINGAN:
ot-half-length = DURASI BABAK
    OVERTIME:
timeouts-counted-per = WAKTU JEDA
    DIHITUNG PER:
game = PERTANDINGAN
half = BABAK
minimum-brk-btwn-games = JEDA MINIMUM
    ANTAR PERTANDINGAN:
ot-half-time-length = DURASI JEDA
    OVERTIME
using-uwh-portal = MENGGUNAKAN UWHPORTAL:
starting-sides = SISI AWAL
sound-enabled = SUARA
    DIAKTIFKAN:
whistle-volume = VOLUME
    PELUIT:
manage-remotes = KELOLA REMOTE
whistle-enabled = PELUIT
    DIAKTIFKAN:
above-water-volume = VOLUME
    DI ATAS AIR:
auto-sound-start-play = SUARA OTOMATIS
    MULAI MAIN:
buzzer-sound = SUARA
    BUZZER:
underwater-volume = VOLUME
    DI BAWAH AIR:
auto-sound-stop-play = SUARA OTOMATIS
    BERHENTI MAIN:
alarm-button = TOMBOL
    ALARM:
alarm = ALARM
hold-to-test = TAHAN UNTUK UJI
or-press-spacebar = Atau Tekan Spacebar
or-hold-spacebar = Atau Tahan Spacebar
game-info = INFO PERTANDINGAN
remotes = REMOTE
default = BAWAAN
sound = SUARA: { $sound_text }
brightness = { $brightness ->
        *[Low] RENDAH
        [Medium] SEDANG
        [High] TINGGI
        [Outdoor] LUAR RUANGAN
    }

waiting = MENUNGGU
add = TAMBAH
half-length = DUR BABAK
length-of-half-during-regular-play = Durasi satu babak selama permainan reguler
half-time-lenght = DUR JEDA
length-of-half-time-period = Durasi periode jeda
nom-break = JEDA NOMINAL
system-will-keep-game-times-spaced = Sistem akan mencoba menjaga waktu mulai pertandingan agar merata, dengan total waktu dari satu mulai ke mulai berikutnya adalah 2 × [Durasi Babak] + [Durasi Jeda] + [Waktu Nominal Antar Pertandingan] (contoh: jika [Durasi Babak] = 15 menit, [Durasi Jeda] = 3 menit dan [Waktu Nominal Antar Pertandingan] = 12 menit, waktu dari mulai satu pertandingan ke berikutnya adalah 45 menit. Waktu jeda atau penghentian jam lainnya akan mengurangi 12 menit hingga mencapai waktu minimum antar pertandingan).
min-break = JEDA MINIMUM
min-time-btwn-games = Jika pertandingan berjalan lebih lama dari jadwal, ini adalah waktu minimum antar pertandingan yang akan dialokasikan sistem. Jika pertandingan tertunda, sistem akan secara otomatis mencoba mengejar pada pertandingan berikutnya, selalu menghormati waktu minimum antar pertandingan ini.
pre-ot-break-abreviated = JEDA SEBELUM OT
pre-sd-brk = Jika overtime diizinkan dan diperlukan, ini adalah durasi jeda antara Babak Kedua dan Babak Pertama Overtime
ot-half-len = DUR BABAK OT
time-during-ot = Durasi satu babak selama overtime
ot-half-tm-len = DUR JEDA OT
len-of-overtime-halftime = Durasi jeda Overtime
pre-sd-break = JEDA SEBELUM SD
pre-sd-len = Durasi jeda antara periode permainan sebelumnya dan Sudden Death
language = BAHASA
this-language = INDONESIA
portal-login-code = KODE
portal-login-instructions = Silakan buka UWH Portal >> Manajemen Acara >> Manajemen Wasit, klik tombol + untuk menambahkan Refbox baru, dan masukkan ID Refbox ini:
    { $id }

    UWH Portal kemudian akan memberikan kode konfirmasi untuk dimasukkan di sebelah kiri menggunakan papan angka.
    Tekan Selesai setelah Anda memasukkan kode

help = BANTUAN:

# Confirmation
game-configuration-can-not-be-changed = Konfigurasi pertandingan tidak dapat diubah saat pertandingan sedang berlangsung.

    Apa yang ingin Anda lakukan?
apply-this-game-number-change = Bagaimana Anda ingin menerapkan perubahan nomor pertandingan ini?
UWHPortal-enabled = Saat UWHPortal diaktifkan, semua kolom harus diisi.
uwhportal-token-invalid-code = Kode yang dimasukkan tidak valid.
    Silakan coba lagi.
uwhportal-token-no-pending-link = Portal tidak mengharapkan koneksi.
    Silakan coba lagi.
go-back-to-editor = KEMBALI KE EDITOR
discard-changes = BUANG PERUBAHAN
end-current-game-and-apply-changes = AKHIRI PERTANDINGAN DAN TERAPKAN PERUBAHAN
end-current-game-and-apply-change = AKHIRI PERTANDINGAN DAN TERAPKAN PERUBAHAN
keep-current-game-and-apply-change = PERTAHANKAN PERTANDINGAN DAN TERAPKAN PERUBAHAN
ok = OKE
confirm-score = Apakah skor ini sudah benar?
    Konfirmasi dengan wasit kepala.

    Hitam: { $score_black }        Putih: { $score_white }

    { confirmation-count-down }
yes = YA
no = TIDAK

# Fouls
equal = SERI

# Game Info
refresh = SEGARKAN
refreshing = MENYEGARKAN...
settings = PENGATURAN
none = Tidak Ada
game-number-error = Error ({ $game_number })
next-game-number-error = Error ({ $next_game_number })
last-game-next-game = Pertandingan Terakhir: { $prev_game },
    Pertandingan Berikutnya: { $next_game }
black-team-white-team = Tim Hitam: { $black_team }
    Tim Putih: { $white_team }
game-length-ot-allowed = Durasi Babak: { $half_length }
         Durasi Jeda: { $half_time_length }
         Overtime Diizinkan: { $overtime }
overtime-details = Durasi Jeda Sebelum Overtime: { $pre_overtime }
             Durasi Babak Overtime: { $overtime_len }
             Durasi Jeda Overtime: { $overtime_half_time_len }
sd-allowed = Sudden Death Diizinkan: { $sd }
pre-sd = Durasi Jeda Sebelum Sudden Death: { $pre_sd_len }
team-to-len = Durasi Waktu Jeda Tim: { $to_len }
time-btwn-games = Waktu Nominal Antar Pertandingan: { $time_btwn }
min-brk-btwn-games = Waktu Minimum Antar Pertandingan: { $min_brk_time }


# List Selecters
select-event = PILIH ACARA
select-court = PILIH LAPANGAN
select-game = PILIH PERTANDINGAN

# Main View
add-warning = TAMBAH PERINGATAN
add-foul = TAMBAH PELANGGARAN
start-now = MULAI SEKARANG
end-timeout = AKHIRI WAKTU JEDA
warnings = PERINGATAN
penalties = PENALTI
dark-score-line-1 = SKOR
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = SKOR
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = PENALTI HITAM
white-penalties = PENALTI PUTIH

# Score edit
final-score = Masukkan skor akhir
confirmation-count-down = Catatan: Skor yang tidak berubah akan dikonfirmasi secara otomatis dalam { $countdown }

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = AKHIRI
end-timeout-line-2 = { timeout }
switch-to = BERALIH KE
ref = WASIT
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = TEMBAKAN
penalty-shot-line-2 = PENALTI
pen-shot = TEMBAKAN PEN
## Penalty string
served = Dijalani
penalty = #{$player_number} - {$time ->
        [pending] Menunggu
        [served] Dijalani
        [total-dismissal] Dikeluarkan
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
infraction = Pelanggaran: {$infraction}
## Config String
error = Error ({ $number })
two-games = Pertandingan Terakhir: { $prev_game },  Pertandingan Berikutnya: { $next_game }
one-game = Pertandingan: { $game }
teams = Tim { -dark-team-name }: { $dark_team }
    Tim { -light-team-name }: { $light_team }
game-config = Durasi Babak: { $half_len },  Durasi Jeda: { $half_time_len }
    Sudden Death Diizinkan: { $sd_allowed },  Overtime Diizinkan: { $ot_allowed }
team-timeouts-per-half = Waktu Jeda Tim yang Diizinkan Per Babak: { $team_timeouts }
team-timeouts-per-game = Waktu Jeda Tim yang Diizinkan Per Pertandingan: { $team_timeouts }
stop-clock-last-2 = Hentikan Jam di 2 Menit Terakhir: { $stop_clock }
ref-list = Wasit Kepala: { $chief_ref }
    Pencatat Waktu: { $timer }
    Wasit Air 1: { $water_ref_1 }
    Wasit Air 2: { $water_ref_2 }
    Wasit Air 3: { $water_ref_3 }
team-ref-list = Wasit: { $ref_team }
    Penjaga Waktu/Skor: { $ts_keeper_team }
unknown = Tidak Diketahui
## Game time button
next-game = PERTANDINGAN BERIKUTNYA
first-half = BABAK PERTAMA
half-time = JEDA
second-half = BABAK KEDUA
pre-ot-break-full = JEDA SEBELUM OVERTIME
overtime-first-half = BABAK PERTAMA OT
overtime-half-time = JEDA OVERTIME
overtime-second-half = BABAK KEDUA OT
pre-sudden-death-break = JEDA SEBELUM SUDDEN DEATH
sudden-death = SUDDEN DEATH
ot-first-half = BABAK 1 OT
ot-half-time = JEDA OT
ot-2nd-half = BABAK 2 OT
white-timeout-short = JEDA PTH
white-timeout-full = WAKTU JEDA PUTIH
black-timeout-short = JEDA HTM
black-timeout-full = WAKTU JEDA HITAM
ref-timeout-short = JEDA WST
penalty-shot-short = TBK PEN
## Make warning container
team-warning-abreviation = T
## Make time editor
zero = NOL

# Time edit
game-time = WAKTU PERTANDINGAN
timeout = WAKTU JEDA
Note-Game-time-is-paused = Catatan: Waktu pertandingan dijeda saat berada di layar ini

# Warning Fouls Summary
fouls = PELANGGARAN
edit-warnings = EDIT PERINGATAN
edit-fouls = EDIT PELANGGARAN

# Warnings
black-warnings = PERINGATAN HITAM
white-warnings = PERINGATAN PUTIH

# Message
player-number = NOMOR
    PEMAIN:
game-number = NOMOR
    PERTANDINGAN:
num-tos-per-half = JUMLAH WAKTU JEDA
    PER BABAK:
num-tos-per-game = JUMLAH WAKTU JEDA
    PER PERTANDINGAN:

# Sound Controller - mod
off = MATI
low = RENDAH
medium = SEDANG
high = TINGGI
max = MAKS

# Config
hockey6v6 = HOKI 6V6
hockey3v3 = HOKI 3V3
rugby = RUGBY

# Infractions
stick-foul = Pelanggaran Tongkat
illegal-advance = Maju Ilegal
sub-foul = Pelanggaran Pergantian
illegal-stoppage = Penghentian Ilegal
out-of-bounds = Keluar Batas
grabbing-the-wall = Memegang Dinding
obstruction = Penghalangan
delay-of-game = Penundaan Pertandingan
unsportsmanlike = Tidak Sportif
free-arm = Lengan Bebas
false-start = Start Palsu
