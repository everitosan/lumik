# Spec Delta

## MODIFIED Requirements

### Requirement: Índice de fotos en el equipo
Cada equipo SHALL mantener un índice de las fotos de todos los proyectos que ha visto, en su base global. El índice SHALL refrescarse cuando se conecta un disco y cuando cambia un proyecto, y SHALL conservar las fotos de discos desconectados. Las fotos en la papelera y los proyectos en la papelera no SHALL estar en el índice; al restaurarse SHALL volver a él. Lo eliminado definitivamente no SHALL quedar en el índice.

#### Scenario: Disco desconectado
- **WHEN** el fotógrafo desconecta el disco donde está "Boda Martínez"
- **THEN** las fotos de "Boda Martínez" siguen apareciendo en la búsqueda

#### Scenario: Cambio en un proyecto
- **WHEN** el fotógrafo califica una foto con 5 estrellas
- **THEN** una búsqueda `estrellas>=5` la incluye sin reconectar el disco

#### Scenario: Proyecto a la papelera y de vuelta
- **WHEN** el fotógrafo manda a la papelera "Boda Martínez" y después lo restaura
- **THEN** sus fotos dejan de aparecer en la búsqueda mientras está en la papelera y vuelven al restaurarlo
