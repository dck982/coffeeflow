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
    body = block(28,back,35,68,h,3)
    # L'extension vers X− qui rejoignait l'ancien trou (23,23) est retirée :
    # la traverse commence désormais au bord gauche de l'anneau.
    body += block(29.5,back,centre_x+22.5,back+7,h,8)
    # Copie du rail du trou partiel, translatée de +40 mm avec le pied.
    body += block(69,back,76,back+10,h,3)
    body += block(71,back+10,76,68,h,3)
    def drop(r):
        return Circle(r)+Polygon((r/2**0.5,r/2**0.5),(0,r*2**0.5),(-r/2**0.5,r/2**0.5),align=None)
    # Deux ancrages sur Y=58 : le trou partiel existant (23,58), puis la
    # goutte déplacée de 40 mm en X vers (63,58). Rampes à 45° dans le sens
    # d'impression ; l'ancien trou (23,23) est entièrement supprimé.
    for xx,x0,x1,web_x in ((23,20,35,31),(63,56,76,71)):
        yy=58
        lead=yy-7-h
        pad_y1=yy+10
        pad=block(x0,yy-7,x1,pad_y1,0,measured('digmesa_semelle_epaisseur'))
#        ramp_points=[(lead,h),(yy-7,0),(yy+7,0),(yy+7,2),(yy-7,2),(lead,h+2)]
        ramp_points=[(lead,h),(yy-7,0),(yy-7,2+4),(lead,2+4)]
        ramp=Polygon(*ramp_points,align=None)
        body += Pos(x0,0,0)*extrude(Plane.YZ*ramp,amount=x1-x0)
        # Raccord 3D vers le Box du guide d'écrou. La petite section repose sur
        # le rail ; la grande reprend exactement toute l'empreinte du pad.
        # Sur 15 mm en Y, l'expansion maximale est 15 mm en X et 5 mm en Z.
        support_y0=yy-22
        web=Polygon((lead,h),(yy-7,0),(yy+7,0),(yy+7,h+3),(lead,h+3),align=None)
        body += Pos(web_x,0,0)*extrude(Plane.YZ*web,amount=4)
        flare=Polygon((web_x,lead-11),(x1,lead-11),(x1,lead+2),(x0,lead+2),(x0,lead),align=None)
        body += Pos(0,0,h)*extrude(flare,amount=3)
        body += pad
        body -= Pos(xx,yy,-1)*extrude(drop(3.3),amount=4)
        # Plot plein anti-rotation, puis cutter hexagonal avec 0,15 mm de jeu
        # radial. Le relief triangulaire prolonge le cutter vers +Y à 45° :
        # en orientation d'impression, la cavité se referme couche par couche.
        nut_radius=6.0+0.15
        boss=block(x0,yy-7,x1,pad_y1,
                   measured('digmesa_semelle_epaisseur'),4)
        roof_y=nut_radius*(3**0.5)/2
        roof_half=nut_radius/2
        nut_profile=RegularPolygon(nut_radius,6,rotation=0)+Polygon(
            (-roof_half,roof_y),(roof_half,roof_y),(0,roof_y+roof_half),align=None)
        nut_cutter=Pos(xx,yy,measured('digmesa_semelle_epaisseur')-0.1)*extrude(
            nut_profile,amount=4.2)
        if xx==23:
            nut_cutter += block(19,yy-5,23.5,yy+11,1.9,4.3)
        nut_guide=boss -nut_cutter
        body += nut_guide
    slot=Polygon((18,54.7),(23,54.7),(23+3.3/2**0.5,58+3.3/2**0.5),(18,58+3.3*2**0.5+5),align=None)
    body -= Pos(0,0,-1)*extrude(slot,amount=4)
    rayon = (38.5 + jeu_berceau)/2
    ring = Cylinder(rayon+3,8,align=a) - extrude(drop(14),amount=9)
    ring -= Pos(0,0,6)*extrude(drop(rayon),amount=3)
    ring -= Pos(0,-24,-1)*Box(60,6.6,20,align=a)
    body += Pos(centre_x,centre_y,h)*ring
    # Deux appuis libres symétriques sous le berceau : aucune vis, aucun drain
    # bouché. Empreinte 6×6 mm au sol et rampe 45° dans l'orientation d'impression.
    foot=Polygon((centre_y-3-h,h),(centre_y-3,0),(centre_y+3,0),
                 (centre_y+3,h+1),(centre_y-3-h,h+1),align=None)
    for foot_x in (centre_x-20,centre_x+14):
        body += Pos(foot_x,0,0)*extrude(Plane.YZ*foot,amount=6)
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
