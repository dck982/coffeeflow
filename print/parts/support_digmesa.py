from nurb import *
from system import digmesa_layout


@part
def support_digmesa(centre_x=52.0, centre_y=29.0, jeu_berceau=0.5,
                    hauteur_pieds=5.0, draft=False):
    """Support sur deux pieds M6 et un appui libre, imprimé sur sa tranche arrière.

    centre_x: Centre du capteur depuis la face gauche, vue arrière.
    centre_y: Centre du capteur depuis le rebord arrière.
    jeu_berceau: Jeu diamétral du logement recevant le berceau.
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
        post-=Pos(x,back-1,d['vis_z'])*Rot(-90,0,0)*Cylinder(1.3,9,align=a)
        body+=post
    body -= Pos(centre_x,centre_y,d['berceau_z'])*extrude(drop(rayon),amount=20)
    if not draft:
        edges=body.edges().filter_by(lambda e: abs(e.bounding_box().min.Z-(h+16))<1e-6 and e.bounding_box().min.Y>back+0.01)
        body=polish(body,edges,1.0)
    # Original Y=back sur le plateau ; Z machine devient -Y d'impression.
    return Pos(0,0,-back)*Rot(90,0,0)*body
