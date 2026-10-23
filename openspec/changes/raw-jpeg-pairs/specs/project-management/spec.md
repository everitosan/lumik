# Spec Delta

## MODIFIED Requirements

### Requirement: Editar proyecto
El usuario SHALL poder editar el nombre, la fecha de sesión, la descripción y la opción de entrega al aprobar de un proyecto. El nombre y la fecha SHALL ser obligatorios. Al guardar se SHALL actualizar el proyecto y, si cambian el nombre o la fecha, mover su carpeta a `{montaje}/lumik/{año}/{mes}/{día}_{slug}`; sus fotos SHALL seguir abriéndose desde la nueva carpeta. Si ya existe una carpeta con la ruta destino, la edición SHALL fallar sin cambios.

#### Scenario: Renombrado correcto
- **WHEN** el proyecto en `lumik/2024/06/15_Boda` se renombra a "Boda Ana"
- **THEN** la carpeta pasa a `lumik/2024/06/15_Boda Ana` y las fotos siguen abriéndose

#### Scenario: Cambiar la fecha
- **WHEN** el proyecto en `lumik/2024/06/15_Boda` cambia su fecha de sesión a `2024-09-02`
- **THEN** la carpeta pasa a `lumik/2024/09/02_Boda` y los nombres de las fotos no cambian

#### Scenario: Editar la descripción
- **WHEN** el usuario cambia solo la descripción
- **THEN** se guarda la descripción nueva y la carpeta no se mueve

#### Scenario: Nombre vacío
- **WHEN** el nuevo nombre está vacío o solo tiene espacios
- **THEN** la edición se rechaza

#### Scenario: Fecha vacía
- **WHEN** el usuario borra la fecha de sesión
- **THEN** la edición se rechaza

#### Scenario: Colisión de carpeta
- **WHEN** ya existe una carpeta con la ruta destino
- **THEN** la edición falla y el proyecto permanece en su ubicación original y con sus datos anteriores

#### Scenario: Cambiar la entrega al aprobar
- **WHEN** el usuario cambia la entrega del proyecto a *todo JPG de cámara* y guarda
- **THEN** el proyecto guarda ese valor, la carpeta no se mueve, y las siguientes aprobaciones lo usan
