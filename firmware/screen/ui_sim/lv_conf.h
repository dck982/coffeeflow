/* Configuration minimale du rendu hôte : les valeurs non définies prennent
 * les défauts LVGL. Le snapshot est volontairement activé ici seulement. */
#pragma once
#define LV_USE_SNAPSHOT 1
#define LV_USE_DRAW_SW 1
#define LV_COLOR_DEPTH 32
#define LV_USE_OS LV_OS_NONE
#define LV_FONT_MONTSERRAT_14 1
#define LV_USE_FONT_COMPRESSED 1
/* 800×480 RGB888 plus la scène : le pool de 64 ko par défaut ne suffit pas. */
#define LV_MEM_SIZE (2 * 1024 * 1024U)
