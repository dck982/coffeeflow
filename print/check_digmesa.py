"""Audit géométrique reproductible ; lancer avec le Python de nurb depuis print/."""
from itertools import combinations
from pathlib import Path
from nurb import checks
from nurb import *
from parts.ensemble_digmesa import ensemble_digmesa
from parts.support_digmesa import support_digmesa
from parts.guide_boucle_digmesa import guide_boucle_digmesa


def overlap(left, right):
    cut=left.intersect(right)
    return 0 if cut is None else sum(s.volume for s in cut)


def check_pose(x,y,h,slack):
    assembly=ensemble_digmesa(centre_x=x,centre_y=y,hauteur_pieds=h,
                              jeu_tube=slack,guide_tube=True)
    scene=assembly._nurb_scene
    for i,shape in enumerate(scene.statics):
        assert len(shape.solids()) == 1, ('solids',i)
    for (i,left),(j,right) in combinations(enumerate(scene.statics),2):
        assert overlap(left,right)<0.001, ('parts',i,j,overlap(left,right))
    for i,shape in enumerate(scene.statics):
        for j,obstacle_shape in enumerate(scene.obstacles):
            volume=overlap(shape,obstacle_shape)
            assert volume<0.001, ('obstacle',i,j,volume)
    for name,printed in [
        ('support_digmesa',support_digmesa(centre_x=x,centre_y=y,hauteur_pieds=h)),
        ('guide_boucle_digmesa',guide_boucle_digmesa(hauteur_passage=34.1+h,jeu_tube=slack)),
    ]:
        findings=checks.run(printed,checks.from_card(Path('parts')/(name+'.py')))
        assert not findings, (name,x,y,h,slack,findings)
    support=scene.statics[0]
    a=(Align.CENTER,Align.CENTER,Align.MIN)
    for xx,yy in [(63,23),(63,58),(103,58),(143,23),(143,58)]:
        assert overlap(support,Pos(xx,yy,0)*Cylinder(3,h,align=a))<0.001
    foot_areas=sum(f.area for f in support.faces()
                   if abs(f.bounding_box().min.Z)<1e-6
                   and abs(f.bounding_box().max.Z)<1e-6)
    tube=scene.obstacles[5]  # sensor, 2 M6, 2 M3, then reference tube
    print(f'X={x}, Y={y}, pieds={h}, jeu={slack}: 4 solides, intersections nulles ; '
          f'appui fond {foot_areas:.1f} mm² ; têtes M3 à Y={y-27:.1f}')
    print(f'  boucle : Z max {tube.bounding_box().max.Z:.2f}, '
          f'garde plaque {102-tube.bounding_box().max.Z:.2f} mm')


if __name__ == '__main__':
    for pose in [(52.,29.,5.,2.5),(52.,28.,5.,2.),(60.,30.,5.,3.),(54.,29.,6.,2.5)]:
        check_pose(*pose)
    print('PASS : pièces, capteur simplifié, boucle, visserie simplifiée et contexte connu.')
