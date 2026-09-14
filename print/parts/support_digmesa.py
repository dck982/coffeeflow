from nurb import *
from system import digmesa_layout, m3_nut_trap


@part
def support_digmesa(centre_x=52.0, centre_y=29.0, jeu_berceau=0.3,
                    berceau_taquet_x=3,
                    hauteur_pieds=5.0, draft=False):
    """Support sur deux pieds M6 et un appui libre, imprimé sur sa tranche arrière.

    centre_x: Centre du capteur depuis la face gauche, vue arrière.
    centre_y: Centre du capteur depuis le rebord arrière.
    jeu_berceau: Jeu diamétral du logement recevant le berceau.
    berceau_taquet_x: Dimension X des taquets
    hauteur_pieds: Vide sous la structure, hors des pieds.
    """
    if not 52.0 <= centre_x <= 60.0:
        reject("Centre X entre 52 et 60 mm pour dégager les écrous M6.", param="centre_x")
    if not 28.0 <= centre_y <= 30.0:
        reject("Centre Y entre 28 et 30 mm pour dégager l'arrière.", param="centre_y")
    if not 0.3 <= jeu_berceau <= 0.8:
        reject("Jeu du berceau entre 0,3 et 0,8 mm.", param="jeu_berceau")
    if not 5.0 <= hauteur_pieds <= 6.0:
        reject("Hauteur des pieds entre 5 et 6 mm pour imprimer la rampe du troisième appui sans porte-à-faux.", param="hauteur_pieds")
    d = digmesa_layout(centre_x, centre_y, hauteur_pieds)
    h = hauteur_pieds
    back = d['arriere']+3
    a = (Align.CENTER, Align.CENTER, Align.MIN)
    def block(x0, y0, x1, y1, z, height):
        return Pos(x0,y0,z)*Box(x1-x0,y1-y0,height,align=(Align.MIN,Align.MIN,Align.MIN))
    # Tranche arrière continue : face d'impression. Rail à droite des écrous.
    body = block(31,back,35,68,h,3)
    body += block(13,back,centre_x+22.5,back+7,h,8)
    def drop(r):
        return Circle(r)+Polygon((r/2**0.5,r/2**0.5),(0,r*2**0.5),(-r/2**0.5,r/2**0.5),align=None)
    # Deux pieds seulement ; rampes à 45° dans le sens d'impression.
    for yy in (23,58):
        x0=13 if yy==23 else 20
        lead=yy-7-h
        pad=block(x0,yy-7,35,yy+7,0,measured('digmesa_semelle_epaisseur'))
        ramp=Polygon((lead,h),(yy-7,0),(yy+7,0),(yy+7,2),(yy-7,2),(lead,h+2),align=None)
        body += Pos(x0,0,0)*extrude(Plane.YZ*ramp,amount=35-x0)
        web=Polygon((lead,h),(yy-7,0),(yy+7,0),(yy+7,h+3),(lead,h+3),align=None)
        body += Pos(31,0,0)*extrude(Plane.YZ*web,amount=4)
        if yy==58:
            flare=Polygon((31,lead-11),(35,lead-11),(35,lead+2),(20,lead+2),(20,lead),align=None)
            body += Pos(0,0,h)*extrude(flare,amount=3)
        body += pad
        body -= Pos(23,yy,-1)*extrude(drop(3.3),amount=4)
    slot=Polygon((18,54.7),(23,54.7),(23+3.3/2**0.5,58+3.3/2**0.5),(18,58+3.3*2**0.5+5),align=None)
    body -= Pos(0,0,-1)*extrude(slot,amount=4)
    rayon = (38.5 + jeu_berceau)/2
    ring = Cylinder(rayon+3,8,align=a) - extrude(drop(14),amount=9)
    ring -= Pos(0,0,6)*extrude(drop(rayon),amount=3)
    ring -= Pos(0,-24,-1)*Box(60,6.6,20,align=a)
    body += Pos(centre_x,centre_y,h)*ring
    # Troisième appui sous le côté libre du berceau : aucune vis, aucun drain bouché.
    # 6×6 mm au sol ; rampe 45° pour l'impression sur la tranche arrière.
    foot=Polygon((centre_y-3-h,h),(centre_y-3,0),(centre_y+3,0),
                 (centre_y+3,h+1),(centre_y-3-h,h+1),align=None)
    body += Pos(centre_x+14,0,0)*extrude(Plane.YZ*foot,amount=6)
    for x in d['vis_x']:
        post=block(x-5,back,x+5,back+7,h,16)
        body+=post
    body -= Pos(centre_x,centre_y,d['berceau_z'])*extrude(drop(rayon),amount=20)
    # Taquets anti-rotation
    # outer radius - inner radius
    outer_radius = (38.5 + jeu_berceau)/2
    ring_width = outer_radius - 14
    if berceau_taquet_x >= ring_width:
        reject(
            f"berceau_taquet_x {berceau_taquet_x} ne peut pas être plus grand que l'anneau {ring_width}",
            param="berceau_taquet_x"
        )
    for bx in (centre_x+outer_radius-berceau_taquet_x, centre_x-outer_radius):
        body += (
            Pos(bx,centre_y,d['berceau_z']) *
            Box(berceau_taquet_x,measured("digmesa_taquet_y"),measured("digmesa_taquet_z"),align=(Align.MIN,Align.CENTER,Align.MIN))
        )
    for x in d['vis_x']:
        # À l'impression, l'ouverture est au sommet du plot (Z imprimé max),
        # jamais contre le plateau. Ce cutter est appliqué au corps déjà uni :
        # il retire donc aussi la matière de base autour du plot, et laisse un
        # vrai hexagone. La masse sous l'écrou porte directement l'épaulement :
        # il n'y a donc pas de plafond en surplomb à ponter. Au montage les vis
        # avancent vers Y+ et atteignent l'écrou par dessous. Ø3,4 ; 5,7 mm sur
        # plats ; épaulement 2,6 mm.
        body-=Pos(x,back+7,d['vis_z'])*Rot(90,0,0)*m3_nut_trap(
            shaft_dia=3.4, nut_af=5.5, nut_th=2.4, shoulder_z=2.6, depth=9.0,
            bridge_roof=False
        )
    # Pas de chanfrein sur cette face : il déformerait l'entrée des pièges M3.
    # Original Y=back sur le plateau ; Z machine devient -Y d'impression.
    return Pos(0,0,-back)*Rot(90,0,0)*body
