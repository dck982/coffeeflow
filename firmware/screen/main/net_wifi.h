// Wi-Fi lot 4 : AP iff credentials absents, STA sinon. Aucun repli AP après
// une panne d'un réseau déjà configuré : forget_network() est explicite.
#pragma once
namespace net_wifi { void init(); }
