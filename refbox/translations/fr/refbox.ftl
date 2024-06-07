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
total-dismissial = TD

# Team Timeout Edit
timeout-length = DURÉE DU TEMPS MORT

# Warning Add
## Shorten
team-warning = ÉQUIPE
    AVERTISSEMENT
team = ÉQUIPE
warning = AVERTISSEMENT

# Configuration
none-selected = Aucun Sélectionné
loading = Chargement...
game = Jeu :
tournament-options = OPTIONS DU TOURNOI
app-options = OPTIONS DE L'APPLICATION
display-options = OPTIONS D'AFFICHAGE
sound-options = OPTIONS SONORES
app-mode = MODE
    APPLICATION
hide-time-for-last-15-seconds = CACHER LE TEMPS POUR
    LES 15 DERNIÈRES SECONDES
track-cap-number-of-scorer = SUIVRE LE NUMÉRO DU
    MARQUEUR
track-fouls-and-warnings = SUIVRE LES FAUTES 
    ET LES AVERTISSEMENTS
tournament = TOURNOI :
court = TERRAIN :
half-length-full = DURÉE DE LA MI-TEMPS :
overtime-allowed = PROLONGATIONS
    AUTORISÉES :
sudden-death-allowed = MORT SUBITE
    AUTORISÉE :
half-time-length = DURÉE DE LA
    MI-TEMPS :
## Shorten
pre-ot-break-length = DURÉE DE LA PAUSE
    AVANT PROLONGATIONS :
## Shorten
pre-sd-break-length = DURÉE DE LA PAUSE
    AVANT MORT SUBITE :
## Shorten
nominal-break-between-games = PAUSE NOMINALE
    ENTRE LES MATCHS :
ot-half-length = DURÉE DE LA
    MI-PROLONGATION :
## Shorten
num-team-tos-allowed-per-half = NOMBRE DE TEMPS MORTS
    PAR MI-TEMPS :
minimum-brk-btwn-games = PAUSE MINIMALE
    ENTRE LES MATCHS :
ot-half-time-length = DURÉE DE LA
    MI-PROLONGATION
using-uwh-portal = UTILISATION DU PORTAIL UWH :
starting-sides = CÔTÉS DE DÉPART
sound-enabled = SON
    ACTIVÉ :
whistle-volume = VOLUME DU
    SIFFLET :
manage-remotes = GÉRER LES TÉLÉCOMMANDES
whistle-enabled = SIFFLET
    ACTIVÉ :
## Shorten 
above-water-volume = VOLUME AU-DESSUS
    DE L'EAU :
auto-sound-start-play = SON AUTOMATIQUE
    DÉBUT JEU :
buzzer-sound = SON DU
    BUZZER :
underwater-volume = VOLUME
    SOUS L'EAU :
auto-sound-stop-play = SON AUTOMATIQUE
    ARRÊT JEU :
remotes = TÉLÉCOMMANDES
default = PAR DÉFAUT
sound = SON : { $sound_text }

waiting = EN ATTENTE
add = AJOUTER
half-length = DURÉE MI-TEMPS
length-of-half-during-regular-play = La durée d'une mi-temps pendant le jeu régulier
half-time-lenght = DURÉE MI-TEMPS
length-of-half-time-period = La durée de la période de mi-temps
nom-break = PAUSE NOMINALE
system-will-keep-game-times-spaced = Le système essaiera de maintenir les heures de début des matchs régulièrement espacées, avec un temps total de 2 * [Durée de la mi-temps] + [Durée de la mi-temps] + [Pause nominale entre les matchs] (exemple : si les matchs ont [Durée de la mi-temps] = 15m, [Durée de la mi-temps] = 3m, et [Pause nominale entre les matchs] = 12m, le temps entre le début d'un match et le suivant sera de 45m. Tout temps mort pris, ou autre arrêt de l'horloge, réduira le temps de 12m jusqu'à ce que la valeur de la pause minimale entre les matchs soit atteinte).
min-break = PAUSE MINIMALE
min-time-btwn-games = Si un match dure plus longtemps que prévu, ceci est le temps minimum entre les matchs que le système allouera. Si les matchs prennent du retard, le système essaiera automatiquement de rattraper après les matchs suivants, en respectant toujours ce temps minimum entre les matchs.
pre-ot-break-abreviated = PAUSE PRÉ PROLONGATIONS
pre-sd-brk = Si les prolongations sont autorisées et nécessaires, ceci est la durée de la pause entre la deuxième mi-temps et la première mi-temps des prolongations
ot-half-len = DURÉE MI-PROLONGATION
time-during-ot = La durée d'une mi-temps pendant les prolongations
ot-half-tm-len = DURÉE MI-PROLONGATION
len-of-overtime-halftime = La durée de la mi-temps des prolongations
pre-sd-break = PAUSE PRÉ MORT SUBITE
pre-sd-len = La durée de la pause entre la période de jeu précédente et la mort subite

help = AIDE : 

# Confirmation
game-configuration-can-not-be-changed = La configuration du jeu ne peut pas être modifiée pendant qu'un jeu est en cours.
    
    Que souhaitez-vous faire ?
apply-this-game-number-change = Comment souhaitez-vous appliquer ce changement de numéro de jeu ?
UWHScores-enabled = Lorsque UWHScores est activé, tous les champs doivent être remplis.
go-back-to-editor = RETOURNER À L'ÉDITEUR
discard-changes = ANNULER LES MODIFICATIONS
end-current-game-and-apply-changes = TERMINER LE JEU EN COURS ET APPLIQUER LES MODIFICATIONS
end-current-game-and-apply-change = TERMINER LE JEU EN COURS ET APPLIQUER LE CHANGEMENT
keep-current-game-and-apply-change = GARDER LE JEU EN COURS ET APPLIQUER LE CHANGEMENT
ok = OK
confirm-score = Ce score est-il correct ?
    Confirmer avec l'arbitre en chef.
    
    Noir : { $score_black }        Blanc : { $score_white }
yes = OUI
no = NON

# Fouls
equal = ÉGAL

# Game Info
settings = PARAMÈTRES 
none = Aucun
game-number-error = Erreur ({ $game_number })
next-game-number-error = Erreur ({ $next_game_number })
last-game-next-game = Dernier Jeu : { $prev_game },
    Prochain Jeu : { $next_game }
black-team-white-team = Équipe Noire : { $black_team }
    Équipe Blanche : { $white_team }
game-length-ot-allowed = Durée de la mi-temps : { $half_length }
         Durée de la mi-temps : { $half_time_length }
         Prolongations Autorisées : { $overtime }
overtime-details = Durée de la pause avant les prolongations : { $pre_overtime }
             Durée de la mi-temps des prolongations : { $overtime_len }
             Durée de la mi-temps des prolongations : { $overtime_half_time_len }
sd-allowed = Mort Subite Autorisée : { $sd }
pre-sd = Durée de la pause avant la mort subite : { $pre_sd_len }
team-tos-per-half = Temps Morts d'Équipe Autorisés par Mi-temps : { $to_num }
team-to-len = Durée du Temps Mort d'Équipe : { $to_len }
time-btwn-games = Temps Nominal Entre les Matchs : { $time_btwn }
min-brk-btwn-games = Temps Minimum Entre les Matchs : { $min_brk_time }
## Shorten if it is easy
stop-clock-last-2-min = Arrêter l'horloge dans les 2 dernières minutes : Inconnu
refs = Arbitre en Chef : Inconnu
        Chronométreur : Inconnu
        Arbitre Aquatique 1 : Inconnu
        Arbitre Aquatique 2 : Inconnu
        Arbitre Aquatique 3 : Inconnu


# List Selecters
select-tournament = SÉLECTIONNER LE TOURNOI
select-court = SÉLECTIONNER LE TERRAIN
select-game = SÉLECTIONNER LE JEU

# Main View
## Shorten
add-warning = AJOUTER UN AVERTISSEMENT 
add-foul = AJOUTER UNE FAUTE
start-now = COMMENCER MAINTENANT
end-timeout = FIN DU TEMPS MORT
warnings = AVERTISSEMENTS
penalties = PÉNALITÉS
score = SCORE

# Penalties
black-penalties = PÉNALITÉS NOIRES
white-penalties = PÉNALITÉS BLANCHES

# Score edit
final-score = Veuillez entrer le score final

# Shared Elements
## Timeout ribbon
end-timeout-line-1 = FIN
end-timeout-line-2 = TEMPS MORT
switch-to = PASSER À
ref = ARBITRE
penalty = PÉNALITÉ
shot = TIR
pen-shot = TIR PÉNALITÉ
## Penalty string
served = Servi
dismissed = DSMS
## Config String
error = Erreur ({ $number })
none = Aucun
two-games = Dernier Jeu : { $prev_game },  Prochain Jeu : { $next_game }
one-game = Jeu : { $game }
teams = Équipe { -dark-team-name } : { $dark_team }
    Équipe { -light-team-name } : { $light_team }
game-config = Durée de la Mi-temps : { $half_len },  Durée de la Mi-temps : { $half_time_len }
    Mort Subite Autorisée : { $sd_allowed },  Prolongations Autorisées : { $ot_allowed }
    Temps Morts d'Équipe Autorisés par Mi-temps : { $team_timeouts }
    Arrêter l'horloge dans les 2 dernières minutes : { $stop_clock_last_2 }
ref-list = Arbitre en Chef : { $chief_ref }
    Chronométreur : { $timer }
    Arbitre Aquatique 1 : { $water_ref_1 }
    Arbitre Aquatique 2 : { $water_ref_2 }
    Arbitre Aquatique 3 : { $water_ref_3 }
## Game time button
next-game = JEU SUIVANT
## Shorten
first-half = PREMIÈRE MI-TEMPS
half-time = MI-TEMPS
## Shorten
second-half = DEUXIÈME MI-TEMPS
pre-ot-break-full = PAUSE PRÉ PROLONGATIONS
## Shorten
overtime-first-half = PREMIÈRE MI-TEMPS PROLONGATIONS
overtime-half-time = MI-TEMPS PROLONGATIONS
## Shorten
overtime-second-half = DEUXIÈME MI-TEMPS PROLONGATIONS
pre-sudden-death-break = PAUSE PRÉ MORT SUBITE
sudden-death = MORT SUBITE
ot-first-half = PREMIÈRE MI-TEMPS PROLONGATIONS
ot-half-time = MI-TEMPS PROLONGATIONS
ot-2nd-half = DEUXIÈME MI-TEMPS PROLONGATIONS
## Shorten
white-timeout-short = TEMPS MORT BLANC
white-timeout-full = TEMPS MORT BLANC
## Shorten
black-timeout-short = TEMPS MORT NOIR
black-timeout-full = TEMPS MORT NOIR
ref-timeout-short = TEMPS MORT ARBITRE
penalty-shot-short = TIR PÉNALITÉ
## Make penalty dropdown
infraction = INFRACTION
## Make warning container
team-warning-abreviation = É

# Time edit
game-time = TEMPS DE JEU
timeout = TEMPS MORT
Note-Game-time-is-paused = Note : Le temps de jeu est en pause sur cet écran

# Warning Fouls Summary
fouls = FAUTES
edit-warnings = ÉDITER LES AVERTISSEMENTS
edit-fouls = ÉDITER LES FAUTES

# Warnings
black-warnings = AVERTISSEMENTS NOIRS
white-warnings = AVERTISSEMENTS BLANCS

# Message
player-number = NUMÉRO DU
    JOUEUR :
game-number = NUMÉRO DU
    JEU :
num-tos-per-half = NOMBRE DE TEMPS MORTS
    PAR MI-TEMPS :

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