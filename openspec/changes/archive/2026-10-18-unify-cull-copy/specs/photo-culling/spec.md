# Spec Delta

## ADDED Requirements

### Requirement: Vocabulario de aprobación en la interfaz
Todo texto visible que nombre la acción o el estado de culling SHALL usar el vocabulario de aprobación. En español: "aprobar", "quitar aprobación" y "aprobada(s)". En inglés: "approve", "remove approval" y "approved". Ningún control, etiqueta, contador, filtro ni descripción de atajo SHALL usar "descartar", "recover", "revisión" ni "cull"/"culled" para nombrar esa acción o ese estado. "culling" solo SHALL usarse como nombre de la actividad en textos descriptivos.

#### Scenario: Panel del visor en español
- **WHEN** el usuario abre una foto en el visor con la interfaz en español
- **THEN** la sección del panel se titula "Aprobación" y el control dice "Aprobar foto"

#### Scenario: Panel del visor en inglés
- **WHEN** el usuario abre una foto en el visor con la interfaz en inglés
- **THEN** la sección del panel se titula "Approval" y el control dice "Approve photo"

#### Scenario: Footer del proyecto en inglés
- **WHEN** el usuario ve un proyecto con 12 fotos aprobadas y la interfaz en inglés
- **THEN** el footer muestra "12 approved" y el filtro dice "Show approved"

#### Scenario: Atajos en Ajustes
- **WHEN** el usuario abre Ajustes → Atajos en español
- **THEN** el atajo de la barra espaciadora se describe como "Aprobar / quitar aprobación" y el de `Ctrl+c` como "Mostrar solo fotos aprobadas"

#### Scenario: Ningún texto contradictorio
- **WHEN** se revisan los textos de la interfaz en ambos idiomas
- **THEN** ninguno usa "Descartar" ni "Recover" para referirse a la aprobación de una foto
