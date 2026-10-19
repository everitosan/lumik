# Spec Delta

## ADDED Requirements

### Requirement: Editar proyecto
El usuario SHALL poder editar el nombre, la fecha de sesión y la descripción de un proyecto. El nombre y la fecha SHALL ser obligatorios. Al guardar se SHALL actualizar el proyecto y, si cambian el nombre o la fecha, mover su carpeta a `{montaje}/lumik/{año}/{mes}/{día}_{slug}`, actualizando las rutas de todas las fotos. Si ya existe una carpeta con la ruta destino, la edición SHALL fallar sin cambios.

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

### Requirement: Fecha de sesión al editar
El campo de fecha SHALL precargarse con la fecha de sesión del proyecto o, si no tiene, con la fecha de su carpeta. Si el proyecto no tenía fecha de sesión y se guarda con la fecha precargada, SHALL seguir sin fecha de sesión. Cualquier otra fecha SHALL guardarse como fecha de sesión.

#### Scenario: Guardar sin tocar la fecha
- **WHEN** un proyecto sin fecha de sesión creado el 2026-10-19 se edita solo para cambiar el nombre
- **THEN** el proyecto sigue sin fecha de sesión y una importación posterior puede inferirla

#### Scenario: Fecha elegida por el usuario
- **WHEN** un proyecto sin fecha de sesión creado el 2026-10-19 se guarda con la fecha `2026-09-30`
- **THEN** su fecha de sesión pasa a ser `2026-09-30`

### Requirement: Renombrar archivos al editar el nombre
Si cambia el nombre y el usuario lo pide, las fotos de `_media/` y `_culled/` cuyo nombre lleve el nombre de proyecto anterior según la regla del renombrado al importar SHALL renombrarse con el nombre nuevo saneado, junto con sus sidecars. Los demás archivos y `_exported/` no SHALL modificarse. Si un renombrado falla, los ya hechos SHALL deshacerse.

#### Scenario: Renombrar archivos
- **WHEN** el proyecto "Boda" se renombra a "Boda Ana" con la opción de renombrar archivos
- **THEN** `2024-06-15__14-30-22__Boda.cr3` pasa a `2024-06-15__14-30-22__Boda_Ana.cr3` y `2024-06-15__14-30-22__Boda-001.cr3` pasa a `2024-06-15__14-30-22__Boda_Ana-001.cr3`

#### Scenario: Archivos no renombrados al importar
- **WHEN** el proyecto tiene la foto `IMG_4821.CR3` y se renombra con la opción de renombrar archivos
- **THEN** `IMG_4821.CR3` conserva su nombre

#### Scenario: Sin la opción
- **WHEN** el proyecto se renombra sin la opción de renombrar archivos
- **THEN** los archivos conservan su nombre y solo cambia la carpeta

#### Scenario: Exportados intactos
- **WHEN** se renombran los archivos de un proyecto con JPEG en `_exported/`
- **THEN** los archivos de `_exported/` conservan su nombre y las fotos ya enlazadas siguen enlazadas

#### Scenario: Fallo a mitad
- **WHEN** falla el renombrado de un archivo después de renombrar otros
- **THEN** los archivos ya renombrados vuelven a su nombre anterior y se informa del error

### Requirement: Actualizar metadatos de las fotos existentes
Si cambian la descripción o el año de la sesión y el usuario lo pide, cada foto del proyecto SHALL pasar a llevar la descripción `{descripción}@{año}`, o ninguna si la descripción queda vacía, con la misma ubicación que al importar (dentro del archivo en Linux y Windows, en el sidecar `.xmp` en Android). El sistema SHALL informar cuántas fotos se actualizaron y cuáles fallaron.

#### Scenario: Corregir la descripción
- **WHEN** el usuario cambia la descripción a "Sesión en la playa" en un proyecto de 2024 con la opción de actualizar metadatos
- **THEN** las fotos del proyecto llevan la descripción `Sesión en la playa@2024`

#### Scenario: Sin la opción
- **WHEN** el usuario cambia la descripción sin la opción de actualizar metadatos
- **THEN** las fotos existentes conservan su descripción y las próximas importaciones usan la nueva

#### Scenario: Fotos que fallan
- **WHEN** algunas fotos no se pueden escribir durante la actualización
- **THEN** el resto se actualiza y el resultado lista las que fallaron

## REMOVED Requirements

### Requirement: Renombrar proyecto
**Reason**: Lo sustituye "Editar proyecto", que además de renombrar permite cambiar la fecha de sesión y la descripción, con las mismas reglas de carpeta y colisión.
**Migration**: El comando `rename_project` se reemplaza por `edit_project`; sus escenarios pasan a "Editar proyecto".
