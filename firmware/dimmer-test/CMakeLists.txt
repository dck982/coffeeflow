# Firmware jetable, sans CAN ni machine de sécurité — pour itérer vite sur le
# bring-up I2C du dimmer RBDimmer/DimmerLink (voir docs/firmware.md, "Dimmer
# — pompe"), sans les contraintes de présence/bail du firmware sensors réel.
# À flasher temporairement sur le XIAO à la place de sensors, puis reflasher
# sensors une fois le test terminé.

cmake_minimum_required(VERSION 3.16)

include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(dimmer-test)
