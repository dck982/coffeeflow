from nurb import *


@part
def guide_boucle_digmesa(jeu_tube=2.5, distance_trou=25.0, draft=False):
    """Guide indépendant, couché sur sa face droite pour l'impression.

    jeu_tube: Jeu sur le diamètre de chacun des deux tubes Ø8.
    distance_trou: distance entre le centre du trou M6 et l'extrémité de la pièce (en Z)
    """
    if not 2.0 <= jeu_tube <= 3.0:
        reject("Jeu de 2 à 3 mm pour laisser les segments libres.",param="jeu_tube")
    east_x = 13.0
    if distance_trou < east_x:
        reject(f"Distance trou au moins {east_x}mm",param="distance_trou")

    plaque_trou_w = 20.0
    north_y = plaque_trou_w/2
    south_y = -plaque_trou_w/2
    
    trou_cx = east_x-distance_trou
    eye_width = measured('digmesa_tube_exterieur')+jeu_tube
    boucle_width= eye_width
    hauteur_boucle = measured('digmesa_tube_boucle_z')
    boucle_cy = measured('digmesa_axe_tube_boucle')
    entree_width=1.0
    eye_dx = 6.0

    a=(Align.CENTER,Align.CENTER,Align.MIN)
    def block(x0,y0,x1,y1,z,h):
        return Pos(x0,y0,z)*Box(x1-x0,y1-y0,h,align=(Align.MIN,Align.MIN,Align.MIN))

    # Plaque de fond    
    body=block(trou_cx-plaque_trou_w/2,south_y,east_x,north_y,0,2)
    
    # Bloc de soutient
    body+=block(east_x-eye_dx,south_y,east_x,north_y,0,8)

    # Oeil de passage
    eye_x0 = east_x - eye_dx
    guide_plane=Plane(origin=(eye_x0,boucle_cy,hauteur_boucle),x_dir=(0,1,0),z_dir=(1,0,0))
    eye=guide_plane*extrude(SlotCenterToCenter(boucle_width,eye_width+6),amount=6)
    pole_dx = 6
    eye+=block(eye_x0,boucle_cy-pole_dx/2,east_x,boucle_cy+pole_dx/2,0,hauteur_boucle-5)
    profile=SlotCenterToCenter(boucle_width,eye_width)
    eye-=Plane(origin=(eye_x0,boucle_cy,hauteur_boucle),x_dir=(0,1,0),z_dir=(1,0,0))*extrude(profile,amount=eye_dx)
    body += eye

    # Axe de vis horizontal pendant l'impression : goutte vers X−.
    r=3.3
    hole=Circle(r)+Polygon((-r/2**0.5,-r/2**0.5),(-r/2**0.5,r/2**0.5),(-r*2**0.5,0),align=None)
    body-=Pos(trou_cx,0,-1)*extrude(hole,amount=4)
    if not draft:
        # Chanfreins extérieurs de l'œillet ; passage du tube et appuis exclus.
        top=hauteur_boucle+boucle_width/2+3
        edges=body.edges().filter_by(lambda e: abs(e.bounding_box().min.Z-top)<1e-6 and e.bounding_box().max.X<east_x-0.01)
        body=polish(body,edges,1.0)
    return Pos(0,0,13)*Rot(0,90,0)*body
