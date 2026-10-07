# CoffeeFlow — contrôleur sensors sur XIAO ESP32-S3

Révision schématique B, 2026-10-07. `sensors.kicad_sch` est la feuille principale ;
`sensors_interfaces.kicad_sch` contient les interfaces. `sensors.pdf` et
`sensors.net.xml` reflètent ce schéma. Ouvrir le projet `sensors.kicad_pro`.

Le PCB a été mis à jour par l’utilisateur après la simplification du schéma.
J9/J10 ont ensuite été remplacés par une empreinte XIAO unique U1 ; les autres
éléments du PCB ont été conservés exactement. Le placement et le routage restent
à terminer. Les BOM, le sourcing, les positions et les archives de fabrication
existants décrivent encore la version MINI et ne représentent pas la nouvelle
version XIAO. Les scripts
`generate_bom.py` et `generate_manufacturing.py` vérifieront la concordance avec
le PCB ; attendre sa mise à jour avant de régénérer les fichiers de commande.
`generate_schematic.py` est un ancien générateur : ne pas le lancer sur ce schéma,
car il rétablirait la version MINI/USB/LDO.

## XIAO et alimentation

Le XIAO ESP32-S3 du stock est désormais représenté par **U1**, symbole
`Seeed_Studio_XIAO_Series:XIAO-ESP32-S3-SMD`, avec l’empreinte
**`Seeed_XIAO:XIAO-ESP32-S3-DIP`**. Ces deux bibliothèques sont celles installées
localement dans `~/Projects/OPL_Kicad_Library` ; les tables du projet les référencent.

Le symbole Seeed présente les broches 1–14 des headers et les pastilles 15–25
accessibles sous le module. L’empreinte DIP ne possède que les numéros 1–14 :
les pastilles 15–25 sont donc marquées NC dans le schéma. Le symbole conserve
son identité de bibliothèque ; seule son empreinte est remplacée par la variante DIP.

Sur le PCB, U1 remplace les deux sockets J9/J10, près de leur position précédente,
avec une rotation de 90° donnant l’USB-C vers le haut. Les broches 1–7 sont à
gauche du haut vers le bas ; 14–8 sont à droite du haut vers le bas. L’empreinte
Seeed possède quatorze trous et des pastilles SMD supplémentaires portant les mêmes
numéros. Deux barrettes femelles 1×7 au pas de 2,54 mm s’y montent ; l’écartement
entre rangées est de 15,24 mm. Le XIAO s’enfiche manuellement depuis le stock.

Sources locales : `../../docs/datasheets/xiao.pdf`, le schéma de la carte Seeed
`Seeed Studio XIAO ESP32S3/ESP32S3.kicad_sch`, la bibliothèque de symboles
`Seeed Studio XIAO Series Library/Seeed_Studio_XIAO_Series.kicad_sym`,
et l’empreinte installée `Seeed_XIAO.pretty/XIAO-ESP32-S3-DIP.kicad_mod`.

J1 conserve l'entrée 5 V régulée : broche 1 GND, broche 2 +5V.
Ce rail alimente le XIAO, le TJA1051 et les interfaces 5 V. La sortie 3V3 du
XIAO alimente VIO du TJA1051, le bus I2C, le TMP102 et le pull-up FLOW.
Le PWR_FLAG 3V3 représente la sortie du régulateur embarqué du XIAO.
Vérifier le budget 3,3 V avec les périphériques effectivement branchés.

L’ancien MINI U1, le LDO U2, USB-C J2, protection USB D1, BOOT/RESET, résistances USB/strapping/EN et
condensateurs associés C1–C6 ont été supprimés. USB, régulation et boutons
sont fournis par le XIAO. Débrancher l'alimentation externe J1 avant le flash
via l'USB-C du XIAO.

## Brochage

Le CAN, l'I2C, FLOW et VALVE reprennent les GPIO natifs du XIAO utilisés dans
`../../firmware/sensors/main/main.cpp` et `../../docs/cablage.md`.

| Fonction | GPIO natif | Nom Seeed | U1 / broche |
|---|---:|---|---|
| BOILER / HEATER_CMD | 4 | D3 | U1 / 4 |
| LED STATUS | 2 | D1 | U1 / 2 |
| I2C SDA | 5 | D4 | U1 / 5 |
| I2C SCL | 6 | D5 | U1 / 6 |
| VALVE_CMD | 9 | D10 | U1 / 11 |
| CAN RX | 8 | D9 | U1 / 10 |
| CAN TX | 7 | D8 | U1 / 9 |
| FLOW_PULSE | 44 | D7 | U1 / 8 |
| +5V | — | 5V | U1 / 14 |
| GND | — | GND | U1 / 13 |
| Sortie +3V3 | — | 3V3 | U1 / 12 |

U1 / 1 (D0/GPIO1), U1 / 3 (D2/GPIO3) et U1 / 7 (D6/GPIO43) sont NC.
GPIO2 et GPIO4 ne sont pas des broches de strapping ; GPIO3 reste inutilisé.
GPIO43, sortie UART0 au démarrage, reste inutilisé. FLOW conserve GPIO44,
entrée UART0 par défaut : le firmware actuel le reconfigure explicitement en GPIO.

## Note pour la mise en service — HEATER_CMD

**Modifier le firmware sensors seulement lors du passage au nouveau PCB.**
Le montage actuel reste sur `kGpioHeater = GPIO_NUM_3` (D2, HW-399).
La nouvelle carte utilise **HEATER_CMD sur GPIO4/D3, broche 4 de U1** :
il faudra définir `kGpioHeater = GPIO_NUM_4` et `kHeaterActiveLevel = 1`
(commande active HIGH pour Q3/Q2, au lieu de LOW pour le HW-399).
Précharger LOW avant de configurer la sortie et désactiver son pull-up.

HEATER_CMD et STATUS_LED ont été échangés pour faciliter le routage :
la LED externe utilise désormais **GPIO2/D1, broche 2 de U1**. GPIO2 et GPIO4
ne sont pas des broches de strapping. Le firmware n’est pas modifié dans
cette révision matérielle ; le changement sera nécessaire avant la mise en service.

## Circuits conservés et futur firmware

Toutes les connexions des composants conservés sont inchangées : TJA1051T/3
avec VCC 5 V, VIO 3,3 V et SLNT à GND, terminaison CAN 120 Ω, deux prises I2C
parallèles et pull-ups 4,7 kΩ, TMP102, FLOW avec pull-up 1 kΩ / filtre 10 nF,
VALVE avec pull-down, et sortie BOILER Q3/2N7002 + Q2/AO3401A.
D2 utilise désormais **C2297 / KT-0805G**, LED vert émeraude **0805**,
Basic chez JLCPCB (vérifié le 2026-10-07), avec l’empreinte
`LED_SMD:LED_0805_2012Metric`. Broche 1 = cathode/GND, broche 2 = anode.
Source : https://jlcpcb.com/partdetail/KT-0805G/C2297.
R25 reste à 1 kΩ : GPIO2 HIGH allume la LED. La fiche catalogue donne
Vf = 2,6–3,1 V à 5 mA ; sous 3,3 V avec 1 kΩ, le courant sera inférieur
à celui de la précédente LED jaune-verte. L’estimation avec cette plage
de Vf est 0,2–0,7 mA ; le courant réel dépend de Vf à faible courant.
Vérifier la luminosité au montage. Le PCB et ses fichiers de sourcing/BOM
conservent l’ancienne LED C2289/0603 jusqu’à la refonte du PCB.

**Le firmware n'a pas été modifié.** Il doit continuer à fonctionner sur le
montage actuel pendant la fabrication du futur PCB. Pour la nouvelle carte,
il faudra ultérieurement passer BOILER de GPIO3 à GPIO4 et surtout passer la
commande de **active LOW (HW-399 actuel) à active HIGH (montage MOSFET conservé)**.
Précharger LOW avant d'activer la sortie et désactiver le pull-up du chauffage.
R19/R21/R22 maintiennent la sortie BOILER coupée lorsque le GPIO est flottant.
Ajouter la commande LED sur GPIO2 si souhaitée. CAN/I2C/FLOW/VALVE ne demandent
pas de migration des GPIO par rapport au XIAO actuel.

## Vérification

ERC KiCad 10.0.6 : zéro violation. Comparaison des netlists avant/après :
toutes les appartenances aux nets des composants conservés sont identiques,
et chaque broche utilisée de U1 correspond au tableau ci-dessus, dans le schéma
et sur le PCB. Les 14 trous et l’écartement entre rangées ont été vérifiés.
PDF exporté sur deux pages ; première page vérifiée visuellement. Export SVG
du PCB réussi. Tous les éléments PCB autres que J9/J10/U1 sont inchangés ;
le routage existant n’a pas été retouché. Le firmware reste inchangé.
