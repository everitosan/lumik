# Spec Delta

## MODIFIED Requirements

### Requirement: Espacio suficiente antes de importar
El import no SHALL poder iniciarse si el espacio libre del disco del proyecto es menor que el tamaño total de los archivos seleccionados. El paso de confirmación SHALL mostrar el espacio disponible. Como la preparación de las fotos ocurre en el mismo disco, ese espacio SHALL ser el único que la importación necesita: no SHALL depender del espacio libre de la carpeta temporal del sistema.

#### Scenario: No caben los archivos
- **WHEN** se seleccionan 40 GB de archivos y el disco del proyecto tiene 25 GB libres
- **THEN** el botón para iniciar la importación está desactivado y se indica que no hay espacio suficiente

#### Scenario: Caben los archivos
- **WHEN** el tamaño seleccionado es menor que el espacio libre
- **THEN** el usuario puede iniciar la importación

#### Scenario: Carpeta temporal del sistema pequeña
- **WHEN** se importan 20 GB de fotos a un disco con 500 GB libres y la carpeta temporal del sistema tiene 8 GB libres
- **THEN** la importación termina correctamente

### Requirement: Copia sin conversión
Las fotos SHALL copiarse en su formato original a `_media/` dentro de la carpeta del proyecto, y los videos a `_video/`, sin conversión. A los videos no se les SHALL escribir metadatos. Los archivos de origen no SHALL modificarse. Un archivo que ya existía en `_media/` o `_video/` no SHALL sobrescribirse.

#### Scenario: Import de RAW y video
- **WHEN** se importan `IMG_1.CR3` y `CLIP.mov`
- **THEN** el CR3 queda en `_media/` y el MOV en `_video/`, y ambos siguen intactos en la tarjeta de origen

### Requirement: Progreso del import
El import SHALL informar, mientras avanza, la fase actual (videos, copiar —con firma y renombrado en escritorio—, metadatos en Android, mover, registrar), el archivo que se está procesando con su número dentro del total, los bytes copiados de ese archivo y de toda la importación, y un registro de actividad legible con una entrada por archivo procesado o fallido. En la fase de metadatos SHALL bastar con indicar la fase, sin porcentaje. Los textos SHALL mostrarse en el idioma de la app.

#### Scenario: Seguimiento en curso
- **WHEN** hay un import en curso
- **THEN** la interfaz muestra la fase actual, el archivo actual, la barra de progreso según los bytes totales y las entradas del registro a medida que avanzan

#### Scenario: Video grande
- **WHEN** se copia un video de 6 GB
- **THEN** la interfaz muestra su nombre y los bytes copiados aumentando hasta 6 GB, sin quedarse sin cambios mientras dura la copia

#### Scenario: Copia de una sesión de fotos
- **WHEN** se copian 400 fotos
- **THEN** la interfaz indica cuál se está copiando y su número (p. ej. "42 de 400"), y cada foto copiada aparece en el registro

## ADDED Requirements

### Requirement: Resultado del import siempre visible
Al terminar, el paso de confirmación SHALL mostrar el resultado: cuántas fotos se importaron, cuántos videos se copiaron, qué archivos fallaron con su motivo, qué archivos se ignoraron por formato, y el registro de actividad. Si el import termina por un error general, SHALL mostrar ese motivo junto con lo que sí quedó importado. La vista no SHALL volver a la confirmación previa ni perder el registro.

#### Scenario: Import completo
- **WHEN** todas las fotos y videos se importan
- **THEN** se muestra cuántas fotos se importaron, cuántos videos se copiaron, el tiempo total y el registro

#### Scenario: Import parcial
- **WHEN** se importan 398 de 400 fotos y dos no se pudieron leer
- **THEN** se muestran las 398 importadas y la lista de las dos fallidas, cada una con su nombre y un motivo legible

#### Scenario: Error general
- **WHEN** el import termina porque no se pudo crear la carpeta del proyecto
- **THEN** la vista muestra el motivo y el registro, y no vuelve al botón "Importar" como si no hubiera pasado nada

#### Scenario: Archivos ignorados
- **WHEN** entre los archivos seleccionados hay uno de formato no admitido o una ruta que ya no es un archivo
- **THEN** el resultado lo lista como ignorado con el motivo

### Requirement: Fallos por archivo sin cancelar el lote
Si un archivo concreto no se puede copiar, preparar o mover al proyecto, el import SHALL registrarlo como fallido con su motivo y SHALL continuar con los demás archivos. Un archivo fallido no SHALL quedar a medias en `_media/` ni en `_video/`, ni registrarse en el proyecto. Solo un problema que impide importar cualquier archivo SHALL terminar el import antes de tiempo.

#### Scenario: Una foto ilegible
- **WHEN** en una importación de 400 fotos una no se puede leer de la tarjeta
- **THEN** las otras 399 se importan y registran, y la ilegible aparece como fallida

#### Scenario: Un video falla
- **WHEN** falla la copia de uno de tres videos
- **THEN** los otros dos quedan en `_video/`, las fotos se importan igual, y el video fallido aparece como fallido sin dejar un archivo incompleto en `_video/`

#### Scenario: Disco destino lleno a mitad
- **WHEN** el disco destino se llena durante la copia
- **THEN** el archivo en curso y los siguientes aparecen como fallidos por falta de espacio, y los que ya se habían importado quedan en el proyecto

### Requirement: Preparación en el disco del proyecto
La preparación de las fotos (renombrado y metadatos) SHALL hacerse en una carpeta oculta dentro de la carpeta del proyecto en el disco destino, no en la carpeta temporal del sistema. Al terminar, esa carpeta SHALL eliminarse. Si quedaron restos de una importación interrumpida, SHALL eliminarse al iniciar la siguiente importación en ese proyecto. La carpeta de preparación no SHALL mostrarse como contenido del proyecto.

#### Scenario: Importación terminada
- **WHEN** termina una importación, con o sin fallos
- **THEN** no queda la carpeta de preparación en el proyecto ni copias en la carpeta temporal del sistema

#### Scenario: Importación interrumpida
- **WHEN** la app se cerró durante una importación y después se inicia otra en el mismo proyecto
- **THEN** los restos de la importación anterior se eliminan y no aparecen como fotos del proyecto

### Requirement: Nombres sin sobrescritura
Si el nombre final de una foto o video ya existe en `_media/` o `_video/`, el archivo nuevo SHALL guardarse con un sufijo `-001`, `-002`… antes de la extensión, el primero que esté libre, y SHALL importarse igual. El archivo existente no SHALL modificarse.

#### Scenario: Misma numeración de dos cámaras
- **WHEN** `_media/` ya tiene `IMG_0001.CR2` y se importa otro `IMG_0001.CR2` con el renombrado desactivado
- **THEN** el nuevo queda como `IMG_0001-001.CR2` y el existente no cambia

#### Scenario: Nombre por fecha ya ocupado
- **WHEN** `_media/` ya tiene `2024-06-15__14-30-22__Boda_Ana.cr2` y se importa otra foto con esa misma fecha de captura
- **THEN** la nueva se guarda con el primer sufijo libre y ambas quedan registradas en el proyecto

#### Scenario: Video repetido
- **WHEN** `_video/` ya tiene `CLIP_001.MP4` y se importa otro `CLIP_001.MP4`
- **THEN** el nuevo queda como `CLIP_001-001.MP4`
