// Wi-Fi modal : inactif en mode machine. start() charge AP/STA et HTTP ;
// stop() rend le driver et les netifs à la SRAM avant le retour à NimBLE.
#pragma once
namespace net_wifi {
void start();
void stop();

// Au démarrage, tente une association STA avec les identifiants enregistrés
// pour obtenir l'heure NTP, puis rend complètement la radio au mode machine.
// Aucun AP de provisioning n'est ouvert dans ce chemin.
void start_boot_time_sync();
}
