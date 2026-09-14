from nurb import *
from system import digmesa_layout


@part
def chapeau_digmesa(centre_x=52.0, centre_y=29.0, hauteur_pieds=5.0,
                    surelevation_berceau=17.0, jeu_retenue=0.1,
                    jeu_connecteur=3.0, draft=False):
    """Chapeau imprimé couché sur sa face arrière ; remise en place par l'assemblage.

    centre_x: Même centre X que le support.
    centre_y: Même centre Y que le support.
    hauteur_pieds: Dégagement du support au-dessus du fond.
    surelevation_berceau: Même surélévation que le berceau.
    jeu_retenue: Jeu à ajouter sur la retenue pour qu'elle ne sert pas trop
    jeu_connecteur: Jeu autour de l'enveloppe du connecteur réservoir.
    """
    if jeu_connecteur < 3.0:
        reject("Jeu connecteur d'au moins 3 mm pour éviter le contact sous vibrations.",param="jeu_connecteur")
    RETENUE_LARGEUR=10.0
    d = digmesa_layout(centre_x,centre_y,hauteur_pieds,surelevation_berceau)
    def block(x0,y0,x1,y1,z,h):
        return Pos(x0,y0,z)*Box(x1-x0,y1-y0,h,align=(Align.MIN,Align.MIN,Align.MIN))
    back = d['arriere']    
    #body = block(centre_x-23,back,centre_x+23,back+3,d['berceau_z'],d['toit']+3-d['berceau_z'])
    b2=Polygon(
        (centre_x-23,d['berceau_z']),
        (centre_x+23,d['berceau_z']),
        (centre_x+23,d['vis_z']),
        (centre_x+RETENUE_LARGEUR,d['retenue']+jeu_retenue),
        (centre_x+RETENUE_LARGEUR,d['toit']+3),
        (centre_x-RETENUE_LARGEUR,d['toit']+3),
        (centre_x-RETENUE_LARGEUR,d['retenue']+jeu_retenue),
        (centre_x-23,d['vis_z']),
        align=None)
    body = Pos(0,back+3,0)*extrude(Plane.XZ*b2,amount=3)

    connector_y=measured('connecteur_reservoir_y_max')-measured('connecteur_reservoir_profondeur_y')
    body += block(centre_x-RETENUE_LARGEUR,back,centre_x+RETENUE_LARGEUR,connector_y-jeu_connecteur,d['toit'],3)
    # Retenue au bord arrière de la collerette, 0,5 mm de garde verticale.
    body += block(centre_x-RETENUE_LARGEUR,back+2,centre_x+RETENUE_LARGEUR,centre_y-10,d['retenue']+jeu_retenue,3)
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
