# Spec Delta

## MODIFIED Requirements

### Requirement: Mover a la papelera desde el menú de la foto
El menú de acciones de cada foto del grid SHALL ofrecer "Mover a la papelera". Antes de moverla SHALL mostrarse un modal de confirmación que nombre el archivo, diga que se moverá a la papelera del proyecto (`_trash/`) y que se puede restaurar desde la sección "Papelera", con las acciones "Cancelar" y "Mover a la papelera". Cancelar no SHALL cambiar nada.

#### Scenario: Confirmar
- **WHEN** el usuario elige "Mover a la papelera" en `IMG_0001.cr2` y confirma
- **THEN** la foto deja de verse en el grid

#### Scenario: Cancelar
- **WHEN** el usuario elige "Mover a la papelera" y cierra el modal con "Cancelar"
- **THEN** la foto sigue en el grid y ningún archivo se mueve

### Requirement: Efectos de una foto en la papelera sobre el proyecto
Una foto en la papelera no SHALL aparecer en el grid de su proyecto, no SHALL contar en el número de fotos del proyecto ni en sus estados de flujo, y no SHALL respaldarse. Si era la portada del proyecto, el proyecto SHALL quedar sin portada.

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
Desde la sección "Papelera", cada foto suelta SHALL ofrecer "Restaurar", sin confirmación. Restaurar SHALL sacar la foto de la papelera y devolver el original y sus sidecars a `_culled/` si la foto estaba aprobada o a `_media/` si no, y el exportado y su sidecar a `_exported/`.

#### Scenario: Restaurar foto aprobada
- **WHEN** se restaura desde la sección "Papelera" una foto que estaba aprobada
- **THEN** su archivo vuelve a `_culled/`, sigue aprobada y reaparece en el grid de su proyecto

#### Scenario: Restaurar con exportado
- **WHEN** se restaura una foto cuyo exportado está en `_trash/_exported/`
- **THEN** el exportado vuelve a `_exported/` y la foto sigue enlazada a él

### Requirement: Sin sobrescribir al restaurar
Si en la carpeta destino ya existe un archivo con el mismo nombre que el original o que el exportado, restaurar no SHALL mover nada y SHALL mostrar el modal de colisión de la sección "Papelera". Si el usuario elige otro nombre, el original, sus sidecars y el exportado SHALL restaurarse con ese nombre, conservando sus extensiones, y la foto SHALL quedar registrada con el nombre nuevo.

#### Scenario: Nombre ocupado
- **WHEN** se restaura `IMG_0001.cr2` y ya existe `_media/IMG_0001.cr2`
- **THEN** se muestra el modal de colisión, la foto sigue en la papelera y ningún archivo se mueve

#### Scenario: Restaurar con otro nombre
- **WHEN** en el modal de colisión el usuario escribe `IMG_0001_b` para `IMG_0001.cr2`, que tiene `IMG_0001.xmp` y exportado `IMG_0001.jpg`
- **THEN** quedan `_media/IMG_0001_b.cr2`, `_media/IMG_0001_b.xmp` y `_exported/IMG_0001_b.jpg`, y la foto sigue enlazada a su exportado

### Requirement: Una foto en la papelera no se aprueba
Aprobar o quitar la aprobación de una foto en la papelera SHALL rechazarse sin mover archivos, para que nunca salga de `_trash/` sin restaurarse.

#### Scenario: Atajo de aprobar en la vista de papelera
- **WHEN** desde la sección "Papelera" o su visor se intenta aprobar una foto
- **THEN** la foto no cambia y su archivo sigue en `_trash/`

## ADDED Requirements

### Requirement: Eliminar definitivamente una foto
Eliminar definitivamente una foto en la papelera SHALL borrar del disco su original, sus sidecars, su exportado con su sidecar, su miniatura y su preview, y SHALL borrar su registro y todo lo asociado a ella en la base del proyecto. No SHALL tocar su respaldo en la nube. Solo SHALL poder hacerse desde la sección "Papelera".

#### Scenario: Foto con exportado
- **WHEN** se elimina definitivamente `IMG_0001.cr2`, con `IMG_0001.xmp` y exportado en `_trash/_exported/IMG_0001.jpg`
- **THEN** ninguno de esos archivos, ni la miniatura ni la preview de la foto, existe ya en el disco

#### Scenario: Sin rastro en el proyecto
- **WHEN** se elimina definitivamente una foto de "Boda"
- **THEN** el proyecto ya no tiene registro de esa foto, y abrir "Boda" o restaurar otras fotos no la hace reaparecer

#### Scenario: Par RAW + JPG
- **WHEN** se elimina definitivamente el par `DSCF1234.RAF` + `DSCF1234.JPG`
- **THEN** se borran los dos archivos y sus sidecars

#### Scenario: Archivo que ya no está
- **WHEN** se elimina definitivamente una foto cuyo original ya no existe en `_trash/`
- **THEN** se borran los demás archivos y el registro de la foto sin error
