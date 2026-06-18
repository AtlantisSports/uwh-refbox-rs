# Takrifan untuk fail terjemahan digunakan
-dark-team-name = Hitam
dark-team-name-caps = HITAM

-light-team-name = Putih
light-team-name-caps = PUTIH

# Berbilang Halaman
done = SELESAI
restart-to-apply = MULAKAN SEMULA UNTUK GUNA
cancel = BATAL
delete = PADAM
back = KEMBALI
apply = GUNA
save = SIMPAN
user-options = PILIHAN PENGGUNA
new = BARU

# Edit Penalti
total-dismissal = PT
penalty-kind = {$kind ->
    [thirty-seconds] 30s
    [one-minute] 1m
    [two-minutes] 2m
    [four-minutes] 4m
    [five-minutes] 5m
    [total-dismissal] { total-dismissal }
   *[other] {$kind}
}

# Edit Masa Rehat Pasukan
timeout-length = MASA REHAT
    PASUKAN:
team-timeout-count = BILANGAN
    MASA REHAT:

# Tambah Amaran
team-warning = AMARAN
    PASUKAN
team-warning-line-1 = AMARAN
team-warning-line-2 = PASUKAN

# Konfigurasi
none-selected = Tiada Dipilih
loading = Memuatkan...
game-select = PERLAWANAN:
game-options = PILIHAN PERLAWANAN
app-options = PILIHAN APLIKASI
display-options = PILIHAN PAPARAN
open-new-display = BUKA PAPARAN BARU
front-display-layout = DISPLAY LAYOUT
layout-default = DEFAULT
layout-classic = CLASSIC
layout-big-time = BIG TIME
layout-corners = CORNERS
layout-scores-only = SCORES ONLY
sound-options = PILIHAN BUNYI
view-mode = VIEW MODE
display-mode-light = LIGHT
display-mode-dark = DARK
display-mode-high-contrast = HIGH CONTRAST
sound-settings = TETAPAN BUNYI
beep-test-edit-levels = SUNTING TAHAP
app-mode = MOD
    APLIKASI
hide-time-for-last-15-seconds = SEMBUNYIKAN MASA
    15 SAAT TERAKHIR
player-display-brightness = KECERAHAN PAPARAN
    PEMAIN
confirm-score-at-game-end = SAHKAN MARKAH
    AKHIR PERLAWANAN
track-cap-number-of-scorer = REKOD NOMBOR PEMAIN
    PENCETAK GOL
event = ACARA:
track-fouls-and-warnings = REKOD KESALAHAN
    DAN AMARAN
show-behind-schedule-time = TUNJUK KELEWATAN
delay = LEWAT
court = GELANGGANG:
single-half = SEPARUH
    TUNGGAL:
half-length-full = PANJANG SEPARUH:
game-length = PANJANG PERLAWANAN:
overtime-allowed = MASA TAMBAHAN
    DIBENARKAN:
sudden-death-allowed = SUDDEN DEATH
    DIBENARKAN:
half-time-length = PANJANG REHAT
    SEPARUH MASA:
pre-ot-break-length = PANJANG REHAT
    PRA-MASA TAMBAHAN:
pre-sd-break-length = PANJANG REHAT
    PRA-SUDDEN DEATH:
nominal-break-between-games = REHAT NOMINAL
    ANTARA PERLAWANAN:
ot-half-length = PANJANG SEPARUH
    MASA TAMBAHAN:
timeouts-counted-per = MASA REHAT
    DIKIRA PER:
game = PERLAWANAN
half = SEPARUH
minimum-brk-btwn-games = REHAT MINIMUM
    ANTARA PERLAWANAN:
ot-half-time-length = PANJANG REHAT SEPARUH
    MASA TAMBAHAN
using-portal = MENGGUNAKAN { $portal }PORTAL:
starting-sides = SISI PERMULAAN
sound-enabled = BUNYI
    DIAKTIFKAN:
whistle-volume = KELANTANGAN
    WISEL:
manage-remotes = URUS KAWALAN JAUH
whistle-enabled = WISEL
    DIAKTIFKAN:
above-water-volume = KELANTANGAN
    ATAS AIR:
auto-sound-start-play = BUNYI AUTO
    MULA MAIN:
buzzer-sound = BUNYI
    PENGGERA:
underwater-volume = KELANTANGAN
    BAWAH AIR:
auto-sound-stop-play = BUNYI AUTO
    HENTI MAIN:
alarm-button = BUTANG
    PENGGERA:
alarm = PENGGERA
hold-to-test = TAHAN UNTUK UJI
or-press-spacebar = Atau Tekan Kekunci Ruang
or-hold-spacebar = Atau Tahan Kekunci Ruang
game-info = MAKLUMAT
remotes = KAWALAN JAUH
default = LALAI
sound = BUNYI: { $sound_text }
brightness = { $brightness ->
        *[Low] RENDAH
        [Medium] SEDERHANA
        [High] TINGGI
        [Outdoor] LUAR RUMAH
    }

waiting = MENUNGGU
add = TAMBAH
half-length = PANJANG SEPARUH
length-of-half-during-regular-play = Panjang separuh semasa permainan biasa
half-time-lenght = PANJANG REHAT SEPARUH MASA
length-of-half-time-period = Panjang tempoh rehat separuh masa
nom-break = REHAT NOMINAL
game-block = BLOK PERLAWANAN
game-block-full = BLOK PERLAWANAN:
game-block-help = Masa dari permulaan satu perlawanan hingga permulaan perlawanan berikutnya
game-block-too-short = Terlalu singkat untuk memuatkan perlawanan ditambah rehat minimum
game-block-tight = Ketat — masa rehat pasukan boleh menyebabkan perlawanan melebihi slot mereka
system-will-keep-game-times-spaced = Sistem akan cuba mengekalkan masa mula perlawanan secara sekata, dengan jumlah masa dari satu permulaan ke permulaan berikutnya ialah 2 × [Panjang Separuh] + [Panjang Rehat Separuh Masa] + [Masa Nominal Antara Perlawanan] (contoh: jika perlawanan mempunyai [Panjang Separuh] = 15m, [Panjang Rehat Separuh Masa] = 3m, dan [Masa Nominal Antara Perlawanan] = 12m, masa dari mula satu perlawanan ke perlawanan berikutnya ialah 45m. Sebarang masa rehat yang diambil, atau penghentian jam yang lain, akan mengurangkan masa 12m itu sehingga nilai masa minimum antara perlawanan dicapai).
min-break = REHAT MINIMUM
min-time-btwn-games = Jika perlawanan berjalan lebih lama daripada yang dijadualkan, ini adalah masa minimum antara perlawanan yang akan diperuntukkan oleh sistem. Jika perlawanan tertangguh, sistem akan cuba mengejar secara automatik selepas perlawanan seterusnya, sentiasa mematuhi masa minimum antara perlawanan ini.
pre-ot-break-abreviated = REHAT PRA-MASA TAMBAHAN
pre-sd-brk = Jika masa tambahan dibenarkan dan diperlukan, ini adalah panjang rehat antara Separuh Kedua dan Separuh Pertama Masa Tambahan
ot-half-len = PANJANG SEPARUH MASA TAMBAHAN
time-during-ot = Panjang separuh semasa masa tambahan
ot-half-tm-len = PANJANG REHAT SEPARUH MASA TAMBAHAN
len-of-overtime-halftime = Panjang rehat separuh masa tambahan
pre-sd-break = REHAT PRA-SUDDEN DEATH
pre-sd-len = Panjang rehat antara tempoh permainan sebelumnya dan Sudden Death
language = BAHASA
this-language = BAHASA MELAYU
portal-login-code = KOD
portal-login-instructions = Sila pergi ke { $portal } Portal >> Pengurusan Acara >> Pengurusan Pengadil, klik butang + untuk menambah Refbox baharu, dan masukkan ID Refbox ini:
    { $id }

    { $portal } Portal kemudiannya akan memberikan kod pengesahan untuk anda masukkan di sebelah kiri menggunakan pad nombor.
    Tekan selesai setelah anda memasukkan kod

help = BANTUAN:

# Pengesahan
game-configuration-can-not-be-changed = Konfigurasi perlawanan tidak boleh diubah semasa perlawanan sedang berlangsung.

    Apakah yang anda ingin lakukan?
apply-this-game-number-change = Bagaimana anda ingin menerapkan perubahan nombor perlawanan ini?
portal-enabled = Apabila { $portal }PORTAL diaktifkan, semua medan mesti diisi.
mode-switch-portal-tenant = Menukar mod daripada { $from_mode } kepada { $to_mode } akan menyahaktifkan pautan ke { $from_portal }PORTAL dan anda perlu menyambung semula ke { $to_portal }PORTAL.
uwhportal-token-invalid-code = Kod yang dimasukkan tidak sah.
    Sila cuba lagi.
uwhportal-token-no-pending-link = Portal tidak menjangka sambungan.
    Sila cuba lagi.
go-back-to-editor = KEMBALI KE EDITOR
discard-changes = BUANG PERUBAHAN
end-current-game-and-apply-changes = TAMATKAN PERLAWANAN SEMASA DAN GUNA PERUBAHAN
end-current-game-and-apply-change = TAMATKAN PERLAWANAN SEMASA DAN GUNA PERUBAHAN
keep-current-game-and-apply-change = KEKALKAN PERLAWANAN SEMASA DAN GUNA PERUBAHAN
ok = OK
confirm-score = Adakah markah ini betul?
    Sahkan dengan ketua pengadil.

    Hitam: { $score_black }        Putih: { $score_white }

    { confirmation-count-down }
yes = YA
no = TIDAK

# Kesalahan
equal = SAMA

# Maklumat Perlawanan
refresh = MUAT SEMULA
refreshing = MEMUAT SEMULA...
settings = TETAPAN
none = Tiada
game-number-error = Ralat ({ $game_number })
next-game-number-error = Ralat ({ $next_game_number })
last-game-next-game = Perlawanan Lepas: { $prev_game },
    Perlawanan Seterusnya: { $next_game }
black-team-white-team = Pasukan Hitam: { $black_team }
    Pasukan Putih: { $white_team }
game-length-ot-allowed = Panjang Separuh: { $half_length }
         Panjang Rehat Separuh Masa: { $half_time_length }
         Masa Tambahan Dibenarkan: { $overtime }
overtime-details = Panjang Rehat Pra-Masa Tambahan: { $pre_overtime }
             Panjang Separuh Masa Tambahan: { $overtime_len }
             Panjang Rehat Separuh Masa Tambahan: { $overtime_half_time_len }
sd-allowed = Sudden Death Dibenarkan: { $sd }
pre-sd = Panjang Rehat Pra-Sudden Death: { $pre_sd_len }
team-to-len = Tempoh Masa Rehat Pasukan: { $to_len }
time-btwn-games = Masa Nominal Antara Perlawanan: { $time_btwn }
game-block-info = Blok Permainan: { $game_block }
min-brk-btwn-games = Masa Minimum Antara Perlawanan: { $min_brk_time }


# Pemilih Senarai
select-event = PILIH ACARA
select-court = PILIH GELANGGANG
select-game = PILIH PERLAWANAN

# Paparan Utama
add-warning = TAMBAH AMARAN
add-foul = TAMBAH KESALAHAN
start-now = MULA SEKARANG
end-timeout = TAMAT MASA REHAT
warnings = AMARAN
penalties = PENALTI
dark-score-line-1 = MARKAH
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = MARKAH
light-score-line-2 = { light-team-name-caps }

# Penalti
black-penalties = PENALTI HITAM
white-penalties = PENALTI PUTIH

# Edit Markah
final-score = Sila masukkan markah akhir
confirmation-count-down = Nota: Markah yang tidak diubah akan disahkan secara automatik dalam { $countdown }

# Elemen Dikongsi
## Reben masa rehat
end-timeout-line-1 = TAMAT
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
switch-to = TUKAR KE
ref = PENGADIL
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
revive-hold-line-1 = TAHAN UNTUK
revive-hold-line-2 = PULIH
revive-deciding-line-2 = DIPULIHKAN
penalty-shot-line-1 = TEMBAKAN
penalty-shot-line-2 = PENALTI
pen-shot = TMBN PENALTI
## Rentetan penalti
served = Dilaksanakan
penalty = #{$player_number} - {$time ->
        [pending] Menunggu
        [served] Dilaksanakan
        [total-dismissal] Disingkirkan
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
## Rentetan konfigurasi
error = Ralat ({ $number })
two-games = Perlawanan Lepas: { $prev_game },  Perlawanan Seterusnya: { $next_game }
one-game = Perlawanan: { $game }
teams = Pasukan { -dark-team-name }: { $dark_team }
    Pasukan { -light-team-name }: { $light_team }
game-config = Panjang Separuh: { $half_len },  Panjang Rehat Separuh Masa: { $half_time_len }
    Sudden Death Dibenarkan: { $sd_allowed },  Masa Tambahan Dibenarkan: { $ot_allowed }
team-timeouts = Masa Rehat Pasukan: { $value }
team-timeouts-label = MASA REHAT
    PASUKAN:
stop-clock-last-2 = Hentikan Jam 2 Minit Terakhir: { $stop_clock }
ref-list = Ketua Pengadil: { $chief_ref }
    Pencatat Masa: { $timer }
    Pengadil Air 1: { $water_ref_1 }
    Pengadil Air 2: { $water_ref_2 }
    Pengadil Air 3: { $water_ref_3 }
team-ref-list = Pengadil: { $ref_team }
    Pencatat Masa/Markah: { $ts_keeper_team }
unknown = Tidak Diketahui
## Butang masa perlawanan
next-game = PERLAWANAN SETERUSNYA
first-half = SEPARUH PERTAMA
half-time = REHAT SEPARUH MASA
second-half = SEPARUH KEDUA
pre-ot-break-full = REHAT PRA-MASA TAMBAHAN
overtime-first-half = SEPARUH PERTAMA MASA TAMBAHAN
overtime-half-time = REHAT SEPARUH MASA TAMBAHAN
overtime-second-half = SEPARUH KEDUA MASA TAMBAHAN
pre-sudden-death-break = REHAT PRA-SUDDEN DEATH
sudden-death = SUDDEN DEATH
ot-first-half = SEPARUH 1 MASA TAMBAHAN
ot-half-time = REHAT SEPARUH MASA TAMBAHAN
ot-2nd-half = SEPARUH 2 MASA TAMBAHAN
white-timeout-short = REHAT PTH
white-timeout-full = MASA REHAT PUTIH
black-timeout-short = REHAT HTM
black-timeout-full = MASA REHAT HITAM
ref-timeout-short = REHAT PENGADIL
penalty-shot-short = TMBN PENALTI
## Buat bekas amaran
team-warning-abreviation = P
## Buat penyunting masa
zero = = 0

# Edit Masa
game-time = MASA PERLAWANAN
timeout = MASA REHAT
Note-Game-time-is-paused = Nota: Masa perlawanan dijeda semasa berada di skrin ini

# Ringkasan Amaran dan Kesalahan
fouls = KESALAHAN
edit-warnings = EDIT AMARAN
edit-fouls = EDIT KESALAHAN

# Amaran
black-warnings = AMARAN HITAM
white-warnings = AMARAN PUTIH

# Mesej
player-number = NOMBOR
    PEMAIN:
game-number = NOMBOR
    PERLAWANAN:
num-tos-per-half = BIL. MASA REHAT
    PASUKAN/SEPARUH:
num-tos-per-game = BIL. MASA REHAT
    PASUKAN/PERLAWANAN:

# Pengawal Bunyi - mod
off = MATI
low = RENDAH
medium = SEDERHANA
high = TINGGI
max = MAKSIMUM

# Konfigurasi
hockey6v6 = HOKI 6LWN6
hockey3v3 = HOKI 3LWN3
rugby = RAGBI
beep-test = BEEP TEST

# Beep-test screen
beep-test-pre = PRA
beep-test-top-time-label = MASA
beep-test-top-level-label = TAHAP
beep-test-top-lap-label = PUSINGAN
beep-test-start = MULA
beep-test-pause = JEDA
beep-test-resume = SAMBUNG
beep-test-reset = SET SEMULA
beep-test-column-level = TAHAP
beep-test-column-count = BILANGAN
beep-test-column-duration = TEMPOH
beep-test-edit-selected = Tahap { $level }
beep-test-edit-time = MASA
beep-test-edit-count = BILANGAN
beep-test-edit-new = TAMBAH TAHAP
beep-test-edit-remove = BUANG TAHAP

# Pelanggaran
stick-foul = Kesalahan Kayu
illegal-advance = Kemajuan Haram
sub-foul = Kesalahan Pertukaran Pemain
illegal-stoppage = Hentian Haram
out-of-bounds = Di Luar Sempadan
grabbing-the-wall = Memegang Dinding
obstruction = Halangan
delay-of-game = Melengahkan Permainan
unsportsmanlike = Kelakuan Tidak Bersukan
free-arm = Lengan Bebas
false-start = Permulaan Palsu


# Portal Health Indicator
portal-summary-title = STATUS PORTAL { $portal }
portal-row-token-expired = Log masuk portal tamat tempoh — ketik untuk log masuk semula
portal-row-stuck = Perlawanan { $game } Ralat hantar markah, ketik untuk perbaiki
portal-row-pending = Perlawanan { $game } Markah belum dihantar, ketik untuk cuba semula
portal-row-attempt-suffix = (percubaan { $attempts })
portal-row-recent = Perlawanan { $game } · Dihantar { $mins } min lalu
portal-action-force-submit = Cuba semula keputusan perlawanan ini
portal-action-discard = Buang keputusan perlawanan ini
portal-action-discard-confirm = KETIK SEKALI LAGI UNTUK SAHKAN BUANG
portal-page-title-attention = Ralat penghantaran perlawanan { $game }
portal-page-attention-info = Keputusan perlawanan tidak diterima di Portal { $portal }
portal-page-attention-score = Keputusan perlawanan disimpan: Putih { $white } - Hitam { $black }
portal-page-attention-remediation = Anda boleh Cuba Semula jika sambungan disahkan, atau buang untuk membersihkan ralat
portal-advisory-at-game-end = Masalah portal dikesan. Markah masih akan dibariskan — cari admin untuk selesaikan.

# 2 Halves / 1 Period selector (Half Length editor)
two-halves = 2 SEPARUH
one-period = 1 PERIOD
game-len = PANJANG PERLAWANAN
length-of-game-during-regular-play = Panjang perlawanan semasa permainan biasa

# Single-period (1 Period) info variants — Game Length, no half-time line
game-config-single-half = Panjang Perlawanan: { $half_len }
    Sudden Death Dibenarkan: { $sd_allowed },  Masa Tambahan Dibenarkan: { $ot_allowed }
game-length-ot-allowed-single-half = Panjang Perlawanan: { $half_length }
         Masa Tambahan Dibenarkan: { $overtime }

# Self-update / Updates page
check-version = Semak Versi
updates-current-version = Versi semasa
updates-check-for-updates = Semak Kemas Kini
updates-install = Pasang
updates-do-revert = Kembalikan
updates-install-note = Mengklik Pasang akan memuat turun dan memasang kemas kini lalu memulakan semula refbox
updates-revert-note = Mengklik Kembalikan akan memulihkan versi sebelumnya dan memulakan semula refbox
updates-unknown = Tidak diketahui
updates-checking = Menyemak…
updates-up-to-date = Sudah terkini.
updates-available = Kemas kini tersedia: {$version}
updates-downloading = Memuat turun…
updates-verifying = Menyemak muat turun…
updates-installing = Memasang…
updates-restarting = Memulakan semula…
updates-confirm-revert = Kembali ke versi sebelumnya ({$version})?
updates-rolled-back = Dikembalikan kepada versi sebelumnya kerana kemas kini tidak bermula dengan betul, sila cuba lagi.
updates-revert = Kembali ke Versi Sebelumnya ({$version})
updates-error-no-internet = Tidak dapat menghubungi pelayan kemas kini, sila semak sambungan internet anda
updates-error-bad-download = Kemas kini yang dimuat turun tidak sah dan tidak dipasang.
updates-error-rate-limited = Pelayan kemas kini sibuk, sila cuba lagi sebentar lagi.
updates-error-no-space = Ruang kosong tidak mencukupi untuk memasang kemas kini.
updates-error-not-writable = Kemas kini tidak dapat disimpan (kebenaran ditolak).

# Game-info table labels
gi-prior-game = Perlawanan Lepas
gi-team-light = { -light-team-name }
gi-team-dark = { -dark-team-name }
gi-current-game = Perlawanan Semasa
gi-next-game = Perlawanan Seterusnya
gi-game-block = Blok Permainan
gi-half-length = Panjang Separuh
gi-half-time-length = Panjang Rehat Separuh Masa
gi-game-length = Panjang Perlawanan
gi-timeouts = Masa Rehat
gi-timeout-duration = Tempoh Masa Rehat
gi-overtime = Masa Tambahan
gi-sudden-death = Sudden Death
gi-pre-overtime-break = Rehat Pra-Masa Tambahan
gi-pre-sudden-death-break = Rehat Pra-Sudden Death
gi-overtime-half-length = Panjang Separuh Masa Tambahan
gi-overtime-half-time-length = Panjang Rehat Separuh Masa Tambahan
gi-minimum-game-break = Masa Minimum Antara Perlawanan
gi-stop-clock-last-2 = Hentikan Jam 2 Minit Terakhir
gi-ref-chief = Ketua Pengadil
gi-ref-timekeeper = Pencatat Masa
gi-ref-timekeeper-helper = Pembantu Pencatat Masa
gi-ref-water-1 = Pengadil Air 1
gi-ref-water-2 = Pengadil Air 2
gi-ref-water-3 = Pengadil Air 3
gi-unknown = ???
