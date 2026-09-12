cmake_minimum_required(VERSION 3.20)
project(coffeeflow_ui_snapshot LANGUAGES C CXX)

set(LV_BUILD_CONF_PATH "${CMAKE_CURRENT_SOURCE_DIR}/lv_conf.h" CACHE PATH "" FORCE)
set(CONFIG_LV_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(CONFIG_LV_BUILD_DEMOS OFF CACHE BOOL "" FORCE)
add_subdirectory(../managed_components/lvgl__lvgl lvgl)
target_compile_definitions(lvgl PUBLIC LV_USE_SNAPSHOT=1 LV_USE_DRAW_SW=1)

add_executable(ui_snapshot
  sim_main.cpp
  core_stubs.cpp
  ../main/ui/ui_home.cpp
  ../main/ui/ui_fonts/ui_font_18.c
  ../main/ui/ui_fonts/ui_font_26.c
  ../main/ui/ui_fonts/ui_font_28.c
  ../main/ui/ui_fonts/ui_font_32.c
  ../main/ui/ui_fonts/ui_font_40.c
  ../main/ui/ui_fonts/ui_font_76.c
  ../main/ui/ui_fonts/ui_font_104.c)
target_include_directories(ui_snapshot PRIVATE ../main include ../main/ui ../../common/include)
target_link_libraries(ui_snapshot PRIVATE lvgl)
target_compile_features(ui_snapshot PRIVATE cxx_std_17)
target_compile_definitions(ui_snapshot PRIVATE LV_USE_SNAPSHOT=1 LV_USE_DRAW_SW=1 UI_SIM=1)
