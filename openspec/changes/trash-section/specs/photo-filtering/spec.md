# Spec Delta

## MODIFIED Requirements

### Requirement: Filtros combinables
Los filtros de aprobadas, estrellas, tags y color SHALL combinarse entre sí: una foto se muestra solo si cumple todos los filtros activos.

#### Scenario: Estrellas y tag
- **WHEN** están activos "al menos 4 estrellas" y el tag `perfil`
- **THEN** solo se muestran las fotos con 4 o más estrellas que tienen el tag `perfil`

#### Scenario: Papelera y color
- **WHEN** está activo el color rojo y el proyecto tiene fotos rojas en la papelera
- **THEN** solo se muestran las fotos rojas que no están en la papelera

## REMOVED Requirements

### Requirement: Filtro de papelera
**Reason**: Las fotos en la papelera se ven, restauran y eliminan desde la sección "Papelera" de la app, que reúne las de todos los proyectos de los discos conectados.
**Migration**: Usar la sección "Papelera" del menú izquierdo (desktop) o del rail (tablet). El grid del proyecto nunca muestra fotos en la papelera.
