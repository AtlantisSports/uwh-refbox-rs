# Definisi untuk file terjemahan
-dark-team-name = Hitam
dark-team-name-caps = HITAM

-light-team-name = Putih
light-team-name-caps = PUTIH

# Multihalaman
done = SELESAI
restart-to-apply = MULAI ULANG UNTUK MENERAPKAN
cancel = BATAL
delete = HAPUS
back = KEMBALI
apply = TERAPKAN
save = SIMPAN
user-options = OPSI PENGGUNA
new = BARU

# Edit Penalti
total-dismissal = KELUAR
penalty-kind = {$kind ->
    [thirty-seconds] 30dtk
    [one-minute] 1mnt
    [two-minutes] 2mnt
    [four-minutes] 4mnt
    [five-minutes] 5mnt
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# Edit Time-out Tim
timeout-length = TIME-OUT TIM
    DURASI

# Tambah Peringatan
team-warning = PERINGATAN
    TIM
team-warning-line-1 = PERINGATAN
team-warning-line-2 = TIM

# Konfigurasi
none-selected = Tidak Ada Dipilih
loading = Memuat...
game-select = Pertandingan:
game-options = OPSI PERTANDINGAN
app-options = OPSI APLIKASI
display-options = OPSI TAMPILAN
open-new-display = BUKA TAMPILAN BARU
front-display-layout = DISPLAY LAYOUT
layout-default = DEFAULT
layout-classic = CLASSIC
layout-big-time = BIG TIME
layout-corners = CORNERS
layout-scores-only = SCORES ONLY
sound-options = OPSI SUARA
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
sound-settings = PENGATURAN SUARA
beep-test-edit-levels = UBAH LEVEL
app-mode = MODE
    APLIKASI
hide-time-for-last-15-seconds = SEMBUNYIKAN WAKTU
    15 DETIK TERAKHIR
player-display-brightness = KECERAHAN
    TAMPILAN PEMAIN
confirm-score-at-game-end = KONFIRMASI SKOR
    DI AKHIR PERTANDINGAN
track-cap-number-of-scorer = LACAK NOMOR TOPI
    PENCETAK GOL
event = ACARA:
track-fouls-and-warnings = LACAK PELANGGARAN
    DAN PERINGATAN
show-behind-schedule-time = TAMPILKAN KELEWATAN
delay = TERLAMBAT
court = LAPANGAN:
single-half = BABAK
    TUNGGAL:
half-length-full = DURASI BABAK:
game-length = DURASI PERTANDINGAN:
overtime-allowed = PERPANJANGAN WAKTU
    DIIZINKAN:
sudden-death-allowed = SUDDEN DEATH
    DIIZINKAN:
half-time-length = DURASI JEDA
    BABAK:
pre-ot-break-length = DURASI JEDA
    SEBELUM PW:
pre-sd-break-length = DURASI JEDA
    SEBELUM SD:
nominal-break-between-games = JEDA NOMINAL
    ANTAR PERTANDINGAN:
ot-half-length = DURASI BABAK
    PERPANJANGAN:
timeouts-counted-per = TIME-OUT
    DIHITUNG PER:
game = PERTANDINGAN
half = BABAK
minimum-brk-btwn-games = JEDA MINIMUM
    ANTAR PERTANDINGAN:
ot-half-time-length = DURASI JEDA
    PERPANJANGAN WAKTU
using-portal = MENGGUNAKAN { $portal }PORTAL:
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
length-of-half-during-regular-play = Durasi satu babak selama pertandingan reguler
half-time-lenght = DUR JEDA BABAK
length-of-half-time-period = Durasi periode jeda babak
nom-break = JEDA NOMINAL
game-block = BLOK PERTANDINGAN
game-block-help = Waktu dari awal satu pertandingan hingga awal pertandingan berikutnya
game-block-too-short = Terlalu singkat untuk pertandingan ditambah jeda minimum
game-block-tight = Terlalu ketat — time-out tim bisa membuat pertandingan melewati slot mereka
system-will-keep-game-times-spaced = Sistem akan mencoba menjaga waktu mulai pertandingan agar merata, dengan total waktu dari satu mulai ke mulai berikutnya adalah 2 × [Durasi Babak] + [Durasi Jeda Babak] + [Waktu Nominal Antar Pertandingan] (contoh: jika [Durasi Babak] = 15 menit, [Durasi Jeda Babak] = 3 menit, dan [Waktu Nominal Antar Pertandingan] = 12 menit, waktu dari mulai satu pertandingan ke berikutnya adalah 45 menit. Setiap time-out yang diambil, atau penghentian jam lainnya, akan mengurangi waktu 12 menit tersebut hingga mencapai nilai waktu minimum antar pertandingan).
min-break = JEDA MINIMUM
min-time-btwn-games = Jika suatu pertandingan berjalan lebih lama dari jadwal, ini adalah waktu minimum antar pertandingan yang akan dialokasikan oleh sistem. Jika pertandingan tertunda, sistem akan secara otomatis mencoba mengejar pada pertandingan berikutnya, selalu menghormati waktu minimum antar pertandingan ini.
pre-ot-break-abreviated = JEDA SEBELUM PW
pre-sd-brk = Jika perpanjangan waktu diizinkan dan diperlukan, ini adalah durasi jeda antara Babak Kedua dan Babak Pertama Perpanjangan Waktu
ot-half-len = DUR BABAK PW
time-during-ot = Durasi satu babak selama perpanjangan waktu
ot-half-tm-len = DUR JEDA PW
len-of-overtime-halftime = Durasi jeda babak perpanjangan waktu
pre-sd-break = JEDA SEBELUM SD
pre-sd-len = Durasi jeda antara periode permainan sebelumnya dan Sudden Death
language = BAHASA
this-language = INDONESIA
portal-login-code = KODE
portal-login-instructions = Silakan buka { $portal } Portal >> Manajemen Acara >> Manajemen Wasit, klik tombol + untuk menambahkan Refbox baru, dan masukkan ID Refbox ini:
    { $id }

    { $portal } Portal kemudian akan memberikan kode konfirmasi untuk dimasukkan di sebelah kiri menggunakan papan angka.
    Tekan Selesai setelah Anda memasukkan kode

help = BANTUAN:

# Konfirmasi
game-configuration-can-not-be-changed = Konfigurasi pertandingan tidak dapat diubah saat pertandingan sedang berlangsung.

    Apa yang ingin Anda lakukan?
apply-this-game-number-change = Bagaimana Anda ingin menerapkan perubahan nomor pertandingan ini?
portal-enabled = Saat { $portal }PORTAL diaktifkan, semua kolom harus diisi.
mode-switch-portal-tenant = Mengubah mode dari { $from_mode } ke { $to_mode } akan menonaktifkan tautan ke { $from_portal }PORTAL dan Anda harus menghubungkan kembali ke { $to_portal }PORTAL.
uwhportal-token-invalid-code = Kode yang dimasukkan tidak valid.
    Silakan coba lagi.
uwhportal-token-no-pending-link = Portal tidak mengharapkan koneksi.
    Silakan coba lagi.
go-back-to-editor = KEMBALI KE EDITOR
discard-changes = BUANG PERUBAHAN
end-current-game-and-apply-changes = AKHIRI PERTANDINGAN DAN TERAPKAN PERUBAHAN
end-current-game-and-apply-change = AKHIRI PERTANDINGAN DAN TERAPKAN PERUBAHAN
keep-current-game-and-apply-change = LANJUTKAN PERTANDINGAN DAN TERAPKAN PERUBAHAN
ok = OKE
confirm-score = Apakah skor ini sudah benar?
    Konfirmasi dengan wasit kepala.

    Hitam: { $score_black }        Putih: { $score_white }

    { confirmation-count-down }
yes = YA
no = TIDAK

# Pelanggaran
equal = SERI

# Info Pertandingan
refresh = SEGARKAN
refreshing = MENYEGARKAN...
settings = PENGATURAN
none = Tidak Ada
game-number-error = Galat ({ $game_number })
next-game-number-error = Galat ({ $next_game_number })
last-game-next-game = Pertandingan Terakhir: { $prev_game },
    Pertandingan Berikutnya: { $next_game }
black-team-white-team = Tim Hitam: { $black_team }
    Tim Putih: { $white_team }
game-length-ot-allowed = Durasi Babak: { $half_length }
         Durasi Jeda Babak: { $half_time_length }
         Perpanjangan Waktu Diizinkan: { $overtime }
overtime-details = Durasi Jeda Sebelum Perpanjangan Waktu: { $pre_overtime }
             Durasi Babak Perpanjangan Waktu: { $overtime_len }
             Durasi Jeda Perpanjangan Waktu: { $overtime_half_time_len }
sd-allowed = Sudden Death Diizinkan: { $sd }
pre-sd = Durasi Jeda Sebelum Sudden Death: { $pre_sd_len }
team-to-len = Durasi Time-out Tim: { $to_len }
time-btwn-games = Waktu Nominal Antar Pertandingan: { $time_btwn }
game-block-info = Blok Permainan: { $game_block }
min-brk-btwn-games = Waktu Minimum Antar Pertandingan: { $min_brk_time }


# Pemilih Daftar
select-event = PILIH ACARA
select-court = PILIH LAPANGAN
select-game = PILIH PERTANDINGAN

# Tampilan Utama
add-warning = TAMBAH PERINGATAN
add-foul = TAMBAH PELANGGARAN
start-now = MULAI SEKARANG
end-timeout = AKHIRI TIME-OUT
warnings = PERINGATAN
penalties = PENALTI
dark-score-line-1 = SKOR
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = SKOR
light-score-line-2 = { light-team-name-caps }

# Penalti
black-penalties = PENALTI HITAM
white-penalties = PENALTI PUTIH

# Edit Skor
final-score = Masukkan skor akhir
confirmation-count-down = Catatan: Skor yang tidak berubah akan dikonfirmasi secara otomatis dalam { $countdown }

# Elemen Bersama
## Pita time-out
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
pen-shot = TBK PENALTI
## Teks penalti
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
## Teks konfigurasi
error = Galat ({ $number })
two-games = Pertandingan Terakhir: { $prev_game },  Pertandingan Berikutnya: { $next_game }
one-game = Pertandingan: { $game }
teams = Tim { -dark-team-name }: { $dark_team }
    Tim { -light-team-name }: { $light_team }
game-config = Durasi Babak: { $half_len },  Durasi Jeda Babak: { $half_time_len }
    Sudden Death Diizinkan: { $sd_allowed },  Perpanjangan Waktu Diizinkan: { $ot_allowed }
team-timeouts-per-half = Time-out Tim yang Diizinkan Per Babak: { $team_timeouts }
team-timeouts-per-game = Time-out Tim yang Diizinkan Per Pertandingan: { $team_timeouts }
stop-clock-last-2 = Hentikan Jam di 2 Menit Terakhir: { $stop_clock }
ref-list = Wasit Kepala: { $chief_ref }
    Pencatat Waktu: { $timer }
    Wasit Air 1: { $water_ref_1 }
    Wasit Air 2: { $water_ref_2 }
    Wasit Air 3: { $water_ref_3 }
team-ref-list = Wasit: { $ref_team }
    Pencatat Waktu/Skor: { $ts_keeper_team }
unknown = Tidak Diketahui
## Tombol waktu pertandingan
next-game = PERTANDINGAN BERIKUTNYA
first-half = BABAK PERTAMA
half-time = JEDA BABAK
second-half = BABAK KEDUA
pre-ot-break-full = JEDA SEBELUM PERPANJANGAN WAKTU
overtime-first-half = BABAK PERTAMA PERPANJANGAN WAKTU
overtime-half-time = JEDA PERPANJANGAN WAKTU
overtime-second-half = BABAK KEDUA PERPANJANGAN WAKTU
pre-sudden-death-break = JEDA SEBELUM SUDDEN DEATH
sudden-death = SUDDEN DEATH
ot-first-half = BABAK 1 PW
ot-half-time = JEDA PW
ot-2nd-half = BABAK 2 PW
white-timeout-short = T/O PTH
white-timeout-full = TIME-OUT PUTIH
black-timeout-short = T/O HTM
black-timeout-full = TIME-OUT HITAM
ref-timeout-short = T/O WASIT
penalty-shot-short = TBK PEN
## Kontainer peringatan tim
team-warning-abreviation = T
## Editor waktu
zero = = 0

# Edit Waktu
game-time = WAKTU PERTANDINGAN
timeout = TIME-OUT
Note-Game-time-is-paused = Catatan: Waktu pertandingan dijeda saat berada di layar ini

# Ringkasan Pelanggaran dan Peringatan
fouls = PELANGGARAN
edit-warnings = EDIT PERINGATAN
edit-fouls = EDIT PELANGGARAN

# Peringatan
black-warnings = PERINGATAN HITAM
white-warnings = PERINGATAN PUTIH

# Pesan
player-number = NOMOR
    PEMAIN:
game-number = NOMOR
    PERTANDINGAN:
num-tos-per-half = JUMLAH TIME-OUT
    TIM PER BABAK:
num-tos-per-game = JUMLAH TIME-OUT
    TIM PER PERTANDINGAN:

# Pengontrol Suara - mod
off = MATI
low = RENDAH
medium = SEDANG
high = TINGGI
max = MAKS

# Konfigurasi
hockey6v6 = HOKI6V6
hockey3v3 = HOKI3V3
rugby = RUGBY
beep-test = BEEP TEST

# Beep-test screen
beep-test-pre = PRA
beep-test-top-time-label = WAKTU
beep-test-top-level-label = LEVEL
beep-test-top-lap-label = PUTARAN
beep-test-start = MULAI
beep-test-pause = JEDA
beep-test-resume = LANJUTKAN
beep-test-reset = ATUR ULANG
beep-test-column-level = LEVEL
beep-test-column-count = JUMLAH
beep-test-column-duration = DURASI
beep-test-edit-selected = Level { $level }
beep-test-edit-time = WAKTU
beep-test-edit-count = JUMLAH
beep-test-edit-new = TAMBAH LEVEL
beep-test-edit-remove = HAPUS LEVEL

# Pelanggaran
stick-foul = Pelanggaran Stik
illegal-advance = Maju Ilegal
sub-foul = Pelanggaran Pergantian Pemain
illegal-stoppage = Penghentian Ilegal
out-of-bounds = Di Luar Batas
grabbing-the-wall = Memegang Dinding
obstruction = Penghalangan
delay-of-game = Mengulur Waktu
unsportsmanlike = Perilaku Tidak Sportif
free-arm = Lengan Bebas
false-start = Start Curang


# Portal Health Indicator
portal-summary-title = STATUS { $portal } PORTAL
portal-row-token-expired = Login Portal kedaluwarsa — ketuk untuk login ulang
portal-row-stuck = Pertandingan { $game } Galat kirim skor, ketuk untuk perbaiki
portal-row-pending = Pertandingan { $game } Skor belum terkirim, ketuk untuk coba lagi
portal-row-attempt-suffix = (percobaan { $attempts })
portal-row-recent = Pertandingan { $game } · Dikirim { $mins } mnt lalu
portal-action-force-submit = Coba lagi kirim hasil pertandingan ini
portal-action-discard = Buang hasil pertandingan ini
portal-action-discard-confirm = KETUK LAGI UNTUK KONFIRMASI BUANG
portal-page-title-attention = Galat pengiriman Pertandingan { $game }
portal-page-attention-info = Hasil pertandingan belum diterima oleh { $portal } Portal
portal-page-attention-score = Hasil pertandingan tersimpan: Putih { $white } - Hitam { $black }
portal-page-attention-remediation = Anda dapat Coba Lagi jika koneksi sudah terverifikasi, atau buang untuk menghapus galat
portal-advisory-at-game-end = Masalah Portal terdeteksi. Skor tetap akan diantrekan — temui admin untuk menyelesaikan.

# 2 Halves / 1 Period selector (Half Length editor)
two-halves = 2 BABAK
one-period = 1 PERIODE
game-len = DURASI PERTANDINGAN
length-of-game-during-regular-play = Durasi permainan selama pertandingan reguler

# Single-period (1 Period) info variants — Game Length, no half-time line
game-config-single-half = Durasi Pertandingan: { $half_len }
    Sudden Death Diizinkan: { $sd_allowed },  Perpanjangan Waktu Diizinkan: { $ot_allowed }
game-length-ot-allowed-single-half = Durasi Pertandingan: { $half_length }
         Perpanjangan Waktu Diizinkan: { $overtime }
