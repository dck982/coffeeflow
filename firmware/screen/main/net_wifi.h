// Wi-Fi modal : inactif en mode machine. start() charge AP/STA et HTTP ;
// stop() rend le driver et les netifs à la SRAM avant le retour à NimBLE.
#pragma once
namespace net_wifi {
void start();
void stop();
}
