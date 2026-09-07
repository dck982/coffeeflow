# Projet ESP-IDF — module écran (Waveshare ESP32-S3-Touch-LCD-4.3).
# Voir docs/firmware.md et docs/firmware-implementation.md.

cmake_minimum_required(VERSION 3.16)

set(EXTRA_COMPONENT_DIRS "${CMAKE_CURRENT_SOURCE_DIR}/../common")

include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(screen)
