# Projet ESP-IDF — adaptateur CAN de secours (M5Stack Atom S3 + Unit CAN).
# Indépendant de sensors/ et screen/. Voir docs/firmware-implementation.md,
# phase 2, section "firmware/can-monitor".

cmake_minimum_required(VERSION 3.16)

include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(can-monitor)
