// Copier ce fichier vers secrets.h et remplacer les deux valeurs. secrets.h
// est ignoré par Git et l'image refuse de compiler sans lui.
#pragma once
namespace secrets {
inline constexpr char kProvisioningApPassword[] = "CHANGE_ME_8_CHARS_MIN";
inline constexpr char kHttpBearerToken[] = "CHANGE_ME";
}  // namespace secrets
