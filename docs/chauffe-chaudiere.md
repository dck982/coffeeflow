# Régulation de la chaudière

L'écran lit la NTC chaudière et calcule la puissance demandée. Le module
capteurs transforme cette puissance en temps de marche du SSR sur une période
fixe de 5 s. L'écran renouvelle un bail de 1500 ms toutes les 500 ms : la
chauffe initiale peut durer plusieurs minutes, mais une perte de l'écran ou
du CAN coupe le chauffage à l'expiration du bail.
La consigne a une résolution de 0,1 %. Une période isolée ne peut produire
moins de 100 ms de chauffe avec le tick actuel ; un accumulateur répartit les
faibles consignes sur plusieurs périodes (1 % = 100 ms toutes les 10 s,
0,6 % ≈ 100 ms toutes les 16,7 s en moyenne).

La configuration NVS v6 contient `heating.brew_temperature_c` (90 °C par
défaut, 50 à 100 °C par pas de 0,5 °C) et `heating.enabled` (`true` par
défaut). La cible est modifiable sur la deuxième page des réglages ; le
commutateur n'est disponible que par `POST /config`. La purge reste possible
quand il vaut `false`. L'infusion exige une mesure fraîche dans la bande
consigne ±0,5 °C pendant trois secondes, et un module capteurs compatible.
La borne basse à 50 °C permet les essais de calibration à température modérée.
Tant que la courbe NTC reste provisoire, cette consigne est celle calculée par
le firmware et ne garantit pas 50 °C réels dans la chaudière.
`firmware/tools/set_heating.py true` ou `false` modifie ce commutateur via
`GET /config` puis `POST /config`, avec `COFFEEFLOW_HTTP_TOKEN` et
`COFFEEFLOW_IP` dans l'environnement.

Toute migration depuis une configuration v1–v5 persiste `heating.enabled=false`
avant le démarrage des tâches ; une installation neuve conserve `true`.

La loi actuelle est un réglage initial : puissance plafonnée à 100 % au repos,
anticipation de 20 s sur la pente filtrée dans les deux sens, maintien nominal
de 3,5 % à la consigne, et estimation bornée de la chaleur encore en transit à
partir de la puissance au-dessus de 3,5 % demandée durant les 22 dernières
secondes. La pente est filtrée sur 8 s et les variations de puissance non nulles
sont lissées sur 5 s ; les coupures restent immédiates. L'intégrale utilise l'erreur prédite, pour ne pas
accumuler une erreur déjà expliquée par cette chaleur. Pendant l'infusion et la
purge, le mode écoulement applique au moins 18 % lorsque la température reste
proche ou sous la cible, et limite la puissance à 35 %. Il ne prolonge pas la
pente négative de la NTC sur les 20 s de prédiction. Durant les 30 s suivant
l'arrêt de l'écoulement, la reprise reste plafonnée à 35 %, la pente négative
n'est pas extrapolée et l'intégrale est suspendue ; la chaleur déjà commandée
reste prise en compte. La prédiction de reprise utilise 10 s et retient le
plus grand effet entre la pente montante et la chaleur en transit, car ces
deux estimations se recouvrent partiellement. En reprise, la chauffe peut être
coupée dès que la prédiction passe au-dessus de la cible ; pendant
l'écoulement, l'appoint est
coupé quand la mesure dépasse la cible de 0,5 °C. Au-dessus de 105 °C, sur mesure
invalide ou si la chauffe est désactivée, la consigne tombe à zéro. Ces
coefficients doivent être ajustés avec des mesures sur la machine réelle.

Une purge commencée à 5 °C ou plus de la consigne, au-dessus ou en dessous,
est traitée comme une purge de réglage thermique : aucune chauffe n'est
commandée jusqu'à son arrêt, même si la NTC traverse la consigne. La reprise
du contrôleur après la purge reste active si la température descend sous la
consigne.

Sur la purge de 8 s du 24 septembre 2026 à consigne 90 °C, la télémétrie
ancienne loi montre une chute de la NTC jusqu'à 80,9 °C environ 15 s après
le début, suivie d'une commande maximale de 93,9 % vers 23 s et d'un pic à
96,6 °C vers 50 s. La température ponctuelle dans le panier était 86,7 °C.
Le nouveau plafonnement vise à réduire ce rebond ; son effet sur la machine
est visible dans l'essai suivant : pour une purge de 8 s à consigne 90 °C,
la puissance ne dépasse pas 35 %, la NTC atteint 79,4 °C au minimum puis
88,2 °C au maximum sur les 90 s enregistrées, et la température mesurée
immédiatement dans le panier est de 88,4 °C. Le départ était à 90,6 °C,
contre 89,7 °C lors de l'essai précédent, et le volume a aussi changé
(environ 33 ml contre 31 ml) : la comparaison n'isole pas l'effet du logiciel.
Le raccourcissement de la prédiction de reprise a été ajouté après cette
capture et reste à vérifier sur la machine.

Depuis 0.3.1, l'écran diffuse un `LOG` CAN à la première erreur ADS1115, puis
au plus toutes les 5 s si la même erreur persiste. `BOILER_ADC_NOT_FOUND`,
`BOILER_ADC_I2C_ERROR`, `BOILER_ADC_CONVERSION_TIMEOUT` et
`BOILER_NTC_INVALID_READING` distinguent les causes ; `BOILER_ADC_RECOVERED`
indique le retour d'une mesure valide et la durée de l'interruption. Le
décodeur `coffeetool` affiche l'adresse, l'étape I²C, le code ESP ou les valeurs
ADC brutes selon le cas.

Depuis 0.3.5, une erreur de lecture I²C entraîne un nouvel essai après 200 ms,
puis 400 ms, 800 ms et 1 s si elle persiste. Une lecture réussie remet ce délai
à 200 ms pour la prochaine erreur. Le PGA ADS1115 reste à ±4,096 V pour A0
et A1 ; le calcul de résistance utilise le rapport des deux codes bruts.

La calibration 0.3.5 a essayé un offset de -18 °C ; de la vapeur est sortie
pendant la purge. Depuis 0.3.6, l'offset est revenu à 0 °C et la coupure de
chauffe à 105 °C de la température publiée est rétablie. En 0.3.8, les
constantes NTC ont été remplacées par l'ajustement des mesures directes de la
sonde ; en 0.3.9, elles ont été arrondies à des valeurs nominales proches.
La nouvelle conversion demande une validation sur la machine.

## État des essais du 24 septembre 2026 — reprise de la calibration

La sonde chaudière est la [Profitec P6036](https://links.imagerelay.com/cdn/2615/ql/423575fef2a046eb97c8b5af763d79b5/Pro-600-Parts-Diagram.pdf),
référencée comme NTC 1/8″ dans la nomenclature du fabricant. Sa courbe
résistance/température n'est pas indiquée dans cette nomenclature. Les
marquages lus sur le métal de la sonde installée sont `1408504` et `16/18` ;
leur signification n'a pas été confirmée et ne donne pas de valeur de `R25`
ou de `Beta`. Lors de ces essais, la sonde n'avait pas encore été démontée. La résistance fixe du pont, entre
A1 et GND, a été mesurée à **2,193 kΩ** ; la résistance de la NTC débranchée,
mesurée directement sur ses deux fils, était **42,1 kΩ** lors des essais à
froid. Les anciennes constantes compilées jusqu'en 0.3.7 étaient
`R25 = 27 290 Ω`, `Beta = 3 728 K`, sans offset. Avec elles, 42,1 kΩ donnent
environ 15 °C.
Avant la dépose de la sonde, deux hypothèses nominales envisagées étaient
`R25 = 47 kΩ, Beta = 4 700 K` et `R25 = 50 kΩ, Beta = 4 800 K` ; seule la
première correspond à une combinaison trouvée dans un
[catalogue Panasonic](https://industrial.panasonic.com/cdbs/www-data/pdf/AUA0000/AUA0000C10.pdf),
et ce composant n'identifie pas la sonde montée dans la chaudière. La mesure
directe ultérieure près de 25 °C est décrite plus bas.

Une troisième hypothèse envisagée était **`R25 = 47 kΩ` avec `Beta = 4 400–4 450 K`**.
Le modèle Beta donne alors environ **3,35–3,25 kΩ à 90 °C**. Le PID d'origine
porte l'inscription **`NTC 3K3`**. Elle pourrait désigner une résistance fixe
de **3,3 kΩ** dans son pont diviseur, choisie proche de la résistance de la
NTC vers 90 °C pour améliorer la sensibilité dans la plage café. Le marquage
seul ne confirme toutefois ni le schéma du PID ni les caractéristiques de la
sonde. Cette hypothèse ne coïncide pas exactement avec les **2,89 kΩ** mesurés
après l'affichage de 90 °C sur le PID ; cet affichage n'est pas une référence
indépendante de température. Le pont ADS1115 ajouté à la machine possède,
quant à lui, une résistance fixe mesurée de **2,193 kΩ**.

Comparer les courbes dans
[l'explorateur NTC](ntc_curve_explorer.html).
Les anciens couples résistance/température de
[`ntc_ads1115_calibration.md`](ntc_ads1115_calibration.md) utilisent la
température entière indiquée par le PID d'origine, lue avant coupure puis
débranchement de la sonde : ce n'est pas une mesure indépendante de la
température réelle de la NTC.

En particulier, **2,89 kΩ est la résistance mesurée au multimètre** après
lecture de 90 °C sur le Gicar, extinction de la machine et débranchement de
la NTC ; le relevé ne précise pas depuis combien de temps l'affichage était
stabilisé. Le léger refroidissement pendant la manipulation peut augmenter
la résistance mesurée. **2,91 kΩ** est la valeur *calculée* à 90 °C avec les
anciennes constantes du firmware (`27 290 Ω / 3 728 K`), et **2,92 kΩ** celle
de l'ajustement de la courbe d'affichage Gicar ci-dessous. Ces deux calculs
ne sont pas des mesures supplémentaires de la sonde.

### Essai de résistances sur le PID Gicar d'origine

Des résistances mesurées ont été branchées à la place de la NTC sur l'entrée
du PID Gicar alimenté séparément ; son affichage est arrondi au degré :

| Résistance | Température affichée |
| ---: | ---: |
| 23,44 kΩ | 28 °C |
| 22,90 kΩ | 29 °C |
| 21,91 kΩ | 30 °C |
| 14,61 kΩ | 41 °C |
| 9,93 kΩ | 52 °C |
| 5,70 kΩ | 68 °C |
| 4,68 kΩ | 74 °C |
| 2,196 kΩ | 101 °C |
| 0,989 kΩ | 132 °C |

Un ajustement indicatif du modèle Beta à ces neuf points donne environ
**`R25 = 27,2 kΩ` et `Beta = 3 710 K`** pour la *courbe d'affichage du PID*.
La valeur calculée pour 42 kΩ est alors environ **15 °C**, et la résistance
correspondant à 90 °C affichés environ **2,92 kΩ**. Ces résultats rejoignent
les constantes provisoires du firmware (`27,29 kΩ / 3 728 K`) parce que
celles-ci ont été déduites de mesures dont la température provenait déjà de
l'affichage du PID : cet accord ne valide pas la température réelle de la
chaudière. Les écarts des points du tableau à une courbe Beta unique restent
de l'ordre du degré ; ils ne montrent pas de rupture de courbe particulière
à basse température.

Avec l'hypothèse physique **`47 kΩ / 4 425 K`**, 42 kΩ correspondraient à
environ **27,3 °C**, cohérents avec la mesure à froid proche de 27–28 °C ;
mais le PID afficherait environ **15 °C** pour cette résistance. À 90 °C
réels, cette hypothèse prévoit environ **3,3 kΩ**, que la courbe Gicar
afficherait vers **86 °C**. L'ancienne mesure de 2,89 kΩ prise lorsque le PID
affichait 90 °C donnerait, avec cette hypothèse, environ **94 °C réels**.
Cette interprétation antérieure est remise en cause par les mesures directes
ci-dessous : `Beta ≈ 3 900–3 950 K` leur correspond mieux. Le PID affiche `UP`
pendant la montée en température et masque ainsi son écart à froid. Pour
vérifier l'écart à chaud, il faut une température indépendante au voisinage
de la NTC, stabilisée et associée à sa résistance ; la température du panier
ne mesure pas celle de la sonde.

Le PID est alimenté par le secteur 230 V et porte un module d'alimentation
`prim BV 202 0154` marqué 6 V / 0,5 VA. Un STMicro L4941BV et un condensateur
ont été repérés près de la piste de l'entrée NTC. Ces observations ne
permettent pas encore d'identifier la résistance fixe du pont ni de
confirmer ce que signifie l'inscription `NTC 3K3`.

### Mesures directes de la sonde démontée

La NTC a ensuite été démontée et mesurée à plusieurs températures. Les points
chauds ont été pris pendant un refroidissement et sont moins stables ; le
point à 24,7 °C utilise un autre thermomètre, avant immersion.
La sonde NTC répond plus lentement que le thermomètre digital. Le minimum de
résistance a été attendu alors que la température de l'eau baissait déjà de
0,1 à 0,2 °C/s : le chiffre du thermomètre à cet instant n'est donc pas
nécessairement la température de l'élément NTC. Cette inertie explique une
partie de la dispersion des points chauds et limite la précision de `Beta`.

| Température relevée | Résistance NTC |
| ---: | ---: |
| 20,1 °C | 57,7 kΩ |
| 21,6 °C | 54,6 kΩ |
| 24,7 °C | 49,1 kΩ |
| 24,9–25,0 °C | 47,6 kΩ |
| 43 °C | 21,42 kΩ |
| 49 °C | 18,70 kΩ |
| 74 °C | 7,60 kΩ |
| 77,7 °C | 6,66 kΩ |
| 78 °C | 5,97 kΩ |
| 86,2 °C | 4,95 kΩ |
| 89,5–89,8 °C | 4,73 kΩ |
| 91,0–91,3 °C | 4,31 kΩ |

Le point stabilisé près de 25 °C soutient fortement une **NTC nominale
47 kΩ**. Un ajustement Beta indicatif des douze points, en prenant le milieu
des intervalles de température, donne **`R25 ≈ 47,2 kΩ` et
`Beta ≈ 3 920 K`**. Sans les deux derniers points chauds, l'ajustement
donnait `47,3 kΩ / 3 944 K` : l'ordre de grandeur est stable. La dispersion
à chaud empêche d'en faire une calibration définitive : les points à 77,7 et
78 °C diffèrent déjà de 10 % en résistance ; les deux nouveaux points vers
90 °C impliquent séparément `Beta ≈ 3 860 K` et `Beta ≈ 3 940 K` lorsqu'ils
sont ancrés à 47,6 kΩ vers 25 °C. En particulier,
l'ancienne hypothèse `47 kΩ / 4 425 K` ne décrit pas ces mesures chaudes :
elle prévoit environ 3,8 kΩ à 86,2 °C, contre 4,95 kΩ mesurés.

En fixant la valeur **nominale** `R25 = 47 kΩ`, les points à 4,95 kΩ
(86,2 °C), 4,73 kΩ (89,5–89,8 °C) et 4,31 kΩ (91,0–91,3 °C)
impliquent séparément des Beta d'environ **3 940 K**, **3 840 K** et
**3 920 K**. Un Beta de **4 050 K** attribuerait à ces résistances environ
**84,2 °C**, **85,7 °C** et **88,6 °C** : les températures relevées dans le
bain devraient alors surestimer celles de la NTC d'environ 2 à 4 °C. Un
point de mesure pris pendant le refroidissement ne permet pas d'établir à
lui seul le sens de cet écart : l'inertie de la NTC, l'emplacement des deux
instruments et les gradients dans l'eau interviennent. `47 kΩ` désigne une
valeur nominale ; la mesure stabilisée près de 25 °C était de 47,6 kΩ.
Les valeurs de Beta proposées pour des NTC ne suivent pas un pas universel,
et le Beta annoncé dépend des températures de référence choisies.

Pour chacune des douze résistances, la courbe ajustée du PID Gicar donne une
température affichée inférieure de **11,7 à 16,0 °C** à la température
relevée sur la sonde, soit **13,6 °C en moyenne**. Ce décalage presque
constant pourrait être volontaire : un afficheur de machine à café peut
viser la température d'infusion plutôt que la température locale de la
chaudière. Cela reste une interprétation, sans documentation Gicar ni mesure
indépendante dans l'eau au point d'extraction. Avec l'ajustement indicatif de
la sonde, la résistance vers 90 °C locaux serait environ **4,49 kΩ** ; le
Gicar afficherait alors environ **75 °C**. Inversement, ses 90 °C affichés
correspondraient à environ **105 °C locaux** selon cet ajustement extrapolé.
Les deux relevés voisins de 90 °C soutiennent cette plage, mais leur écart
montre qu'un bain à température presque constante serait nécessaire avant
d'en faire une calibration précise de chauffe. Les mesures actuelles suffisent
à identifier l'ordre de grandeur et ne justifient pas de retarder le
remontage de la sonde.

Avec `47 kΩ / 3 950 K`, les **2,89 kΩ mesurés** correspondraient à environ
**104,5 °C** à la sonde ; avec `47 kΩ / 4 050 K`, à environ **102 °C**.
Pour que 2,89 kΩ représentent 90 °C locaux avec `R25 = 47 kΩ`, il faudrait
un Beta d'environ **4 646 K**, incompatible avec les mesures directes à chaud.
Modifier modérément le Beta réduit donc l'écart, mais ne fait pas coïncider
90 °C affichés par Gicar avec 90 °C locaux à la NTC.

Jusqu'en 0.3.7, le firmware (`27,29 kΩ / 3 728 K`) suivait presque exactement
la courbe d'affichage Gicar. Le firmware 0.3.8 utilisait l'ajustement mesuré
`47,2 kΩ / 3 922 K` ; depuis 0.3.9, il utilise les valeurs nominales proches
**`47 kΩ / 3 950 K`** pour estimer la température **locale de la sonde**.
La consigne enregistrée reste inchangée ; à 90 °C, elle correspond désormais
à environ 4,38 kΩ, contre 2,92 kΩ avant 0.3.8. Ce changement réduit la
température visée par la chaudière pour une même consigne numérique. Une
nouvelle validation de la chauffe et de la température d'infusion est
nécessaire.

Avec la courbe indicative de la sonde, sa résistance vers **90 °C locaux**
est d'environ **4,5 kΩ**. Une résistance fixe de **4,7 kΩ** aurait donc placé
le pont ADS1115 près de sa sensibilité maximale à cette température. Le
pont actuel mesure **2,193 kΩ** ; à 90 °C, le modèle donne environ **21,7 mV/°C**
sur A1 contre **24,5 mV/°C** avec 4,7 kΩ, à alimentation 3,3 V identique :
un gain d'environ **13 %** de sensibilité, pas de justesse absolue. Avec le
PGA ADS1115 à ±4,096 V (125 µV par code), cela représente environ **173**
contre **196 codes par degré**, soit un pas théorique de **0,0058** contre
**0,0051 °C par code**. Cette différence est minime devant l'incertitude
actuelle de la courbe et de la température réelle de la sonde ; elle ne
justifie pas à elle seule de modifier le montage.
La résistance **3,3 kΩ** évoquée par le marquage du Gicar reste une hypothèse
sur son propre circuit ; elle serait proche de l'optimum vers 100 °C locaux
avec cette sonde. Sa présence réelle sur la carte n'a pas été vérifiée.

Après trois heures d'arrêt, la pièce était à **24,8 °C** près de la machine,
la plaque supérieure de la chaudière et l'écrou du raccord à **26,7 °C** au
thermomètre IR, tandis que le firmware 0.3.7 affichait environ **13–15 °C**.
Les températures d'eau mesurées en tasse après purges, de 28,50 à 26,69 °C,
ne sont pas des mesures au contact de la NTC mais confortent l'existence d'un
écart à froid. Captures :
[`143100`](../captures/260924-143100.json),
[`143456`](../captures/260924-143456.json),
[`143921`](../captures/260924-143921.json),
[`145454`](../captures/260924-145454.json).

À température café, les mesures dans le panier après purge donnent **81,6 °C**
pour une consigne de 80 °C ([capture](../captures/260924-162535.json)) et
**86,7 puis 88,4 °C** pour une consigne de 90 °C
([avant mode écoulement](../captures/260924-164051.json),
[après](../captures/260924-165337.json)). Il semble donc possible de faire
un café autour de 90 °C avec le firmware 0.3.7, à quelques degrés près dans
ces conditions ; la température du panier n'est pas identique à celle de la
NTC. À consigne 65 °C, deux purges ont donné **72,6 puis 69,6 °C** dans le
panier malgré une NTC proche de 65 °C avant chaque purge
([première](../captures/260924-170809.json),
[seconde](../captures/260924-171142.json)). Une purge intermédiaire a eu lieu
et le panier a été vidé deux fois entre ces mesures : sa chaleur résiduelle
n'explique pas seule la baisse. Le groupe et le circuit d'eau ont pu se
refroidir ; ces points ne permettent pas encore de modifier la courbe NTC.

Les valeurs dans le panier ont été relevées **après purge**, avec l'ancien
firmware proche de la courbe Gicar : elles ne mesurent pas simultanément la
température de l'eau stagnante à la NTC et celle de l'eau en écoulement au
groupe. Elles ne permettent pas d'attribuer l'écart estimé entre chaudière et
panier au seul groupe, ni de conclure que Gicar a calibré son affichage pour
la température d'infusion. Aucune vapeur n'a été constatée à 90 °C affichés
par Gicar ; cela contraint la température de l'eau **à la sortie à l'air
libre**, sans exclure à lui seul une eau plus chaude sous pression dans la
chaudière. De la vapeur était en revanche sortie pendant la purge avec
l'offset de −18 °C essayé en 0.3.5.

Les relevés longs des purges à 90 °C sont
[`163749`](../captures/monitor-heating-20260924-163749-565936.json) et
[`165225`](../captures/monitor-heating-20260924-165225-542907.json) ; les
relevés à 65 °C sont
[`170713`](../captures/monitor-heating-20260924-170713-740281.json) et
[`171044`](../captures/monitor-heating-20260924-171044-990745.json).
Ces fichiers `captures/` sont locaux et ignorés par Git.

### Comparaison directe de la résistance ADS1115 et du multimètre

À froid, le montage initial alimentait le pont NTC par un LDO AMS1117 et
retournait sa résistance fixe au Wago GND de l'alimentation 5 V. Le GND de
l'ADS1115 se trouvait **10–13 mV** au-dessus de ce Wago : pour une même entrée
A1, le multimètre lisait environ 0,143 V par rapport au GND de l'alimentation,
contre environ 0,133 V par rapport au GND de l'ADS. La résistance NTC débranchée
était proche de 50 kΩ, tandis que les codes ADC conduisaient à plus de 52 kΩ.
Le pont utilise désormais **3V3 et GND du port Sensor AD du Waveshare** ; le
LDO a été retiré. La première capture après correction (`t5.json`, locale)
donne `A0_raw = 26305` et `A1_raw = 1151`, soit **3,288125 V** et
**0,143875 V**, et **47,926 kΩ** calculés. La NTC débranchée immédiatement
après mesurait **47,9 kΩ** au multimètre : écart d'environ **26 Ω (0,05 %)**.
La lecture de résistance est ainsi validée à ce point froid ; la courbe
résistance/température reste à vérifier à chaud. Le câblage corrigé et le
diagnostic détaillé figurent dans [la calibration NTC](ntc_ads1115_calibration.md).

Pour la validation à chaud, après stabilisation à 90 °C avec le firmware
actuel (`47 kΩ / 3 950 K`), sauvegarder plusieurs réponses `GET /telemetry`
**avant extinction**, sans purge immédiatement préalable. Vérifier
`boiler_temperature_valid` et relever
ensemble `boiler_temperature_c`, `boiler_ntc_a0_raw` et
`boiler_ntc_a1_raw`. Les deux derniers champs sont des **codes ADS1115**,
pas des tensions en volts. Calculer, pour chaque paire de codes,
`R_NTC = 2193 × (A0_raw/A1_raw − 1)` en ohms ; la cible de 90 °C du firmware
actuel correspond à environ **4,38 kΩ**, pas à 5 kΩ. Éteindre ensuite la
machine, débrancher la sonde et mesurer rapidement sa résistance au
multimètre. Une légère hausse est attendue avec le refroidissement. La
comparaison vérifie la chaîne de lecture ADC et le calcul de résistance ;
elle ne valide pas encore la conversion résistance/température.

Si l'ancienne courbe proche de Gicar est réinstallée et stabilisée elle aussi
à 90 °C dans les mêmes conditions, elle vise environ **2,91 kΩ**. Refaire
alors les deux relevés permettrait de confirmer que les deux réglages
stabilisent la chaudière à des résistances réellement différentes. Pour
étudier l'écart jusqu'au panier, relever séparément la température du groupe
et du porte-filtre après plusieurs minutes de chauffe au repos, puis mesurer
la température de l'eau **pendant l'écoulement** avec un dispositif adapté
au débit d'une infusion ; la température du métal au repos ne remplace pas
cette dernière mesure.

### Vérification à froid après remontage

La sonde démontée est maintenant caractérisée près de 25 °C. Après son
remontage, laisser `heating.enabled=false` et la machine au repos toute la nuit. Au
redémarrage, vérifier que la chauffe est toujours désactivée. Avant toute
purge, relever la température ambiante près de la chaudière, celle de l'eau
du réservoir avec le même thermomètre digital utilisé dans le panier, et
sauvegarder `GET /telemetry`, notamment `boiler_temperature_c`,
`boiler_ntc_a0_raw`, `boiler_ntc_a1_raw`, `xdb401_temperature_c`,
`heating_enabled` et l'état de la chauffe. L'eau du réservoir peut différer
de l'air ambiant. La mesure IR sur métal nu est sensible à l'émissivité ; la
température de la pièce et l'équilibre nocturne sont les meilleurs repères
pour la NTC installée.

Faire ensuite **deux purges identiques de 8 s**, avec le même panier et le
thermomètre digital immédiatement après chaque purge, en vidant le panier
entre les deux. Noter la température de l'eau du réservoir, la température
dans le panier, la durée, le volume et toute différence de procédure.
`firmware/tools/purge.py 8` lit la télémétrie avant et après mais n'enregistre
que les valeurs affichées dans le terminal ; sauvegarder séparément la
télémétrie brute avant la première purge et les captures haute fréquence :

```sh
curl -fsS -H "Authorization: Bearer ${COFFEEFLOW_HTTP_TOKEN}" \
  "http://${COFFEEFLOW_IP}/telemetry" > captures/cold-before.json
uv run firmware/tools/purge.py 8
uv run firmware/tools/download_hf_capture.py
```

Répéter les deux dernières commandes pour la seconde purge et renommer les
captures si nécessaire. `record_heating.py --mode monitor` requiert
`heating.enabled=true` et ne convient donc pas à cet essai. Le calcul de
résistance à partir de la télémétrie est
`R_NTC = 2193 × (A0/A1 − 1)` en ohms ; comparer ce résultat à la mesure
directe antérieure de 42,1 kΩ. Avec la courbe compilée depuis 0.3.9, cette
résistance donne environ 27,5 °C ; comparer à la température stabilisée de la
machine, puis contrôler la conversion à chaud et la coupure de chauffe.
[Explorateur interactif des courbes](ntc_curve_explorer.html) :
il montre la sensibilité de chaque paramètre sans modifier le firmware.

Pour mesurer une montée depuis l'ambiante :

Le script `firmware/tools/record_heating.py` automatise les étapes 3 et 4,
coupe ensuite la chauffe et écrit un JSON dans `captures/`. Il lit
`COFFEEFLOW_HTTP_TOKEN` et `COFFEEFLOW_IP` ; `--output` permet de choisir
le fichier.

1. Envoyer `POST /config` avec
   `{"version":6,"heating":{"enabled":false}}`, puis laisser refroidir.
2. Allumer la machine et activer le Wi-Fi. La valeur `false` reste persistée.
3. Envoyer `POST /config` avec
   `{"version":6,"heating":{"enabled":true}}`.
4. Interroger `GET /telemetry` toutes les 500 ms. Enregistrer `uptime_ms`,
   `boiler_temperature_c`, sa validité et son âge, `heating_power_pct`,
   `heating_power_accepted_pct`, `heater_on` et
   `brew_temperature_target_c`. Le client doit horodater lui aussi chaque
   réponse. L'écho de puissance et `heater_on` viennent du module capteurs ;
   une absence d'écho frais doit apparaître comme telle dans l'analyse.

Un essai de chauffe et un essai d'infusion sont nécessaires avant de considérer
les gains calibrés. Le mode vapeur reste à ajouter : il devra sélectionner une
cible propre, bloquer l'infusion et garder la purge disponible.
