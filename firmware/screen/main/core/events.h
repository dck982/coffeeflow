// Flux d'événements du cœur — face transverse de core/core.h
// (docs/plan-phase6.md, "L'idée qui structure tout : le cœur machine").
//
// Lot 2 : juste assez pour porter un flux simple (boot, présence CAN perdue
// et retrouvée), consommé par service_screen.cpp. Les lots suivants
// l'enrichissent (progression de flash, résumé de fin de shot, etc.) sans
// rien changer à cette interface.
#pragma once

#include <cstddef>
#include <cstdint>

namespace core {

enum class EventKind : uint8_t {
  kBoot,
  kCanPresenceLost,
  kCanPresenceRestored,
};

struct Event {
  EventKind kind;
  // Horodatage monotone (esp_timer), en ms depuis le boot — jamais l'heure
  // murale (docs/plan-phase6.md, "tout ce qui mesure une durée utilise
  // l'horloge monotone").
  uint32_t uptime_ms;
};

namespace events {

// Publie un événement. Sûr à appeler depuis n'importe quelle tâche (can_link
// tourne sur le cœur 1, service_screen aussi) : protégé par une section
// critique courte.
void push(EventKind kind);

// Copie au plus `max_count` événements dans `out`, le plus récent en
// premier (out[0] est le dernier publié). Retourne le nombre écrit.
size_t recent(Event* out, size_t max_count);

// Texte court (majuscules, pas de style) pour l'écran de service.
const char* to_text(EventKind kind);

}  // namespace events
}  // namespace core
