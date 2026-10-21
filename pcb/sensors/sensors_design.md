# CoffeeFlow — contrôleur sensors sur XIAO ESP32-S3

Révision schématique B, 2026-10-07. `sensors.kicad_sch` est la feuille principale ;
`sensors_interfaces.kicad_sch` contient les interfaces. `sensors.pdf` et
`sensors.net.xml` reflètent ce schéma. Ouvrir le projet `sensors.kicad_pro`.

Le PCB est placé et routé. Le 2026-10-11, U1 a été sorti du PCB au profit de
deux barrettes J11/J12 soudées par JLCPCB (voir ci-dessous), puis le sourcing, les
BOM et les archives de fabrication ont été régénérés pour cette version XIAO :
`uv run sensors/generate_bom.py`, puis `uv run generate_manufacturing.py sensors`
depuis `pcb/`. Les choix de pièces sont dans `sensors_bom_notes.md`.
`generate_schematic.py` est un ancien générateur : ne pas le lancer sur ce schéma,
car il rétablirait la version MINI/USB/LDO.

## XIAO et alimentation

Le XIAO ESP32-S3 du stock est désormais représenté par **U1**, symbole
`Seeed_Studio_XIAO_Series:XIAO-ESP32-S3-SMD`, avec l’empreinte
**`Sensors_Local:XIAO-ESP32-S3-DIP-Headers`**. Le symbole vient de la bibliothèque
installée localement dans `~/Projects/OPL_Kicad_Library` ; les tables du projet
la référencent. L’empreinte est une copie locale de `Seeed_XIAO:XIAO-ESP32-S3-DIP`
réduite au montage sur barrettes. Les quatorze pastilles SMD de l’original,
prévues pour souder le module à plat par ses bords crénelés, sont retirées :
elles n’auraient été que du cuivre nu sous le corps des barrettes. Les trous
passent de 0,889 à **1,0 mm** de perçage, pastilles de 1,7 mm, comme
`PinSocket_1x07_P2.54mm` : une broche carrée de 0,64 mm mesure 0,91 mm en
diagonale. Le courtyard ne couvre plus tout le module : deux rectangles de
18,3 × 3,05 mm entourent les barrettes (|y| de 6,1 à 9,15 mm, soit 0,25 mm
autour de leur corps). La bande d’environ 12,2 × 18 mm entre les rangées
accepte des composants bas, puisque le XIAO repose sur des barrettes femelles
d’environ 8,5 mm ; rien n’y doit rester accessible une fois le module enfiché.

Le symbole Seeed présente les broches 1–14 des headers et les pastilles 15–25
accessibles sous le module. L’empreinte DIP ne possède que les numéros 1–14 :
les pastilles 15–25 sont donc marquées NC dans le schéma. Le symbole conserve
son identité de bibliothèque ; seule son empreinte est remplacée par la variante DIP.

Sur le PCB, U1 n’est plus placé (`on_board no`, `in_bom no`) : il reste dans le
schéma pour son brochage. Les deux barrettes femelles 1×7 au pas de 2,54 mm sont
des composants à part, que JLCPCB soude en traversant : **J11** porte les broches
1–7 du XIAO (J11 broche n = U1 broche n) et **J12** les broches 8–14 (J12 broche n
= U1 broche n+7). Ce sont des hanxia HX PM2.54-1x7P ZC-Y (C22438157), 8,5 mm de
haut, sur `Sensors_Local:XIAO_Socket_1x07_P2.54mm_Vertical` : le
`PinSocket_1x07_P2.54mm_Vertical` de KiCad, sans son contour de sérigraphie et avec
le courtyard de 0,25 mm de l’ancienne empreinte U1. Leurs pads occupent les
quatorze trous de U1, avec les mêmes nets : le routage n’a pas changé. Le contour du
XIAO (languette USB-C, repère de la broche 1) est dessiné en sérigraphie sur la
carte. L’empreinte `XIAO-ESP32-S3-DIP-Headers` reste dans `Sensors_Local` mais
n’est plus utilisée sur le PCB. Le XIAO s’enfiche à la main ; pour souder les
barrettes soi-même, désélectionner J11/J12 à la commande.

Sources locales : `../../docs/datasheets/xiao.pdf`, le schéma de la carte Seeed
`Seeed Studio XIAO ESP32S3/ESP32S3.kicad_sch`, la bibliothèque de symboles
`Seeed Studio XIAO Series Library/Seeed_Studio_XIAO_Series.kicad_sym`,
et l’empreinte d’origine `Seeed_XIAO.pretty/XIAO-ESP32-S3-DIP.kicad_mod`.

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

U1 / 1 (D0/GPIO1), U1 / 3 (D2/GPIO3) et U1 / 7 (D6/GPIO43) sont NC. GPIO2 et GPIO4 ne sont pas des broches de
strapping. GPIO43, sortie UART0 au démarrage, reste inutilisé. FLOW conserve GPIO44,
entrée UART0 par défaut : le firmware actuel le reconfigure explicitement en GPIO.

## Pas de connecteur pour une carte AC

La carte `accontrol` a été mise de côté le 11 octobre 2026 : la vanne et la
pompe restent sur le SSR M5Stack (J7) et le dimmer RobotDyn (Grove I2C).
Le 13 octobre 2026, elle est reprise sous le nom `combined` (`../combined/`),
qui réunit sur une seule carte la partie AC et ce schéma, sans J4, J5 ni J7. J9,
qui lui était destiné, est retiré, et GPIO1 et GPIO3 restent libres.

Une future carte AC unique prendrait la place du dimmer I2C et de VALVE sur J7 :
un XH 5 broches (GND, 3V3, DIM, ZC, SSR) avec DIM sur GPIO1, ZC sur GPIO3 et
SSR sur GPIO9. ZC demande alors le pull-down interne de GPIO3, pour que la ligne
débranchée ne passe pas pour un passage par zéro.

## U6 / J10 — entrée pression analogique

U6 (ADS1115IDGSR) et J10 (XH 3 broches, B3B-XH-A, LCSC C144394) permettent de
brancher la sonde de pression XDB401 sur sensors au lieu de screen. Le
circuit reprend celui de `screen_sensors` :

| J10 | Net | Rôle |
|---:|---|---|
| 1 | +3V3 | alimentation de la sonde |
| 2 | GND | masse |
| 3 | PRESSURE | sortie de la sonde, 0,4–2,4 V |

- L’ordre de J10 est celui de J3 sur `screen_sensors` : le même câble passe
  d’une carte à l’autre.
- U6 est alimenté en +3V3 (C13, 100 nF, au plus près de VDD). AIN0 lit le
  +3V3 local, AIN1 lit PRESSURE_ADC ; AIN2, AIN3 et ALERT/RDY sont NC.
- R26 (1 kΩ) et C14 (10 µF) filtrent PRESSURE avant AIN1 : 16 Hz, 10 ms,
  comme R3/C3 sur `screen_sensors`. C15 (100 nF) découple l’alimentation
  de la sonde sur J10.
- **Adresse I2C 0x49 : ADDR est relié au +3V3.** Le TMP102 occupe déjà 0x48
  (ADD0 à GND) sur le même bus ; l’ADS1115 de `screen_sensors` est à 0x48,
  mais sur le bus de l’écran.
- **Prochaine révision : ajouter un pull-down d’environ 1 MΩ sur PRESSURE**
  (J10 broche 3 vers GND). Sans lui, une entrée débranchée flotte derrière
  R26/C14 et peut tomber dans la fenêtre 0,279–2,60 V que le firmware prend
  pour une sonde présente ; avec lui, elle lit 0 V. La sonde ne fournit
  que 2,4 µA de plus. Même remarque pour R3/C3 de `screen_sensors`.
- La sortie de la XDB401 n’est pas ratiométrique : la pression se calcule sur
  la tension absolue de AIN1, AIN0 ne sert qu’à surveiller le +3V3.

## Mise en service — firmware

Le firmware `sensors` 0.4.16 (2026-10-21) suit ce brochage :
**HEATER_CMD sur GPIO4/D3, actif HIGH** (Q3/Q2), préchargé LOW sans pull-up ;
R19/R21/R22 maintiennent la sortie BOILER coupée tant que le GPIO flotte.
La LED D2 (GPIO2) est fixe 1 s au démarrage, puis clignote à 4 Hz si U5 ou
U6 ne répondent pas, à 1 Hz si le dimmer ne répond pas, et sinon s’allume
50 ms à chaque trame CAN reçue. CAN, I2C, FLOW et VALVE gardent leurs GPIO.
Depuis la 0.4.17, U5 (TMP102) donne la température du boîtier par
`STATUS_ENCLOSURE` (0x24), affichée par l'écran et exposée sous
`enclosure.sensors` dans `/telemetry`.

L’image `factory` C++ (v0.2.9) n’a pas besoin d’être refaite pour cette
carte : elle ne pilote que GPIO3 (désormais NC) pour la chauffe, que R19
laisse coupée, et garde CAN, VALVE et la réception OTA sur les mêmes GPIO.

## Circuits conservés

Toutes les connexions des composants conservés sont inchangées : TJA1051T/3
avec VCC 5 V, VIO 3,3 V et SLNT à GND, terminaison CAN 120 Ω, deux prises I2C
parallèles et pull-ups 4,7 kΩ, TMP102, FLOW avec pull-up 1 kΩ / filtre 10 nF,
VALVE avec pull-down, et sortie BOILER Q3/2N7002 + Q2/AO3401A.
La terminaison R12 est la seule résistance en **1206 / 0,25 W** (C17909,
Basic) ; les autres sont en 0603 / 0,1 W. En service, le différentiel
dominant d’environ 2 V n’y dissipe que 2² / 120 ≈ 33 mW. Mais si CAN_H
est court-circuité au 5 V avec CAN_L à GND (câblage inversé, défaut de
faisceau), R12 voit 5² / 120 ≈ 210 mW : une 0603 grillerait et laisserait
le bus sans terminaison, alors qu’une 1206 tient.
D2 utilise désormais **C2297 / KT-0805G**, LED vert émeraude **0805**,
Basic chez JLCPCB (vérifié le 2026-10-07), avec l’empreinte
`LED_SMD:LED_0805_2012Metric`. Broche 1 = cathode/GND, broche 2 = anode.
Source : https://jlcpcb.com/partdetail/KT-0805G/C2297.
R25 reste à 1 kΩ : GPIO2 HIGH allume la LED. La fiche catalogue donne
Vf = 2,6–3,1 V à 5 mA ; sous 3,3 V avec 1 kΩ, le courant sera inférieur
à celui de la précédente LED jaune-verte. L’estimation avec cette plage
de Vf est 0,2–0,7 mA ; le courant réel dépend de Vf à faible courant.
Vérifier la luminosité au montage.

## Vérification

2026-10-11, KiCad 10.0.6 : ERC sans violation. DRC avec parité schéma/PCB :
zéro violation, zéro connexion manquante, zéro écart de parité. Les broches 1–14
de U1 et leurs nets se retrouvent à l’identique sur J11/J12, et chaque pad de
J11/J12 tombe à moins de 1 µm du trou de U1 qu’il remplace. L’export de
fabrication passe : 34 composants, 23,5 × 60 mm.
