from nurb import *


@part
def guide_boucle_digmesa(hauteur_passage=39.1, jeu_tube=2.5, draft=False):
    """Guide indépendant, couché sur sa face droite pour l'impression.

    hauteur_passage: Axe des deux segments depuis la tôle du fond.
    jeu_tube: Jeu sur le diamètre de chacun des deux tubes Ø8.
    """
    if not 2.0 <= jeu_tube <= 3.0:
        reject("Jeu de 2 à 3 mm pour laisser les segments libres.",param="jeu_tube")
    if not 34.0 <= hauteur_passage <= 42.0:
        reject("Hauteur du passage entre 34 et 42 mm sous la plaque réservoir.",param="hauteur_passage")
    width=measured('digmesa_tube_exterieur')+jeu_tube
    a=(Align.CENTER,Align.CENTER,Align.MIN)
    def block(x0,y0,x1,y1,z,h):
        return Pos(x0,y0,z)*Box(x1-x0,y1-y0,h,align=(Align.MIN,Align.MIN,Align.MIN))
    # Repère montage : fixation (0,0), axe du passage X=10,Y=6.
    guide_plane=Plane(origin=(7,6,hauteur_passage),x_dir=(0,1,0),z_dir=(1,0,0))
    eye=guide_plane*extrude(SlotCenterToCenter(width,width+6),amount=6)
    eye+=block(7,3,13,9,5,hauteur_passage-5)
    profile=SlotCenterToCenter(width,width)
    eye-=Plane(origin=(6,6,hauteur_passage),x_dir=(0,1,0),z_dir=(1,0,0))*extrude(profile,amount=8)
    # Pied mince et colonne déportée hors de l'écrou M6 (rayon enveloppe 6).
    base=block(-10,-10,13,10,0,2)
    base+=block(8,-4,13,10,0,8)
    # Deux dégagements sous la semelle ; restent deux bandes d'appui.
    for yy in (-6,6):
        base-=block(-11,yy-1.5,8,yy+1.5,-1,2)
    # Axe de vis horizontal pendant l'impression : goutte vers X−.
    r=3.3
    hole=Circle(r)+Polygon((-r/2**0.5,-r/2**0.5),(-r/2**0.5,r/2**0.5),(-r*2**0.5,0),align=None)
    base-=Pos(0,0,-1)*extrude(hole,amount=4)
    body=eye+base
    if not draft:
        # Chanfreins extérieurs de l'œillet ; passage du tube et appuis exclus.
        top=hauteur_passage+width/2+3
        edges=body.edges().filter_by(lambda e: abs(e.bounding_box().min.Z-top)<1e-6 and e.bounding_box().max.X<12.99)
        body=polish(body,edges,1.0)
    return Pos(0,0,13)*Rot(0,90,0)*body
