# Composant ESP-IDF partagé par sensors/ et screen/ — protocole CAN, CRC,
# table de codes LOG. Voir docs/firmware.md et docs/firmware-implementation.md.

idf_component_register(
  SRCS "src/crc.cpp"
  INCLUDE_DIRS "include" "generated"
)

# Tout ce qui suit doit rester APRÈS idf_component_register() : ce fichier
# est aussi lu en mode "script" (extraction des dépendances, avant
# idf_component_register) et add_custom_command() n'y est pas autorisé.

set(LOG_CODES_YAML "${CMAKE_CURRENT_SOURCE_DIR}/codegen/log_codes.yaml")
set(LOG_CODES_HPP "${CMAKE_CURRENT_SOURCE_DIR}/generated/log_codes.hpp")

# Régénéré à chaque build : la source YAML est la seule chose qu'on édite,
# jamais log_codes.hpp / log_codes.py directement.
add_custom_command(
  OUTPUT "${LOG_CODES_HPP}"
  COMMAND python3 "${CMAKE_CURRENT_SOURCE_DIR}/codegen/gen_log_codes.py"
  DEPENDS "${LOG_CODES_YAML}" "${CMAKE_CURRENT_SOURCE_DIR}/codegen/gen_log_codes.py"
  COMMENT "Génération de log_codes.hpp / log_codes.py depuis log_codes.yaml"
)
add_custom_target(common_generate_log_codes DEPENDS "${LOG_CODES_HPP}")

add_dependencies(${COMPONENT_LIB} common_generate_log_codes)
