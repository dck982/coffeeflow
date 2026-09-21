from nurb import *

from system import AMIN

@part
def goujon_pid(draft=False):
    goujon_w = measured("boitier_pid_goujon_w")
    goujon_z = measured("goujon_pid_z")
    body = Box(goujon_w, goujon_w, goujon_z,align=AMIN)
    return body
