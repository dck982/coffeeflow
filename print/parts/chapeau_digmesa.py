from nurb import *
from system import digmesa_layout


@part
def chapeau_digmesa(centre_x=52.0, centre_y=29.0, hauteur_pieds=5.0,
                    surelevation_berceau=17.0, jeu_retenue=0.1,
                    jeu_connecteur=3.0, largeur_faces=2.0, draft=False):
    """Chapeau imprimé couché sur sa face arrière ; remise en place par l'assemblage.

    centre_x: Même centre X que le support.
    centre_y: Même centre Y que le support.
    hauteur_pieds: Dégagement du support au-dessus du fond.
    surelevation_berceau: Même surélévation que le berceau.
    jeu_retenue: Jeu à ajouter sur la retenue pour qu'elle ne sert pas trop
    jeu_connecteur: Jeu autour de l'enveloppe du connecteur réservoir.
    largeur_faces: Largeur des faces de côté
    """
    if jeu_connecteur < 3.0:
        reject("Jeu connecteur d'au moins 3 mm pour éviter le contact sous vibrations.",param="jeu_connecteur")
    RETENUE_LARGEUR=12.0
    RETENUE_DEPTH=4.0

    d = digmesa_layout(centre_x,centre_y,hauteur_pieds,surelevation_berceau)
    def block(x0,y0,x1,y1,z,h):
        return Pos(x0,y0,z)*Box(x1-x0,y1-y0,h,align=(Align.MIN,Align.MIN,Align.MIN))
    back = d['arriere']    
    #body = block(centre_x-23,back,centre_x+23,back+3,d['berceau_z'],d['toit']+3-d['berceau_z']
    top_x0 = centre_x-(RETENUE_LARGEUR+largeur_faces)
    top_x1 = centre_x+(RETENUE_LARGEUR+largeur_faces)
    bottom_x0 = centre_x-23
    bottom_x1 = centre_x+23
    b2=Polygon(
        (bottom_x0,d['vis_z']-10),
        (bottom_x1,d['vis_z']-10),
        (bottom_x1,d['vis_z']),
        (top_x1,d['retenue']+jeu_retenue),
        (top_x1,d['toit']+3),
        (top_x0,d['toit']+3),
        (top_x0,d['retenue']+jeu_retenue),
        (bottom_x0,d['vis_z']),
        align=None)
    body = Pos(0,back+3,0)*extrude(Plane.XZ*b2,amount=3)

    connector_y=(
        measured('connecteur_reservoir_y_max')-
        measured('connecteur_reservoir_profondeur_y')-
        jeu_connecteur
    )
    body += block(top_x0,back,top_x1,connector_y,d['toit'],3)
    # Retenue au bord arrière de la collerette, 0,5 mm de garde verticale.
    retenue_top = d['retenue']+jeu_retenue+RETENUE_DEPTH
    body += block(
        top_x0,
        back+2,
        top_x1,
        centre_y-10,
        d['retenue']+jeu_retenue,
        RETENUE_DEPTH)

    # Faces
    body += block(top_x0,back,top_x0+largeur_faces,connector_y,retenue_top,d['toit']-retenue_top)
    body += block(top_x1-largeur_faces,back,top_x1,connector_y,retenue_top,d['toit']-retenue_top)

    for x in d['vis_x']:
        body -= Pos(x,back-1,d['vis_z'])*Rot(-90,0,0)*Cylinder(1.7,5,
            align=(Align.CENTER,Align.CENTER,Align.MIN))
    if not draft:
        # Les jonctions concaves et les deux faces de la retenue sont fonctionnelles.
        concaves=concave_edges(body)
        edges=body.edges().filter_by(lambda e:
            not any(e.is_same(c) for c in concaves)
            and e.bounding_box().min.Y > back+0.01
            and e.bounding_box().min.Z > d['toit']-0.01)
        body=polish(body,edges,1.0)
    # Back Y=back devient Z=0 ; toit et butée montent verticalement à l'impression.
    return Pos(0,0,-back)*Rot(90,0,0)*body
