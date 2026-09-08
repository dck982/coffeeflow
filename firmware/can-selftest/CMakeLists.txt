# Auto-test transceiver CAN, isolé du câble — voir la discussion de bring-up
# phase 2 dans docs/firmware-implementation.md. Émet en TWAI_MODE_NO_ACK
# (pas de flag SELF : la boucle passe par le vrai transceiver, pas par un
# rebouclage interne du contrôleur) et vérifie que la trame revient par
# CANH/CANL. Câble CAN débranché pendant ce test : voir le commentaire en
# tête de main.cpp pour le choix des GPIO selon la carte.

cmake_minimum_required(VERSION 3.16)

include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(can-selftest)
