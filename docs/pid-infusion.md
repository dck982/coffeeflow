# PID chauffage pendant l'infusion : première capture

Capture : `captures/260924-104023.json` et son PNG. Elle contient 244
échantillons sur 31,3 s, sans perte d'échantillon. Le fichier HF ne contient
pas la version du firmware ; les constats ci-dessous portent sur les données
enregistrées, et les explications sur la loi actuellement présente dans le
code restent des hypothèses à vérifier lors d'un prochain essai.

## Observations

| Temps depuis le début | Température chaudière | Chauffe demandée | État |
| --- | ---: | ---: | --- |
| 0 s | 89,94 °C | 3,7 % | remplissage |
| 1 à 3 s | environ 90 °C | 15 à 16 % | débit proche de 4 ml/s dès 2 s |
| 6 s | 89,08 °C | 32,7 % | pré-infusion |
| 9 à 21 s | 86,1 puis 83,8 °C | environ 100 % | pré-infusion puis infusion |
| 15,5 s | **81,84 °C** | 100 % | minimum mesuré |
| 21,95 s | 84,51 °C, déjà en hausse | 96,5 % | infusion |
| 27 s | 87,83 °C | 0 % | fin de l'infusion |
| 31,25 s | **91,43 °C**, toujours en hausse | 0 % | fin de la capture |

La température remonte dès environ 16 s, alors que la commande reste proche
de 100 % encore plusieurs secondes. Après la capture, une montée jusqu'à
environ 96 °C a été observée sur la machine ; ce maximum **n'est pas contenu
dans le JSON**. La capture s'arrête trop tôt pour mesurer le pic ni le temps
de retour à la consigne.

Un recalcul indicatif de la pente filtrée sur 8 s montre qu'elle reste
négative jusqu'à environ 21–22 s, malgré la remontée visible dès 16 s. Cela
peut expliquer la baisse tardive de la commande. Dans la loi actuelle, la
chaleur des commandes récentes est aussi bornée à 1,5 °C prédits, même quand
la puissance atteint 100 % pendant plusieurs secondes : cette borne est
plausiblement trop basse pour l'infusion. La capture seule ne permet pas de
séparer l'effet de ces deux réglages ni de déduire une puissance optimale.

## Pistes pour un essai suivant

1. **Avancer une puissance intermédiaire** au début du remplissage ou dès
   qu'un débit fiable apparaît. Un premier essai pourrait imposer environ
   35–40 % pendant 4–6 s, tant que la température ne dépasse pas la consigne.
   Cela vise à faire arriver la chaleur avant la chute, plutôt qu'à réagir
   quand plusieurs degrés sont déjà perdus.
2. **Réduire le retard de la commande pendant l'infusion.** Essayer un filtre
   de pente plus rapide dans ce seul mode, par exemple 3 s au lieu de 8 s,
   et/ou augmenter la borne de chaleur en transit, par exemple vers 4–5 °C.
   L'objectif est de commencer à réduire la puissance dès que la température
   remonte, sans attendre qu'elle approche de 90 °C.
3. **Changer les réglages un par un** sur des infusions comparables. Avancer
   la chauffe seul pourrait aggraver le dépassement si la commande reste à
   100 % aussi longtemps. Il faut mesurer à la fois le creux pendant
   l'infusion et le pic après l'arrêt.

Pour chaque essai, relever le débit, la température et la puissance pendant
l'infusion, puis continuer la télémétrie au moins 60 s après son arrêt. Noter
la température minimale, la durée à 100 %, le pic après infusion et le délai
de retour dans 90 ± 0,5 °C. Le réglage de maintien n'a pas besoin d'être
modifié pour ce premier travail sur l'infusion.

**Statut :** observations et hypothèses uniquement ; aucune modification de
la loi d'infusion n'est faite dans ce document.
