# photo-culling Specification

## Purpose

Permite seleccionar durante el culling las fotos que vale la pena editar, apartándolas físicamente en `_culled/` para separarlas del resto de la sesión, de forma reversible y sin borrar nunca un archivo.

## Requirements

### Requirement: Aprobar mueve el archivo a `_culled/`
En Lumik "culled" significa **aprobada/seleccionada**. Aprobar una foto SHALL mover su archivo de `_media/` a `_culled/` dentro de la misma carpeta de proyecto, conservando el nombre, y SHALL actualizar su ruta guardada. Quitar la aprobación SHALL devolverlo a `_media/`. Ninguna de las dos operaciones SHALL borrar el archivo.

#### Scenario: Aprobar
- **WHEN** el usuario aprueba `lumik/2024/06/15_Boda/_media/IMG_0001.cr2`
- **THEN** el archivo queda en `lumik/2024/06/15_Boda/_culled/IMG_0001.cr2` y la foto figura como aprobada

#### Scenario: Quitar aprobación
- **WHEN** el usuario quita la aprobación de una foto aprobada
- **THEN** el archivo vuelve a `_media/` y la foto deja de figurar como aprobada

#### Scenario: Carpeta destino inexistente
- **WHEN** se aprueba la primera foto de un proyecto sin carpeta `_culled/`
- **THEN** la carpeta se crea y el archivo se mueve a ella

### Requirement: Operación idempotente
Pedir el mismo estado que la foto ya tiene SHALL completarse sin mover nada.

#### Scenario: Aprobar una foto ya aprobada
- **WHEN** se aprueba una foto que ya estaba aprobada
- **THEN** la operación termina con éxito y el archivo no se mueve

### Requirement: Los sidecars acompañan a la foto
Al aprobar o quitar la aprobación, el sidecar `.xmp` de la foto y su sidecar de etiquetas para iPad (`._{nombre}`), si existen, SHALL moverse junto al archivo.

#### Scenario: Foto con sidecar XMP
- **WHEN** se aprueba `IMG_0001.cr2` que tiene `IMG_0001.xmp` al lado
- **THEN** ambos archivos quedan en `_culled/`

### Requirement: Rollback si falla el guardado
Si el archivo se movió pero no se pudo guardar el nuevo estado, el sistema SHALL devolver el archivo a su ubicación original e informar el error, para que el disco y la base de datos del proyecto no queden desalineados.

#### Scenario: Fallo de base de datos
- **WHEN** al aprobar una foto falla el guardado del estado
- **THEN** el archivo vuelve a `_media/`, la foto sigue sin aprobar y se muestra el error

#### Scenario: Fallo al mover
- **WHEN** el sistema de archivos impide mover la foto
- **THEN** se informa el error y no cambia nada

### Requirement: Actualización optimista en la interfaz
La interfaz SHALL reflejar el cambio al instante, antes de que el sistema lo confirme, SHALL indicar en el grid qué fotos están aprobadas y SHALL permitir alternar la aprobación con un atajo configurable (por defecto la barra espaciadora).

#### Scenario: Aprobar desde el visor
- **WHEN** el usuario pulsa la barra espaciadora en el visor
- **THEN** la foto aparece como aprobada de inmediato

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
