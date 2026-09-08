# Projet ESP-IDF — adaptateur CAN de secours (M5Stack Atom S3 + Unit CAN).
# Indépendant de sensors/ et screen/. Voir docs/firmware-implementation.md,
# phase 2, section "firmware/can-monitor".

cmake_minimum_required(VERSION 3.16)

set(EXTRA_COMPONENT_DIRS "${CMAKE_CURRENT_SOURCE_DIR}/../common")

include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(can-monitor)
