# Definitions for the translation file to use
-dark-team-name = Noir
dark-team-name-caps = NOIR
-light-team-name = Blanc
light-team-name-caps = BLANC

# Multipage 
done = TERMINÉ
cancel = ANNULER
delete = SUPPRIMER
back = RETOUR
new = NOUVEAU

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
timeout-length = DURÉE DU TEMPS MORT

# Warning Add
team-warning = AVERT. D'ÉQUIPE
team-warning-line-1 = AVERT.
team-warning-line-2 = D'ÉQUIPE

# Configuration
none-selected = Aucun Sélectionné
loading = Chargement...
game-select = MATCH:
game-options = OPTIONS DU MATCH
app-options = OPTIONS DE L'APPLICATION
display-options = OPTIONS D'AFFICHAGE
sound-options = OPTIONS SONORES
app-mode = MODE
hide-time-for-last-15-seconds = CACHER LE TEMPS POUR
    LES 15 DERNIÈRES SECONDES
### Shorten
player-display-brightness = LUMINOSITÉ DE
    L'AFFICHAGE DES JOUEURS
confirm-score-at-game-end = CONFIRMER LE SCORE
    À LA FIN DU MATCH
track-cap-number-of-scorer = SUIVRE LE NUMÉRO DU
    BUTEUR
track-fouls-and-warnings = SUIVRE LES FAUTES 
    ET LES AVERTISSEMENTS
event = EVÉNEMENT:
court = TERRAIN:
single-half = UNE SEULE PÉRIODE:
half-length-full = DURÉE DE LA
    PÉRIODE:
game-length = DURÉE DU
    MATCH:
overtime-allowed = PROLONGATIONS
    AUTORISÉES:
sudden-death-allowed = MORT SUBITE
    AUTORISÉE:
half-time-length = DURÉE DE LA
    MI-TEMPS:
pre-ot-break-length = PAUSE AVANT
    PROLONGATIONS:
pre-sd-break-length = PAUSE AVANT
    MORT SUBITE:
nominal-break-between-games = PAUSE ENTRE
    LES MATCHS:
ot-half-length = PÉRIODE DE
    PROLONGATION:
timeouts-counted-per = TEMPS MORTS
    COMPTÉS PAR:
game = MATCH
half = PÉRIODE
minimum-brk-btwn-games = PAUSE MIN. ENTRE
    LES MATCHS:
ot-half-time-length = DURÉE DE LA PAUSE
    A LA PROLONGATION
using-uwh-portal = UTILISATION DU UWHPORTAL:
starting-sides = CÔTÉS DE DÉPART
sound-enabled = SON
    ACTIVÉ:
whistle-volume = VOLUME DU
    SIFFLET:
manage-remotes = GÉRER LES TÉLÉCOMMANDES
whistle-enabled = SIFFLET
    ACTIVÉ:
above-water-volume = VOLUME
    SURFACE:
auto-sound-start-play = SON AUTOMATIQUE
    DÉBUT MATCH:
buzzer-sound = SON DU
    BUZZER:
underwater-volume = VOLUME
    SOUS L'EAU:
auto-sound-stop-play = SON AUTOMATIQUE
    ARRÊT MATCH:
remotes = TÉLÉCOMMANDES
default = DÉFAUT
sound = SON: { $sound_text }
brightness = { $brightness ->
        *[Low] FAIBLE
        [Medium] MOYEN
        [High] HAUT
        [Outdoor] EXTÉRIEUR
    }

waiting = EN ATTENTE
add = AJOUTER
half-length = DURÉE PÉRIODE
length-of-half-during-regular-play = La durée d'une période pendant le match régulier
half-time-lenght = DURÉE MI-TEMPS
length-of-half-time-period = La durée de la mi-temps
nom-break = PAUSE NOMINALE
system-will-keep-game-times-spaced = Le système essaiera de maintenir les heures de début des matchs régulièrement espacées, avec un temps total de 2 * [Durée de la mi-temps] + [Durée de la mi-temps] + [Pause nominale entre les matchs] (exemple: si les matchs ont [Durée de la mi-temps] = 15m, [Durée de la mi-temps] = 3m, et [Pause nominale entre les matchs] = 12m, le temps entre le début d'un match et le suivant sera de 45m. Tout temps mort pris, ou autre arrêt de l'horloge, réduira le temps de 12m jusqu'à ce que la valeur de la pause minimale entre les matchs soit atteinte).
min-break = PAUSE MINIMUM
min-time-btwn-games = Si un match dure plus longtemps que prévu, ceci est le temps minimum entre les matchs que le système allouera. Si les matchs prennent du retard, le système essaiera automatiquement de rattraper après les matchs suivants, en respectant toujours ce temps minimum entre les matchs.
pre-ot-break-abreviated = P. PRÉ PROL.
pre-sd-brk = Si les prolongations sont autorisées et nécessaires, ceci est la durée de la pause entre la deuxième mi-temps et la première mi-temps des prolongations
ot-half-len = PÉRIODE PROLONGATION
time-during-ot = La durée d'une période pendant les prolongations
ot-half-tm-len = DURÉE DE LA PAUSE A LA PROLONGATION
len-of-overtime-halftime = La durée de la mi-temps des prolongations
pre-sd-break = PAUSE PRÉ M/S
pre-sd-len = La durée de la pause entre la période de match précédente et la mort subite
language = LANGUE
this-language = FRANÇAIS
### Check
portal-login-code = Code de connexion
### Check
portal-login-instructions = Veuillez aller sur le Portail UWH >> Gestion des Événements >> Gestion des Arbitres, cliquer sur le bouton + pour ajouter une nouvelle Refbox, et entrer cet ID Refbox :
    { $id }
    
    Le Portail UWH fournira ensuite un code de confirmation que vous devrez entrer à gauche en utilisant le pavé numérique.
    Appuyez sur Terminé une fois que vous avez entré le code.

help = AIDE: 

# Confirmation
game-configuration-can-not-be-changed = La configuration du match ne peut pas être modifiée pendant qu'un match est en cours.
    
    Que souhaitez-vous faire ?
apply-this-game-number-change = Comment souhaitez-vous appliquer ce changement de numéro ?
UWHPortal-enabled = Lorsque UWHPortal est activé, tous les champs doivent être remplis.
### Check
uwhportal-token-invalid-code = Code invalide.
    Veuillez réessayer.
### Check
uwhportal-token-no-pending-link = Aucun lien en attente trouvé.
    Veuillez réessayer.
go-back-to-editor = RETOURNER À L'ÉDITEUR
discard-changes = ANNULER LES MODIFICATIONS
end-current-game-and-apply-changes = TERMINER LE MATCH EN COURS ET APPLIQUER LES MODIFICATIONS
end-current-game-and-apply-change = TERMINER LE MATCH EN COURS ET APPLIQUER LE CHANGEMENT
keep-current-game-and-apply-change = GARDER LE MATCH EN COURS ET APPLIQUER LE CHANGEMENT
ok = OK
confirm-score = Ce score est-il correct ?
    Confirmer avec l'arbitre en chef.
    
    Noir: { $score_black }        Blanc: { $score_white }

    { confirmation-count-down }
yes = OUI
no = NON

# Fouls
equal = ÉGAL

# Game Info
refresh = RAFRAÎCHIR
refreshing = RAFRAÎCHISSANT...
settings = PARAMÈTRES 
none = Aucun
game-number-error = Erreur ({ $game_number })
next-game-number-error = Erreur ({ $next_game_number })
last-game-next-game = Dernier Match: { $prev_game },
    Prochain Match: { $next_game }
black-team-white-team = Équipe Noire: { $black_team }
    Équipe Blanche: { $white_team }
game-length-ot-allowed = Durée de la période: { $half_length }
         Durée de la mi-temps: { $half_time_length }
         Prolongations Autorisées: { $overtime }
overtime-details = Durée de la pause avant les prolongations: { $pre_overtime }
             Durée de la période des prolongations: { $overtime_len }
             Durée de la mi-temps des prolongations: { $overtime_half_time_len }
sd-allowed = Mort Subite Autorisée: { $sd }
pre-sd = Durée de la pause avant la mort subite: { $pre_sd_len }
team-to-len = Durée du Temps Mort d'Équipe: { $to_len }
time-btwn-games = Temps Entre les Matchs: { $time_btwn }
min-brk-btwn-games = Temps Minimum Entre les Matchs: { $min_brk_time }

# List Selecters
select-event = SÉLECTIONNER LE EVÉNEMENT
select-court = SÉLECTIONNER LE TERRAIN
select-game = SÉLECTIONNER LE MATCH

# Main View
add-warning = AJOUTER UN AVERT.
add-foul = AJOUTER UNE FAUTE
start-now = COMMENCER MAINTENANT
end-timeout = FIN DU TEMPS MORT
warnings = AVERTISSEMENTS (AVERT.)
penalties = PÉNALITÉS
dark-score-line-1 = SCORE
dark-score-line-2 = { dark-team-name-caps }
light-score-line-1 = SCORE
light-score-line-2 = { light-team-name-caps }

# Penalties
black-penalties = PÉNALITÉS NOIR
white-penalties = PÉNALITÉS BLANC

# Score edit
final-score = Veuillez entrer le score final
### Check
confirmation-count-down = Note : Le score inchangé sera automatiquement confirmé dans { $countdown }

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = FIN
end-timeout-line-2 = { timeout }
switch-to = PASSER À
ref = ARBITRE
ref-timeout-line-1 = { timeout }
ref-timeout-line-2 = D'ARBITRE
dark-timeout-line-1 = { timeout }
dark-timeout-line-2 = { dark-team-name-caps }
light-timeout-line-1 = { timeout }
light-timeout-line-2 = { light-team-name-caps }
penalty-shot-line-1 = TIR DE
penalty-shot-line-2 = PÉNALITÉ
pen-shot = TIR DE PÉNALITÉ
## Penalty string
served = Servi
penalty = #{$player_number} - {$time ->
        [pending] En Attente
        [served] Servi
        [total-dismissal] Expulsé
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
infraction = Faute: {$infraction}
## Config String
error = Erreur ({ $number })
two-games = Dernier Match: { $prev_game },  Prochain Match: { $next_game }
one-game = Match: { $game }
teams = Équipe { -dark-team-name }: { $dark_team }
    Équipe { -light-team-name }: { $light_team }
game-config = Durée de la Pér.: { $half_len },  Mi-temps: { $half_time_len }
    Mort Subite: { $sd_allowed },  Prolongations: { $ot_allowed }
team-timeouts-per-half = Temps Morts d'Équipe Autorisés par Période: { $team_timeouts }
team-timeouts-per-game = Temps Morts d'Équipe Autorisés par Match: { $team_timeouts }
stop-clock-last-2 = Arrêter le temps dans les 2 dernières minutes: { $stop_clock }
ref-list = Arbitre en Chef: { $chief_ref }
    Chronométreur: { $timer }
    Arbitre Aquatique 1: { $water_ref_1 }
    Arbitre Aquatique 2: { $water_ref_2 }
    Arbitre Aquatique 3: { $water_ref_3 }
unknown = Inconnu
## Game time button
next-game = MATCH SUIVANT
first-half = 1ere PÉRIODE
half-time = MI-TEMPS
second-half = 2eme PÉRIODE
pre-ot-break-full = PAUSE PRÉ PROLONGATIONS
overtime-first-half = 1ere PROLONGATION
overtime-half-time = MI-TEMPS PROLONGATION
overtime-second-half = 2eme PROLONGATION
pre-sudden-death-break = PAUSE PRÉ MORT SUBITE
sudden-death = MORT SUBITE
ot-first-half = 1ere PROL.
ot-half-time = M-T PROL.
ot-2nd-half = 2eme PROL.
white-timeout-short = T/M BLANC
white-timeout-full = T/M BLANC
black-timeout-short = T/M NOIR
black-timeout-full = T/M NOIR
ref-timeout-short = T/M D'ARB.
penalty-shot-short = TIR DE PÉN.
## Make warning container
team-warning-abreviation = É
## Make time editor
zero = ZÉRO

# Time edit
game-time = TEMPS DE MATCH
timeout = TEMPS MORT (T/M)
Note-Game-time-is-paused = Note: Le temps de match est en pause sur cet écran

# Warning Fouls Summary
fouls = FAUTES
edit-warnings = ÉDITER LES AVERTISSEMENTS
edit-fouls = ÉDITER LES FAUTES

# Warnings
black-warnings = AVERTISSEMENTS NOIR
white-warnings = AVERTISSEMENTS BLANC

# Message
player-number = NUMÉRO DU
    JOUEUR:
game-number = NUMÉRO DU
    MATCH:
num-tos-per-half = NOMBRE DE TEMPS
    MORTS PAR MI-TEMPS:
num-tos-per-game = NOMBRE DE TEMPS
    MORTS PAR MATCH:

# Sound Controller - mod
off = ÉTEINT
low = BAS
medium = MOYEN
high = HAUT
max = MAX

# Config
hockey6v6 = Hockey6V6
hockey3v3 = Hockey3V3
rugby = Rugby

# Infractions
stick-foul = Faute de crosse
illegal-advance = Faire avancer le palet
sub-foul = Changement incorrect
illegal-stoppage = Arrêt irrégulier
out-of-bounds = Palet sorti
grabbing-the-wall = Agripper avec les barrières
obstruction = Obstruction
delay-of-game = Faute pour ralentir le jeu
unsportsmanlike = Conduite anti sportive
free-arm = Usage irrégulier du bras libre
false-start = Faux départs
