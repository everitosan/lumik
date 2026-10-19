# Spec Delta

## Purpose

Permite quitar del proyecto una foto que no sirve sin borrar nunca un archivo: la foto se mueve a la papelera del proyecto (la carpeta `_trash/`) y puede restaurarse desde la app.

## ADDED Requirements

### Requirement: Mover a la papelera desde el menú de la foto
El menú de acciones de cada foto del grid SHALL ofrecer "Mover a la papelera". Antes de moverla SHALL mostrarse un modal de confirmación que nombre el archivo, diga que se moverá a la papelera del proyecto (`_trash/`) y que se puede restaurar desde el filtro "Papelera", con las acciones "Cancelar" y "Mover a la papelera". Cancelar no SHALL cambiar nada.

#### Scenario: Confirmar
- **WHEN** el usuario elige "Mover a la papelera" en `IMG_0001.cr2` y confirma
- **THEN** la foto deja de verse en el grid

#### Scenario: Cancelar
- **WHEN** el usuario elige "Mover a la papelera" y cierra el modal con "Cancelar"
- **THEN** la foto sigue en el grid y ningún archivo se mueve

### Requirement: La papelera aparta los archivos en `_trash/`
Mover una foto a la papelera SHALL marcarla como en la papelera y SHALL mover su archivo original a `_trash/` dentro de la carpeta del proyecto, conservando el nombre, junto con su sidecar `.xmp` y su sidecar `._{nombre}` si existen. Ningún archivo SHALL borrarse. La carpeta `_trash/` SHALL crearse si no existe.

#### Scenario: Foto sin aprobar
- **WHEN** se mueve a la papelera `lumik/2024/06/15_Boda/_media/IMG_0001.cr2`
- **THEN** el archivo queda en `lumik/2024/06/15_Boda/_trash/IMG_0001.cr2`

#### Scenario: Foto aprobada con sidecars
- **WHEN** se mueve a la papelera `_culled/IMG_0002.cr2`, que tiene `IMG_0002.xmp` y `._IMG_0002.cr2` al lado
- **THEN** los tres archivos quedan en `_trash/`

### Requirement: El exportado acompaña a la foto en la papelera
Si la foto movida a la papelera tiene un JPG exportado enlazado, ese archivo y su sidecar `._{nombre}` SHALL moverse a `_trash/_exported/`, de modo que no se vuelva a enlazar ni a respaldar.

#### Scenario: Foto con exportado
- **WHEN** se mueve a la papelera `IMG_0001.cr2`, enlazada a `_exported/IMG_0001.jpg`
- **THEN** el exportado queda en `_trash/_exported/IMG_0001.jpg` y `_exported/` ya no lo contiene

#### Scenario: Original JPG con exportado del mismo nombre
- **WHEN** se mueve a la papelera `_media/IMG_0003.jpg`, enlazada a `_exported/IMG_0003.jpg`
- **THEN** quedan `_trash/IMG_0003.jpg` y `_trash/_exported/IMG_0003.jpg` sin sobrescribirse

### Requirement: Efectos de una foto en la papelera sobre el proyecto
Una foto en la papelera no SHALL aparecer en el grid salvo con el filtro "Papelera", no SHALL contar en el número de fotos del proyecto ni en sus estados de flujo, y no SHALL respaldarse. Si era la portada del proyecto, el proyecto SHALL quedar sin portada.

#### Scenario: Conteo del dashboard
- **WHEN** un proyecto de 120 fotos tiene 3 en la papelera
- **THEN** el dashboard muestra 117 fotos

#### Scenario: Era la portada
- **WHEN** se mueve a la papelera la foto que es portada del proyecto
- **THEN** el proyecto se muestra sin portada

#### Scenario: Respaldo posterior
- **WHEN** se respalda un proyecto con una foto en la papelera pendiente de subir
- **THEN** esa foto no se sube

### Requirement: Restaurar una foto de la papelera
Con el filtro "Papelera" activo, el menú de cada foto SHALL ofrecer "Restaurar" en lugar de "Mover a la papelera", sin confirmación. Restaurar SHALL sacar la foto de la papelera y devolver el original y sus sidecars a `_culled/` si la foto estaba aprobada o a `_media/` si no, y el exportado y su sidecar a `_exported/`.

#### Scenario: Restaurar foto aprobada
- **WHEN** se restaura una foto de la papelera que estaba aprobada
- **THEN** su archivo vuelve a `_culled/`, sigue aprobada y reaparece en el grid normal

#### Scenario: Restaurar con exportado
- **WHEN** se restaura una foto cuyo exportado está en `_trash/_exported/`
- **THEN** el exportado vuelve a `_exported/` y la foto sigue enlazada a él

### Requirement: Sin sobrescribir al restaurar
Si en la carpeta destino ya existe un archivo con el mismo nombre, restaurar SHALL fallar sin mover nada e informar el conflicto.

#### Scenario: Nombre ocupado
- **WHEN** se restaura `IMG_0001.cr2` y ya existe `_media/IMG_0001.cr2`
- **THEN** se informa el error, la foto sigue en la papelera y ningún archivo se mueve

### Requirement: Disco y base de datos alineados
Si los archivos se movieron pero no se pudo guardar el nuevo estado, al mover a la papelera o al restaurar, el sistema SHALL devolver los archivos ya movidos a su ubicación anterior e informar el error. Si falla mover el original, no SHALL cambiar nada.

#### Scenario: Fallo de base de datos al mover a la papelera
- **WHEN** al mover una foto a la papelera falla el guardado del estado
- **THEN** los archivos vuelven a su carpeta, la foto sigue en el grid y se muestra el error

#### Scenario: Fallo al mover el exportado
- **WHEN** el original se movió a `_trash/` pero el exportado no se puede mover
- **THEN** el original vuelve a su carpeta, la foto no queda en la papelera y se muestra el error

### Requirement: Una foto en la papelera no se aprueba
Aprobar o quitar la aprobación de una foto en la papelera SHALL rechazarse sin mover archivos, para que nunca salga de `_trash/` sin restaurarse.

#### Scenario: Atajo de aprobar en la vista de papelera
- **WHEN** con el filtro "Papelera" activo el usuario intenta aprobar una foto
- **THEN** la foto no cambia y su archivo sigue en `_trash/`
