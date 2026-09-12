from nurb import *
from system import digmesa_layout
from math import sin, cos, pi


@assembly
def ensemble_digmesa(centre_x=52.0, centre_y=29.0, eclate=0.0, contexte=True,
                     guide_tube=False, hauteur_pieds=5.0, jeu_tube=2.5):
    """Montage bas ; les obstacles représentent uniquement les cotes connues.

    centre_x: Déplacement latéral du capteur, les M6 restent fixes.
    centre_y: Déplacement vers l'avant du capteur, les M6 restent fixes.
    eclate: Séparation verticale des pièces pour lire le montage.
    contexte: Afficher le capteur simplifié et les limites connues de la machine.
    guide_tube: Afficher le guide optionnel, position à vérifier face à la pompe.
    jeu_tube: Jeu diamétral du guide pour les deux segments.
    hauteur_pieds: Vide sous la structure, hors des ancrages M6.
    """
    # Les wrappers de sliders nurb ne sont pas sérialisables dans use().
    centre_x,centre_y,hauteur_pieds,jeu_tube=map(float,(centre_x,centre_y,hauteur_pieds,jeu_tube))
    d=digmesa_layout(centre_x,centre_y,hauteur_pieds)
    support=Pos(0,d['arriere']+3,0)*Rot(-90,0,0)*use('support_digmesa',hauteur_pieds=hauteur_pieds,centre_x=centre_x,centre_y=centre_y)
    berceau=Pos(centre_x,centre_y,d['berceau_z']+eclate)*use('berceau_digmesa')
    cap=Pos(0,d['arriere'],2*eclate)*Rot(-90,0,0)*use('chapeau_digmesa',hauteur_pieds=hauteur_pieds,centre_x=centre_x,centre_y=centre_y)
    solids=[support,berceau,cap]
    gx,gy=113.0,29.0
    gz=d['assise']+20
    if guide_tube:
        guide=Pos(gx-10,gy-6,0)*Rot(0,-90,0)*Pos(0,0,-13)*use('guide_boucle_digmesa',hauteur_passage=gz,jeu_tube=jeu_tube)
        solids.append(guide)
    if contexte:
        a=(Align.CENTER,Align.CENTER,Align.MIN)
        z=d['assise']+eclate
        sensor=Cylinder(16,12.5,align=a)+Pos(0,0,12.5)*Cylinder(20.5,7.5,align=a)
        sensor+=Pos(0,0,-7.1)*Cylinder(2.125,7.1,align=a)
        sensor+=Pos(0,-12,-5.1)*Cylinder(1.425,5.1,align=a)
        # Connectique : enveloppe illustrative, seule la hauteur 55 est mesurée.
        sensor+=Pos(0,0,20)*Box(12,12,35,align=a)
        solids.append(obstacle(Pos(centre_x,centre_y,z)*sensor,'Digmesa simplifié ; connectique indicative'))
        for yy in (23,58):
            nut=extrude(RegularPolygon(6,6),amount=6)-Cylinder(3,7,align=a)
            solids.append(obstacle(Pos(23,yy,2)*nut,'enveloppe écrou M6, à vérifier avec rondelle'))
        for xx in d['vis_x']:
            head=Pos(xx,d['arriere']-measured('vis_tete_hauteur'),d['vis_z']+2*eclate)*Rot(-90,0,0)*Cylinder(measured('vis_tete')/2,measured('vis_tete_hauteur'),align=a)
            solids.append(obstacle(head,'tête M3 ISO 4762, Ø5,5 × 3 ; garde arrière 2 mm au défaut'))
        if guide_tube:
            pitch=measured('digmesa_tube_exterieur')+jeu_tube
            radius=measured('digmesa_boucle_diametre_min')/2
            points=[(gx+radius*sin(t),gy-pitch/2+pitch*t/(2*pi),gz+radius*(1-cos(t))) for t in [2*pi*i/96 for i in range(97)]]
            start=(gx-18,gy-pitch/2,gz)
            end=(gx+18,gy+pitch/2,gz)
            path=Wire([Line(start,points[0]),Spline(*points),Line(points[-1],end)])
            boucle=sweep(Plane(origin=start,x_dir=(0,1,0),z_dir=(1,0,0))*Circle(4),path=path)
            solids.append(obstacle(boucle,'tube de référence : 360°, deux segments libres ; trajets vers embouts à valider'))
            nut=extrude(RegularPolygon(6,6),amount=6)-Cylinder(3,7,align=a)
            solids.append(obstacle(Pos(gx-10,gy-6,2)*nut,'écrou M6 guide indépendant ; trou proposé (103,23) au défaut'))
        # Fragment de fond : épaisseur illustrative, seuls Z=0 et la trame sont connus.
        floor=Pos(0,0,-1)*Box(160,85,1,align=(Align.MIN,Align.MIN,Align.MIN))
        for xx in (23,63,103,143):
            for yy in (23,58):
                floor-=Pos(xx,yy,-2)*Cylinder(3,3,align=a)
        solids.append(obstacle(floor,'fond Z=0 et trous Ø6 sur trame 40×35 ; épaisseur illustrative'))
        solids.append(obstacle(Pos(-2,0,0)*Box(2,85,102,align=(Align.MIN,Align.MIN,Align.MIN)),'face gauche X=0'))
        solids.append(obstacle(Pos(0,-2,0)*Box(100,2,102,align=(Align.MIN,Align.MIN,Align.MIN)),'arrière Y=0'))
        solids.append(obstacle(Pos(0,0,102)*Box(160,85,2,align=(Align.MIN,Align.MIN,Align.MIN)),'plan plaque réservoir Z=102, fragment de contexte'))
        # Hauteur non mesurée : exclusion conservatrice jusqu'à la plaque.
        solids.append(obstacle(Pos(0,35,0)*Box(19,40,102,align=(Align.MIN,Align.MIN,Align.MIN)),'pied : emprise connue, hauteur volontairement majorée'))
    return tuple(solids)
