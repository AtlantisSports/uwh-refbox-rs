# Definitions for the translation file to use
-dark-team-name = Hitam
dark-team-name-caps = HITAM

-light-team-name = Putih
light-team-name-caps = PUTIH

# Multipage
done = SELESAI
restart-to-apply = MULAKAN SEMULA
cancel = BATAL
delete = PADAM
back = KEMBALI
new = BARU

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
timeout-length = MASA REHAT
    PASUKAN

# Warning Add
team-warning = AMARAN
    PASUKAN
team-warning-line-1 = AMARAN
team-warning-line-2 = PASUKAN

# Configuration
none-selected = Tiada Dipilih
loading = Memuatkan...
game-select = Perlawanan:
game-options = OPSYEN PERLAWANAN
app-options = OPSYEN APLIKASI
display-options = OPSYEN PAPARAN
sound-options = OPSYEN BUNYI
app-mode = MOD
    APLIKASI
hide-time-for-last-15-seconds = SEMBUNYIKAN MASA
    15 SAAT TERAKHIR
player-display-brightness = KECERAHAN PAPARAN
    PEMAIN
confirm-score-at-game-end = SAHKAN SKOR
    AKHIR PERLAWANAN
track-cap-number-of-scorer = REKOD NOMBOR
    PENCETAK GOL
event = ACARA:
track-fouls-and-warnings = REKOD KESALAHAN
    DAN AMARAN
court = GELANGGANG:
single-half = SEPARUH
    TUNGGAL:
half-length-full = PANJANG SEPARUH:
game-length = PANJANG PERLAWANAN:
overtime-allowed = MASA TAMBAHAN
    DIBENARKAN:
sudden-death-allowed = SUDDEN DEATH
    DIBENARKAN:
half-time-length = PANJANG MASA
    REHAT:
pre-ot-break-length = PANJANG REHAT
    PRA-MASA TAMBAHAN:
pre-sd-break-length = PANJANG REHAT
    PRA-SD:
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
ot-half-time-length = PANJANG MASA REHAT
    MASA TAMBAHAN
using-uwh-portal = MENGGUNAKAN UWHPORTAL:
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
    BERHENTI MAIN:
alarm-button = BUTANG
    PENGGERA:
alarm = PENGGERA
hold-to-test = TAHAN UNTUK UJI
or-press-spacebar = Atau Tekan Kekunci Ruang
or-hold-spacebar = Atau Tahan Kekunci Ruang
game-info = MAKLUMAT PERLAWANAN
remotes = KAWALAN JAUH
default = LALAI
sound = BUNYI: { $sound_text }
brightness = { $brightness ->
        *[Low] RENDAH
        [Medium] SEDERHANA
        [High] TINGGI
        [Outdoor] LUAR
    }

waiting = MENUNGGU
add = TAMBAH
half-length = PANJANG SEPARUH
length-of-half-during-regular-play = Panjang separuh semasa permainan biasa
half-time-lenght = PANJANG MASA REHAT
length-of-half-time-period = Panjang tempoh masa rehat
nom-break = REHAT NOMINAL
system-will-keep-game-times-spaced = Sistem akan cuba mengekalkan masa mula perlawanan secara sekata, dengan jumlah masa dari satu permulaan ke permulaan berikutnya ialah 2 × [Panjang Separuh] + [Panjang Masa Rehat] + [Masa Nominal Antara Perlawanan] (contoh: jika perlawanan mempunyai [Panjang Separuh] = 15m, [Panjang Masa Rehat] = 3m, dan [Masa Nominal Antara Perlawanan] = 12m, masa dari mula satu perlawanan ke perlawanan berikutnya ialah 45m. Sebarang masa rehat yang diambil, atau penghentian jam lain, akan mengurangkan 12m tersebut sehingga nilai masa minimum antara perlawanan dicapai).
min-break = REHAT MINIMUM
min-time-btwn-games = Jika perlawanan berjalan lebih lama daripada yang dijadualkan, ini adalah masa minimum antara perlawanan yang akan diperuntukkan oleh sistem. Jika perlawanan tertangguh, sistem akan cuba mengejar secara automatik selepas perlawanan seterusnya, sentiasa mematuhi masa minimum antara perlawanan ini.
pre-ot-break-abreviated = REHAT PRA-MASA TAMBAHAN
pre-sd-brk = Jika masa tambahan dibenarkan dan diperlukan, ini adalah panjang rehat antara Separuh Kedua dan Separuh Pertama Masa Tambahan
ot-half-len = PANJANG SEPARUH MASA TAMBAHAN
time-during-ot = Panjang separuh semasa masa tambahan
ot-half-tm-len = PANJANG MASA REHAT MASA TAMBAHAN
len-of-overtime-halftime = Panjang Masa Rehat Masa Tambahan
pre-sd-break = REHAT PRA-SUDDEN DEATH
pre-sd-len = Panjang rehat antara tempoh permainan sebelumnya dan Sudden Death
language = BAHASA
this-language = BAHASA MELAYU
portal-login-code = KOD
portal-login-instructions = Sila pergi ke UWH Portal >> Pengurusan Acara >> Pengurusan Pengadil, klik butang + untuk menambah Refbox baharu, dan masukkan ID Refbox ini:
    { $id }

    UWH Portal kemudian akan memberikan kod pengesahan untuk anda masukkan di sebelah kiri menggunakan pad nombor.
    Tekan selesai setelah anda memasukkan kod

help = BANTUAN:

# Confirmation
game-configuration-can-not-be-changed = Konfigurasi perlawanan tidak boleh diubah semasa perlawanan sedang berlangsung.

    Apakah yang anda ingin lakukan?
apply-this-game-number-change = Bagaimana anda ingin menerapkan perubahan nombor perlawanan ini?
UWHPortal-enabled = Apabila UWHPortal diaktifkan, semua medan mesti diisi.
uwhportal-token-invalid-code = Kod yang dimasukkan tidak sah.
    Sila cuba lagi.
uwhportal-token-no-pending-link = Portal tidak menjangka sambungan.
    Sila cuba lagi.
go-back-to-editor = KEMBALI KE EDITOR
discard-changes = BUANG PERUBAHAN
end-current-game-and-apply-changes = TAMATKAN PERLAWANAN SEMASA DAN TERAPKAN PERUBAHAN
end-current-game-and-apply-change = TAMATKAN PERLAWANAN SEMASA DAN TERAPKAN PERUBAHAN
keep-current-game-and-apply-change = KEKALKAN PERLAWANAN SEMASA DAN TERAPKAN PERUBAHAN
ok = OK
confirm-score = Adakah skor ini betul?
    Sahkan dengan pengadil ketua.

    Hitam: { $score_black }        Putih: { $score_white }

    { confirmation-count-down }
yes = YA
no = TIDAK

# Fouls
equal = SAMA

# Game Info
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
         Panjang Masa Rehat: { $half_time_length }
         Masa Tambahan Dibenarkan: { $overtime }
overtime-details = Panjang Rehat Pra-Masa Tambahan: { $pre_overtime }
             Panjang Separuh Masa Tambahan: { $overtime_len }
             Panjang Masa Rehat Masa Tambahan: { $overtime_half_time_len }
sd-allowed = Sudden Death Dibenarkan: { $sd }
pre-sd = Panjang Rehat Pra-Sudden Death: { $pre_sd_len }
team-to-len = Tempoh Masa Rehat Pasukan: { $to_len }
time-btwn-games = Masa Nominal Antara Perlawanan: { $time_btwn }
min-brk-btwn-games = Masa Minimum Antara Perlawanan: { $min_brk_time }


# List Selecters
select-event = PILIH ACARA
select-court = PILIH GELANGGANG
select-game = PILIH PERLAWANAN

# Main View
add-warning = TAMBAH AMARAN
add-foul = TAMBAH KESALAHAN
start-now = MULA SEKARANG
end-timeout = TAMAT MASA REHAT
warnings = AMARAN
penalties = PENALTI
dark-score-line-1 = SKOR
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = SKOR
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = PENALTI HITAM
white-penalties = PENALTI PUTIH

# Score edit
final-score = Sila masukkan skor akhir
confirmation-count-down = Nota: Skor yang tidak diubah akan disahkan secara automatik dalam { $countdown }

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = TAMAT
end-timeout-line-2 = { timeout }
switch-to = TUKAR KE
ref = PENGADIL
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
penalty-shot-line-1 = PENALTI
penalty-shot-line-2 = SEPAKAN
pen-shot = SEPAKAN PENALTI
## Penalty string
served = Dilaksanakan
penalty = #{$player_number} - {$time ->
        [pending] Menunggu
        [served] Dilaksanakan
        [total-dismissal] Diusir
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
error = Ralat ({ $number })
two-games = Perlawanan Lepas: { $prev_game },  Perlawanan Seterusnya: { $next_game }
one-game = Perlawanan: { $game }
teams = Pasukan { -dark-team-name }: { $dark_team }
    Pasukan { -light-team-name }: { $light_team }
game-config = Panjang Separuh: { $half_len },  Panjang Masa Rehat: { $half_time_len }
    Sudden Death Dibenarkan: { $sd_allowed },  Masa Tambahan Dibenarkan: { $ot_allowed }
team-timeouts-per-half = Masa Rehat Pasukan Dibenarkan Per Separuh: { $team_timeouts }
team-timeouts-per-game = Masa Rehat Pasukan Dibenarkan Per Perlawanan: { $team_timeouts }
stop-clock-last-2 = Hentikan Jam 2 Minit Terakhir: { $stop_clock }
ref-list = Pengadil Ketua: { $chief_ref }
    Pemasa: { $timer }
    Pengadil Air 1: { $water_ref_1 }
    Pengadil Air 2: { $water_ref_2 }
    Pengadil Air 3: { $water_ref_3 }
team-ref-list = Pengadil: { $ref_team }
    Penjaga Masa/Markah: { $ts_keeper_team }
unknown = Tidak Diketahui
## Game time button
next-game = PERLAWANAN SETERUSNYA
first-half = SEPARUH PERTAMA
half-time = MASA REHAT
second-half = SEPARUH KEDUA
pre-ot-break-full = REHAT PRA-MASA TAMBAHAN
overtime-first-half = SEPARUH PERTAMA MASA TAMBAHAN
overtime-half-time = MASA REHAT MASA TAMBAHAN
overtime-second-half = SEPARUH KEDUA MASA TAMBAHAN
pre-sudden-death-break = REHAT PRA-SUDDEN DEATH
sudden-death = SUDDEN DEATH
ot-first-half = SEPARUH 1 MASA TAMBAHAN
ot-half-time = MASA REHAT MASA TAMBAHAN
ot-2nd-half = SEPARUH 2 MASA TAMBAHAN
white-timeout-short = REHAT PTH
white-timeout-full = MASA REHAT PUTIH
black-timeout-short = REHAT HTM
black-timeout-full = MASA REHAT HITAM
ref-timeout-short = REHAT PENGADIL
penalty-shot-short = SEPAKAN PENALTI
## Make warning container
team-warning-abreviation = P
## Make time editor
zero = SIFAR

# Time edit
game-time = MASA PERLAWANAN
timeout = MASA REHAT
Note-Game-time-is-paused = Nota: Masa perlawanan dijeda semasa berada di skrin ini

# Warning Fouls Summary
fouls = KESALAHAN
edit-warnings = EDIT AMARAN
edit-fouls = EDIT KESALAHAN

# Warnings
black-warnings = AMARAN HITAM
white-warnings = AMARAN PUTIH

# Message
player-number = NOMBOR
    PEMAIN:
game-number = NOMBOR
    PERLAWANAN:
num-tos-per-half = BIL. MASA REHAT
    PASUKAN/SEPARUH:
num-tos-per-game = BIL. MASA REHAT
    PASUKAN/PERLAWANAN:

# Sound Controller - mod
off = MATI
low = RENDAH
medium = SEDERHANA
high = TINGGI
max = MAKSIMUM

# Config
hockey6v6 = HOKI 6LWN6
hockey3v3 = HOKI 3LWN3
rugby = RAGBI

# Infractions
stick-foul = Kesalahan Kayu
illegal-advance = Kemajuan Haram
sub-foul = Kesalahan Penggantian
illegal-stoppage = Pemberhentian Haram
out-of-bounds = Luar Sempadan
grabbing-the-wall = Pegang Dinding
obstruction = Halangan
delay-of-game = Melengah-lengahkan Perlawanan
unsportsmanlike = Tidak Bersemangat Sukan
free-arm = Lengan Bebas
false-start = Permulaan Palsu
