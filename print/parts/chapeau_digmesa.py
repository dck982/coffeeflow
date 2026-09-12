from nurb import *
from system import digmesa_layout


@part
def chapeau_digmesa(centre_x=52.0, centre_y=29.0, hauteur_pieds=5.0, draft=False):
    """Chapeau imprimé couché sur sa face arrière ; remise en place par l'assemblage.

    centre_x: Même centre X que le support.
    centre_y: Même centre Y que le support.
    hauteur_pieds: Dégagement du support au-dessus du fond.
    """
    d = digmesa_layout(centre_x,centre_y,hauteur_pieds)
    def block(x0,y0,x1,y1,z,h):
        return Pos(x0,y0,z)*Box(x1-x0,y1-y0,h,align=(Align.MIN,Align.MIN,Align.MIN))
    back = d['arriere']
    body = block(centre_x-23,back,centre_x+23,back+3,d['berceau_z'],d['toit']+3-d['berceau_z'])
    body += block(centre_x-23,back,centre_x+23,centre_y+23,d['toit'],3)
    # Retenue au bord arrière de la collerette, 0,5 mm de garde verticale.
    body += block(centre_x-6,back+2,centre_x+6,centre_y-17,d['retenue'],3)
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
