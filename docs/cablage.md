Voici le résumé synthétique de l'architecture retenue pour votre installation.

---

### Synthèse du câblage 230V et Compatibilité Électromagnétique (CEM)

Pour éliminer le bruit électromagnétique (EMI) généré par le gradateur RobotDyn et la vanne inductive, la règle d'or est d'**annuler la surface de boucle** en acheminant le courant aller et retour dans des paires de conducteurs strictement contiguës.

Le câblage de l'alimentation générale, de la vanne et de la pompe se fait via des paires dédiées reliées au Boîtier 2. Deux options viables ont été retenues :

#### 1. Option 6 conducteurs (3 paires) — Mode Manuel via ESP32

Le bouton manuel de la façade n'interrompt plus directement le 230V. Il est raccordé en basse tension (3.3V/5V) sur une entrée GPIO de l'ESP32, qui pilote ensuite la vanne via le relais SSR.

* **Composition :**
* **Paire 1 ($L+N$) :** Alimentation générale (depuis la machine vers le Boîtier 2)
* **Paire 2 ($L+N$) :** Pompe à vibration OLAB (depuis le RobotDyn du Boîtier 2)
* **Paire 3 ($L+N$) :** Vanne solénoïde OLAB (depuis le SSR du Boîtier 2)


* **Avantages :** Gain de place (3 câbles 230V), câblage ultra-simple, aucune puissance sur le bouton de façade.
* **Limite :** Dépendance totale à l'ESP32 (si l'ESP32 plante, le bouton ne fonctionne plus).

#### 2. Option 8 conducteurs (4 paires) — Mode Manuel 100% Matériel (Autonome)

Le point de dérivation de la Phase du bouton est déporté à l'intérieur du Boîtier 2. Le bouton manuel conserve son pouvoir de coupure 230V en parallèle du SSR, sans créer de boucle de neutre.

* **Composition :**
* **Paires 1, 2 et 3 :** Identiques à l'option 6 conducteurs (Alim, Pompe, Vanne).
* **Paire 4 ($L_{\text{aller}} + L_{\text{retour}}$) :** Deux fils de Phase reliés au bouton manuel (Aller du 230V permanent vers le bouton, Retour du 230V commuté vers la vanne dans le Boîtier 2).


* **Avantages :** Secours 100% matériel (le bouton fonctionne même si l'ESP32 est éteint ou planté) tout en conservant une surface de boucle nulle.
* **Limite :** Encombrement physique plus important (4 câbles 230V à passer).

---

### Choix du câble 230V

* **Référence retenue :** **LAPP ÖLFLEX® HEAT 180 SiHF 2x0.75 mm²** (EAN 2050000411601).
* **Section :** $0,75\text{ mm}^2$ (amplement suffisant pour la consommation totale inférieure à 50W / 0,2A).
* **Propriétés :**
* Isolation et gaine 100% silicone résistant de **-50 °C à +180 °C** (adapté à l'enceinte chaude de la machine à café).
* Les deux conducteurs sont maintenus côte à côte par la gaine extérieure : **pas besoin de torsader les fils**, la surface de boucle est déjà minimale.

Sortie du boitier: presse-étoupe M12 x 1.5 pour câble 6.40mm de diamètre. Trou de 12.5mm ou PG7.

---

### Signal de commande 5V (RobotDyn $\leftrightarrow$ ESP32)

* **Câble retenue :** **LiYCY 4x0.25 mm² blindé** pour véhiculer `GND`, `3.3V/5V`, `ZC` (Zero-Cross) et `DIM`.
* **Règle de blindage :** Tresse métallisée (queue de cochon sous gaine thermo) raccordée à la masse **$GND$ uniquement du côté ESP32** (laisser la tresse coupée et isolée flottante du côté RobotDyn pour éviter toute boucle de masse).


