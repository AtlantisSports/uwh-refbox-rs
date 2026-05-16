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
    PASUKAN

# Tambah Amaran
team-warning = AMARAN
    PASUKAN
team-warning-line-1 = AMARAN
team-warning-line-2 = PASUKAN

# Konfigurasi
none-selected = Tiada Dipilih
loading = Memuatkan...
game-select = Perlawanan:
game-options = PILIHAN PERLAWANAN
app-options = PILIHAN APLIKASI
display-options = PILIHAN PAPARAN
sound-options = PILIHAN BUNYI
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
game-info = MAKLUMAT PERLAWANAN
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
switch-to = TUKAR KE
ref = PENGADIL
ref-timeout-line-1 = { ref }
ref-timeout-line-2 = { timeout }
dark-timeout-line-1 = { dark-team-name-caps }
dark-timeout-line-2 = { timeout }
light-timeout-line-1 = { light-team-name-caps }
light-timeout-line-2 = { timeout }
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
team-timeouts-per-half = Masa Rehat Pasukan Dibenarkan Per Separuh: { $team_timeouts }
team-timeouts-per-game = Masa Rehat Pasukan Dibenarkan Per Perlawanan: { $team_timeouts }
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
# NOTE: Awaiting native-speaker translation; English placeholders for now.
portal-summary-title = { $portal } PORTAL STATUS
portal-row-token-expired = Portal login expired — tap to re-login
portal-row-stuck = Game { $game } Score send error, tap to fix
portal-row-pending = Game { $game } Score not sent, tap to retry
portal-row-recent = Game { $game } · Submitted { $mins } min ago
portal-action-force-submit = Retry this game result
portal-action-discard = Discard this game result
portal-action-discard-confirm = TAP AGAIN TO CONFIRM DISCARD
portal-page-title-attention = Game { $game } submission error
portal-page-attention-info = The game result has not been accepted on { $portal } Portal
portal-page-attention-score = Stored game result: Light { $white } - Dark { $black }
portal-page-attention-remediation = You can Retry if connection is verified, or discard to clear the error
portal-advisory-at-game-end = Portal issue detected. Score will still be queued — find an admin to resolve.
