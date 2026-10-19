# Spec Delta

## ADDED Requirements

### Requirement: Representación visual del estado de flujo
El estado de flujo de cada proyecto en el dashboard SHALL mostrarse como un badge con color, icono y etiqueta propios de cada estado, de modo que el estado sea identificable sin depender solo del color: `imported` en amarillo con un icono de importación y la etiqueta "Importado"; `edited` en azul con un icono de lápiz y la etiqueta "Editado"; `delivered` en gris con un icono de check y la etiqueta "Entregado". Las etiquetas SHALL seguir el idioma de la interfaz.

#### Scenario: Proyecto importado
- **WHEN** un proyecto tiene estado de flujo `imported`
- **THEN** su tarjeta muestra un badge amarillo con el icono de importación y la etiqueta "Importado"

#### Scenario: Proyecto editado
- **WHEN** un proyecto tiene estado de flujo `edited`
- **THEN** su tarjeta muestra un badge azul con el icono de lápiz y la etiqueta "Editado"

#### Scenario: Proyecto entregado
- **WHEN** un proyecto tiene estado de flujo `delivered`
- **THEN** su tarjeta muestra un badge gris con el icono de check y la etiqueta "Entregado"

#### Scenario: Interfaz en inglés
- **WHEN** el idioma de la interfaz es inglés y un proyecto tiene estado `edited`
- **THEN** la etiqueta del badge es "Edited"

#### Scenario: Mismo badge en desktop y portal web
- **WHEN** el mismo proyecto se ve en el dashboard de la app de escritorio y en el del portal web con el mismo estado de flujo
- **THEN** ambos muestran el badge con el mismo color, icono y etiqueta
