# Spec Delta

## MODIFIED Requirements

### Requirement: Aprobar mueve el archivo a `_culled/`
En Lumik "culled" significa **aprobada/seleccionada**. Aprobar una foto SHALL mover su archivo original de `_media/` a `_culled/` dentro de la misma carpeta de proyecto, conservando el nombre, y SHALL actualizar su ruta guardada. El JPG de un par SHALL ir a `_culled/` o a `_exported/` según la opción de entrega del proyecto. Quitar la aprobación SHALL devolver a `_media/` el original y el JPG del par que no sea una edición del fotógrafo. Ninguna de las dos operaciones SHALL borrar un archivo, salvo la copia de entrega de un JPG suelto que Lumik hizo y que sigue idéntica a su original.

#### Scenario: Aprobar
- **WHEN** el usuario aprueba `lumik/2024/06/15_Boda/_media/IMG_0001.cr2`
- **THEN** el archivo queda en `lumik/2024/06/15_Boda/_culled/IMG_0001.cr2` y la foto figura como aprobada

#### Scenario: Quitar aprobación
- **WHEN** el usuario quita la aprobación de una foto aprobada
- **THEN** el archivo vuelve a `_media/` y la foto deja de figurar como aprobada

#### Scenario: Carpeta destino inexistente
- **WHEN** se aprueba la primera foto de un proyecto sin carpeta `_culled/`
- **THEN** la carpeta se crea y el archivo se mueve a ella

#### Scenario: Aprobar un par sin entrega
- **WHEN** el proyecto entrega *nada* y el usuario aprueba el par `IMG_0001.cr2` + `IMG_0001.jpg`
- **THEN** los dos archivos quedan en `_culled/`

### Requirement: Los sidecars acompañan a la foto
Al aprobar o quitar la aprobación, el sidecar `.xmp` de la foto y los sidecars de etiquetas para iPad (`._{nombre}`) de sus archivos, si existen, SHALL moverse junto al archivo al que pertenecen.

#### Scenario: Foto con sidecar XMP
- **WHEN** se aprueba `IMG_0001.cr2` que tiene `IMG_0001.xmp` al lado
- **THEN** ambos archivos quedan en `_culled/`

#### Scenario: Par entregado con sidecars para iPad
- **WHEN** se aprueba un par con entrega de JPG y existen `._IMG_0001.cr2` y `._IMG_0001.jpg`
- **THEN** `._IMG_0001.cr2` queda en `_culled/` y `._IMG_0001.jpg` en `_exported/`
