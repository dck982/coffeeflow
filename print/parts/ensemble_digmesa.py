from nurb import *
from system import digmesa_layout
from math import sin, cos, pi


@assembly
def ensemble_digmesa(centre_x=52.0, centre_y=29.0, eclate=0.0, contexte=True,
                     guide_tube=False, hauteur_pieds=5.0,
                     surelevation_berceau=17.0, jeu_tube=2.5, distance_trou=25.0):
    """Montage bas ; les obstacles représentent uniquement les cotes connues.

    centre_x: Déplacement latéral du capteur, les M6 restent fixes.
    centre_y: Déplacement vers l'avant du capteur, les M6 restent fixes.
    eclate: Séparation verticale des pièces pour lire le montage.
    contexte: Afficher le capteur simplifié et les limites connues de la machine.
    guide_tube: Afficher le guide optionnel, position à vérifier face à la pompe.
    jeu_tube: Jeu diamétral du guide pour les deux segments.
    distance_trou: Distance du guide au trou.
    hauteur_pieds: Vide sous la structure, hors des ancrages M6.
    surelevation_berceau: Hauteur ajoutée dans le berceau, sans changer le support.
    """
    # Les wrappers de sliders nurb ne sont pas sérialisables dans use().
    centre_x,centre_y,hauteur_pieds,surelevation_berceau,jeu_tube=map(
        float,(centre_x,centre_y,hauteur_pieds,surelevation_berceau,jeu_tube))
    d=digmesa_layout(centre_x,centre_y,hauteur_pieds,surelevation_berceau)
    support=Pos(0,d['arriere']+3,0)*Rot(-90,0,0)*use('support_digmesa',hauteur_pieds=hauteur_pieds,centre_x=centre_x,centre_y=centre_y)
    berceau=Pos(centre_x,centre_y,d['berceau_z']+eclate)*use(
        'berceau_digmesa',surelevation_berceau=surelevation_berceau)
    cap=Pos(0,d['arriere'],2*eclate)*Rot(-90,0,0)*use(
        'chapeau_digmesa',hauteur_pieds=hauteur_pieds,
        surelevation_berceau=surelevation_berceau,
        centre_x=centre_x,centre_y=centre_y)
    solids=[support,berceau,cap]
    gx,gy=153.0,29.0
    # Le bas de la sortie était mesuré à Z=23 pour hauteur_pieds=5. Monter le
    # support de 12…17 mm place donc sortie et guide ensemble à Z=35…40.
    gz=measured('digmesa_tube_boucle_z')
    if guide_tube:
        guide=Pos(gx-10,gy-6,0)*Rot(0,-90,0)*Rot(180,0,0)*Pos(0,0,-distance_trou)*use('guide_boucle_digmesa',jeu_tube=jeu_tube,distance_trou=distance_trou)
        solids.append(guide)
    if contexte:
        a=(Align.CENTER,Align.CENTER,Align.MIN)
        z=d['assise']+eclate
        sensor=Cylinder(16,12.5,align=a)+Pos(0,0,12.5)*Cylinder(20.5,7.5,align=a)
        # Pin central étagé selon le dessin constructeur : Ø4,25 sur les
        # 4,6 mm inférieurs, puis Ø3,85 sur 2,5 mm jusqu'au corps.
        pin_total=measured('digmesa_pin_longueur')
        pin_high=measured('digmesa_pin_hauteur_haut')
        sensor+=Pos(0,0,-pin_total)*Cylinder(
            measured('digmesa_pin_diametre_max')/2,
            pin_total-pin_high,align=a)
        sensor+=Pos(0,0,-pin_high)*Cylinder(
            measured('digmesa_pin_diametre_haut')/2,pin_high,align=a)
        sensor+=Pos(0,-12,-5.1)*Cylinder(1.425,5.1,align=a)
        # Connectique : enveloppe illustrative, seule la hauteur 55 est mesurée.
        sensor+=Pos(0,0,20)*Box(12,12,35,align=a)
        solids.append(obstacle(Pos(centre_x,centre_y,z)*sensor,'Digmesa simplifié ; connectique indicative'))
        for xx,yy in ((23,58),(63,58)):
            nut=extrude(RegularPolygon(6,6),amount=6)-Cylinder(3,7,align=a)
            solids.append(obstacle(Pos(xx,yy,2)*nut,'enveloppe écrou M6, à vérifier avec rondelle'))
        for xx in d['vis_x']:
            head=Pos(xx,d['arriere']-measured('vis_tete_hauteur'),d['vis_z']+2*eclate)*Rot(-90,0,0)*Cylinder(measured('vis_tete')/2,measured('vis_tete_hauteur'),align=a)
            solids.append(obstacle(head,'tête M3 ISO 4762, Ø5,5 × 3 ; garde arrière 2 mm au défaut'))
        if guide_tube:
            pitch=measured('digmesa_tube_exterieur')+jeu_tube
            radius=measured('digmesa_boucle_diametre_min')/2
            # Centre réel du passage après remise du guide dans son orientation
            # de montage : (121,17) au défaut, pour le trou M6 (143,23).
            eye_x=(gx-10)-distance_trou+3
            # +0,5 mm reste dans le jeu de l'ovale et dégage la paroi arrière
            # du chapeau de 0,25 mm pour le premier segment Ø8.
            eye_y=(gy-6)-6+0.5
            points=[(eye_x+radius*sin(t),eye_y-pitch/2+pitch*t/(2*pi),gz+radius*(1-cos(t))) for t in [2*pi*i/96 for i in range(97)]]
            start=(centre_x+24,eye_y-pitch/2,gz)
            end=(eye_x+18,eye_y+pitch/2,gz)
            path=Wire([Line(start,points[0]),Spline(*points),Line(points[-1],end)])
            boucle=sweep(Plane(origin=start,x_dir=(0,1,0),z_dir=(1,0,0))*Circle(4),path=path)
            solids.append(obstacle(boucle,'tube de référence : 360°, deux segments libres ; trajets vers embouts à valider'))
            nut=extrude(RegularPolygon(6,6),amount=6)-Cylinder(3,7,align=a)
            solids.append(obstacle(Pos(gx-10,gy-6,2)*nut,f'écrou M6 guide indépendant ; trou proposé ({gx-10:g},{gy-6:g}) au défaut'))
        # Fragment de fond : épaisseur illustrative, seuls Z=0 et la trame sont connus.
        floor=Pos(0,0,-1)*Box(160,85,1,align=(Align.MIN,Align.MIN,Align.MIN))
        for xx in (23,63,103,143):
            for yy in (23,58):
                floor-=Pos(xx,yy,-2)*Cylinder(3,3,align=a)
        solids.append(obstacle(floor,'fond Z=0 et trous Ø6 sur trame 40×35 ; épaisseur illustrative'))
        solids.append(obstacle(Pos(-2,0,0)*Box(2,85,102,align=(Align.MIN,Align.MIN,Align.MIN)),'face gauche X=0'))
        solids.append(obstacle(Pos(0,-2,0)*Box(100,2,102,align=(Align.MIN,Align.MIN,Align.MIN)),'arrière Y=0'))
        solids.append(obstacle(Pos(0,0,102)*Box(160,85,2,align=(Align.MIN,Align.MIN,Align.MIN)),'plan plaque réservoir Z=102, fragment de contexte'))
        # Presse-étoupe 230 V : X et Z mesurés. Son implantation Y reste
        # illustrative faute de cote ; le volume est placé contre l'avant du
        # fragment de fond pour rendre son emprise visible dans l'assemblage.
        gland_width=measured('presse_etoupe_230v_largeur')
        gland_radius=gland_width/2
        gland_y=measured('presse_etoupe_230v_y_min')
        gland_z=measured('presse_etoupe_230v_z_max')-gland_radius
        gland_plane=Plane(
            origin=(measured('presse_etoupe_230v_centre_x'),gland_y,gland_z),
            x_dir=(1,0,0),z_dir=(0,1,0),
        )
        gland=gland_plane*extrude(Circle(gland_radius),amount=measured('presse_etoupe_230v_profondeur'))
        cable_plane=Plane(
            origin=(measured('presse_etoupe_230v_centre_x'),
                    gland_y+measured('presse_etoupe_230v_profondeur'),gland_z),
            x_dir=(1,0,0),z_dir=(0,1,0),
        )
        gland+=cable_plane*extrude(
            Circle(measured('cable_230v_diametre')/2),
            amount=measured('cable_230v_longueur_contexte'),
        )
        solids.append(obstacle(gland,'presse-étoupe et câble 230 V : X/Z mesurés ; implantation Y indicative'))
        # Connecteur sous la plaque réservoir. Les positions indiquées par
        # l'utilisateur définissent son enveloppe confirmée X=55…80, Y=50…70.
        connector_width=measured('connecteur_reservoir_largeur_x')
        connector_depth=measured('connecteur_reservoir_profondeur_y')
        connector_drop=measured('connecteur_reservoir_descente_z')
        connector=Pos(
            measured('connecteur_reservoir_x_min'),
            measured('connecteur_reservoir_y_max')-connector_depth,
            measured('digmesa_plaque_reservoir_z')-connector_drop,
        )*Box(
            connector_width,connector_depth,connector_drop,
            align=(Align.MIN,Align.MIN,Align.MIN),
        )
        solids.append(obstacle(connector,'connecteur réservoir : X=55…80, Y=50…70, enveloppe approximative'))
        # Hauteur non mesurée : exclusion conservatrice jusqu'à la plaque.
        solids.append(obstacle(Pos(0,35,0)*Box(19,40,102,align=(Align.MIN,Align.MIN,Align.MIN)),'pied : emprise connue, hauteur volontairement majorée'))
    return tuple(solids)
