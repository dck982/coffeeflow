#pragma once

namespace boiler_ntc {

// Lance la mesure sur le bus I2C déjà créé par board. Une sonde absente ne
// bloque jamais le démarrage de l'écran, du CAN ou du réseau.
void start();

}  // namespace boiler_ntc
