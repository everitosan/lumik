# Spec Delta

## ADDED Requirements

### Requirement: Filtro de papelera
La barra de filtros SHALL ofrecer un filtro "Papelera" que muestra solo las fotos del proyecto que están en la papelera, y SHALL indicar cuántas hay. Sin el filtro, las fotos en la papelera no SHALL mostrarse. El filtro no SHALL guardarse con las preferencias de vista: al abrir un proyecto la vista SHALL empezar sin él.

#### Scenario: Ver la papelera
- **WHEN** el usuario activa el filtro "Papelera" en un proyecto con 3 fotos en la papelera
- **THEN** el grid muestra solo esas 3 fotos

#### Scenario: Reabrir el proyecto
- **WHEN** el usuario cierra el proyecto con el filtro "Papelera" activo y lo vuelve a abrir
- **THEN** el grid muestra las fotos que no están en la papelera

## MODIFIED Requirements

### Requirement: Filtros combinables
Los filtros de aprobadas, estrellas, tags, color y papelera SHALL combinarse entre sí: una foto se muestra solo si cumple todos los filtros activos.

#### Scenario: Estrellas y tag
- **WHEN** están activos "al menos 4 estrellas" y el tag `perfil`
- **THEN** solo se muestran las fotos con 4 o más estrellas que tienen el tag `perfil`

#### Scenario: Papelera y color
- **WHEN** están activos "Papelera" y el color rojo
- **THEN** solo se muestran las fotos en la papelera con etiqueta roja
