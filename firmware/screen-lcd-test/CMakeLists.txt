# Bring-up LCD jetable — isolé du reste du firmware (docs/plan-phase6.md, lot
# 2, section "Piège"). Teste si les timings HSYNC/VSYNC confirmés par
# tests/screen/hello_waveshare/hello_waveshare.ino (Arduino_GFX direct, sans
# glitch) corrigent le glitch horizontal observé dans firmware/screen quand
# LVGL pilote le même panneau RGB avec les timings génériques de l'exemple
# ESP-IDF officiel Waveshare. Projet autonome, indépendant de firmware/common.

cmake_minimum_required(VERSION 3.16)

include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(screen-lcd-test)
