#pragma once
#include <cstdint>
/* Horloge contrôlée par le scénario de capture. */
extern int64_t g_sim_time_us;
inline int64_t esp_timer_get_time() { return g_sim_time_us; }
