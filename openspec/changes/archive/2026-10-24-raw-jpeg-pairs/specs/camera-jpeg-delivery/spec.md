# Spec Delta

## Purpose

Permite entregar el JPG que genera la cámara (por ejemplo, con una receta de película) directamente al aprobar, sin pasar por un editor, guardando el RAW por si hace falta revelarlo.

## ADDED Requirements

### Requirement: Opción de entrega del proyecto
Cada proyecto SHALL tener la opción "Al aprobar, entregar directamente" con tres valores: *nada*, *el JPG de los pares RAW+JPG* y *todo JPG de cámara* (pares y JPG sueltos). Un proyecto nuevo SHALL tomar el valor global de fábrica, y después SHALL conservar el suyo aunque el global cambie.

#### Scenario: Proyecto nuevo
- **WHEN** el valor global es *el JPG de los pares RAW+JPG* y el usuario crea un proyecto
- **THEN** el proyecto queda con *el JPG de los pares RAW+JPG*

#### Scenario: Cambiar el global
- **WHEN** un proyecto tiene *nada* y el usuario cambia el valor global a *todo JPG de cámara*
- **THEN** ese proyecto sigue con *nada*

### Requirement: Destino de los archivos al aprobar
Al aprobar, el RAW SHALL ir a `_culled/`. El JPG de un par SHALL moverse a `_exported/` con los valores *pares* y *todo JPG de cámara*, y a `_culled/` con *nada*. Un JPG suelto SHALL ir a `_culled/` y, solo con *todo JPG de cámara*, SHALL copiarse además a `_exported/`, para que su único original nunca quede expuesto a una sobrescritura. Un JPG entregado SHALL quedar enlazado a su foto, que pasa a estado editado.

#### Scenario: Par con entrega de pares
- **WHEN** el proyecto entrega *el JPG de los pares RAW+JPG* y el usuario aprueba el par `DSCF1234`
- **THEN** `DSCF1234.RAF` queda en `_culled/`, `DSCF1234.JPG` en `_exported/` y la foto figura como aprobada y editada

#### Scenario: Par sin entrega
- **WHEN** el proyecto entrega *nada* y el usuario aprueba un par
- **THEN** los dos archivos quedan en `_culled/` y `_exported/` no cambia

#### Scenario: JPG suelto con entrega de pares
- **WHEN** el proyecto entrega *el JPG de los pares RAW+JPG* y el usuario aprueba un JPG suelto
- **THEN** el JPG queda en `_culled/`

#### Scenario: JPG suelto con todo JPG de cámara
- **WHEN** el proyecto entrega *todo JPG de cámara* y el usuario aprueba el JPG suelto `DSCF0008.JPG`
- **THEN** el original queda en `_culled/` y una copia en `_exported/`, enlazada a su foto

#### Scenario: Sobrescribir la copia de un JPG suelto
- **WHEN** el fotógrafo edita ese JPG y exporta encima de `_exported/DSCF0008.JPG`
- **THEN** el original de cámara sigue intacto en `_culled/`

#### Scenario: Ya existe un archivo con ese nombre
- **WHEN** al aprobar, `_exported/` ya tiene un archivo con el nombre del JPG
- **THEN** el archivo existente no se modifica, el JPG queda en `_culled/` y la foto sigue enlazada al exportado existente

### Requirement: Huella del JPG entregado
Al entregar un JPG, el sistema SHALL guardar su tamaño y su SHA-256. Un JPG de `_exported/` SHALL considerarse el de cámara solo mientras coincida con esa huella; si no coincide, SHALL tratarse como una edición del fotógrafo.

#### Scenario: El fotógrafo sobrescribe con su edición
- **WHEN** el fotógrafo revela el RAW y exporta su edición a `_exported/` con el mismo nombre que el JPG entregado
- **THEN** Lumik trata ese archivo como una edición del fotógrafo

### Requirement: Regreso del JPG entregado
Al quitar la aprobación de una foto con un JPG entregado, el JPG de un par SHALL regresar junto al RAW solo si coincide con su huella, y la copia de un JPG suelto SHALL borrarse solo si coincide con su huella, porque su original sigue en el proyecto. Una edición del fotógrafo SHALL quedarse en `_exported/`. Regresar un JPG no SHALL borrar nada en la nube.

#### Scenario: Quitar la aprobación de un par entregado
- **WHEN** el usuario quita la aprobación de un par cuyo JPG de cámara está en `_exported/`
- **THEN** el RAW y el JPG vuelven a `_media/` y la foto deja de estar editada

#### Scenario: Quitar la aprobación con una edición encima
- **WHEN** el usuario quita la aprobación de un par cuyo JPG en `_exported/` ya es una edición suya
- **THEN** el RAW vuelve a `_media/` y la edición se queda en `_exported/`, enlazada a la foto

#### Scenario: Quitar la aprobación de un JPG suelto entregado
- **WHEN** el usuario quita la aprobación de un JPG suelto cuya copia en `_exported/` no cambió
- **THEN** la copia se borra de `_exported/` y el original vuelve de `_culled/` a `_media/`

#### Scenario: JPG que ya estaba en la nube
- **WHEN** se regresa un JPG que ya se había respaldado
- **THEN** sale de `_exported/` y su copia en la nube no se borra

### Requirement: Aviso al cambiar la opción de entrega
Si al cambiar la opción de entrega de un proyecto hay fotos aprobadas a las que el nuevo valor afecta, el sistema SHALL preguntar si se aplica también a ellas, y SHALL indicar cuántos JPG se entregarán o regresarán. Al regresar, SHALL indicar cuántos se quedan por ser ediciones del fotógrafo y cuántos siguen en la nube. Elegir no aplicarlo SHALL cambiar solo el valor.

#### Scenario: Pasar a entregar pares
- **WHEN** el proyecto tiene 40 pares aprobados con sus JPG en `_culled/` y el usuario cambia a *el JPG de los pares RAW+JPG*
- **THEN** el aviso ofrece entregar los 40 JPG y, si se acepta, quedan en `_exported/`

#### Scenario: Pasar a nada
- **WHEN** el proyecto tiene 40 JPG entregados, 2 ya reemplazados por ediciones y 12 respaldados, y el usuario cambia a *nada*
- **THEN** el aviso indica que regresarán 38 JPG, que 2 se quedan por ser ediciones y que 12 siguen en la nube; si se acepta, los 38 vuelven a `_culled/`

#### Scenario: Solo cambiar el valor
- **WHEN** el usuario elige no aplicarlo a las fotos aprobadas
- **THEN** las fotos aprobadas no se mueven y las siguientes aprobaciones usan el nuevo valor

#### Scenario: Sin fotos afectadas
- **WHEN** el proyecto no tiene fotos aprobadas a las que afecte el cambio
- **THEN** el valor cambia sin aviso

### Requirement: Entrega al unirse un JPG a una foto aprobada
Si un JPG se une a una foto que ya estaba aprobada, SHALL ir al destino que le corresponde según la opción de entrega del proyecto en ese momento.

#### Scenario: JPG de la segunda tarjeta para un RAW aprobado
- **WHEN** el proyecto entrega *el JPG de los pares RAW+JPG*, `DSCF1234.RAF` ya está aprobado y se importa `DSCF1234.JPG`
- **THEN** el JPG queda en `_exported/`, enlazado a la foto, sin volver a aprobarla
