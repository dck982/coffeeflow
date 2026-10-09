#include "events.h"

#include "esp_timer.h"
#include "freertos/FreeRTOS.h"

namespace core::events {

namespace {

constexpr size_t kCapacity = 8;

Event g_ring[kCapacity];
size_t g_count = 0;
size_t g_next = 0;  // prochain index à écrire

portMUX_TYPE g_lock = portMUX_INITIALIZER_UNLOCKED;

}  // namespace

void push(EventKind kind) {
  Event e{kind, static_cast<uint32_t>(esp_timer_get_time() / 1000)};
  portENTER_CRITICAL(&g_lock);
  g_ring[g_next] = e;
  g_next = (g_next + 1) % kCapacity;
  if (g_count < kCapacity) {
    g_count++;
  }
  portEXIT_CRITICAL(&g_lock);
}

size_t recent(Event* out, size_t max_count) {
  portENTER_CRITICAL(&g_lock);
  size_t n = g_count < max_count ? g_count : max_count;
  for (size_t i = 0; i < n; ++i) {
    size_t idx = (g_next + kCapacity - 1 - i) % kCapacity;
    out[i] = g_ring[idx];
  }
  portEXIT_CRITICAL(&g_lock);
  return n;
}

const char* to_text(EventKind kind) {
  switch (kind) {
    case EventKind::kBoot:
      return "BOOT";
    case EventKind::kCanPresenceLost:
      return "CAN PERDU";
    case EventKind::kCanPresenceRestored:
      return "CAN OK";
    case EventKind::kWifiApStarted:
      return "AP DEMARRE";
    case EventKind::kWifiConnected:
      return "WIFI CONNECTE";
    case EventKind::kWifiDisconnected:
      return "WIFI PERDU";
    case EventKind::kNetworkForgotten:
      return "RESEAU OUBLIE";
    case EventKind::kTimeKnown:
      return "HEURE CONNUE";
  }
  return "?";
}

}  // namespace core::events
