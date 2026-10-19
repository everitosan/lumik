# Spec Delta

## MODIFIED Requirements

### Requirement: Fecha de sesión inferida
Si el proyecto no tiene fecha de sesión y el import registró al menos una foto, el sistema SHALL tomar como fecha de sesión la fecha de captura más antigua y mover la carpeta del proyecto a la ruta de esa fecha. Una fecha de sesión fijada al editar el proyecto no SHALL sustituirse. Si mover la carpeta falla, el import SHALL considerarse exitoso igualmente.

#### Scenario: Proyecto sin fecha
- **WHEN** se importan fotos de 2023-11-04 a un proyecto creado hoy sin fecha de sesión
- **THEN** el proyecto pasa a tener fecha de sesión `2023-11-04` y su carpeta se mueve a `lumik/2023/11/04_{nombre}`

#### Scenario: Proyecto con fecha
- **WHEN** el proyecto ya tiene fecha de sesión
- **THEN** ni la fecha ni la carpeta cambian

#### Scenario: Fecha fijada al editar
- **WHEN** el usuario fijó la fecha `2023-12-01` al editar el proyecto y después importa fotos capturadas el 2023-11-04
- **THEN** la fecha de sesión sigue siendo `2023-12-01`
