# photo-import Specification

## Purpose

Lleva las fotos y videos de una sesión desde la tarjeta o carpeta de origen al proyecto en el disco externo, conservando el formato original, firmando los archivos con los datos del fotógrafo y dejándolos listos para hacer culling.

## Requirements

### Requirement: Formatos admitidos
El import SHALL aceptar fotos RAW (`cr2`, `cr3`, `nef`, `arw`, `raf`, `orf`, `rw2`, `dng`), JPEG (`jpg`, `jpeg`) y TIFF (`tif`, `tiff`), y videos (`mp4`, `mov`, `avi`, `mts`, `m2ts`, `mkv`, `mxf`), sin distinguir mayúsculas. Los archivos de otros formatos SHALL ignorarse.

#### Scenario: Selección mixta
- **WHEN** se seleccionan `A.CR3`, `B.jpg`, `C.mp4` y `D.png`
- **THEN** se importan `A.CR3` y `B.jpg` como fotos, `C.mp4` como video, y `D.png` se ignora

### Requirement: Wizard de importación
El import SHALL guiar al usuario por los pasos origen, destino y confirmación. Cuando se inicia desde un proyecto abierto, el paso de destino SHALL omitirse. En origen el usuario SHALL poder añadir archivos arrastrándolos, sin duplicar los que ya estaban en la lista.

#### Scenario: Import desde el dashboard
- **WHEN** el usuario abre el import desde el menú principal
- **THEN** recorre los pasos origen, destino y confirmación

#### Scenario: Import desde un proyecto
- **WHEN** el usuario pulsa importar dentro de un proyecto
- **THEN** el wizard solo muestra origen y confirmación, con ese proyecto como destino

#### Scenario: Arrastrar archivos repetidos
- **WHEN** el usuario arrastra un archivo que ya estaba en la lista
- **THEN** no se añade de nuevo

### Requirement: Copia sin conversión
Las fotos SHALL copiarse en su formato original a `_media/` dentro de la carpeta del proyecto, y los videos a `_video/`, sin conversión. A los videos no se les SHALL escribir metadatos. Los archivos de origen no SHALL modificarse.

#### Scenario: Import de RAW y video
- **WHEN** se importan `IMG_1.CR3` y `CLIP.mov`
- **THEN** el CR3 queda en `_media/` y el MOV en `_video/`, y ambos siguen intactos en la tarjeta de origen

### Requirement: Renombrado por fecha de captura
Si el ajuste "renombrar al importar" está activo (por defecto sí), cada foto SHALL renombrarse a `{AAAA-MM-DD}__{HH-MM-SS}__{proyecto}.{ext}` a partir de su fecha de captura, con el nombre del proyecto saneado (caracteres no alfanuméricos salvo `-` y `_` pasan a `_`). Si varias fotos comparten fecha, a partir de la segunda se SHALL añadir un sufijo `-001`, `-002`…

#### Scenario: Nombres únicos
- **WHEN** se importa una foto capturada el 2024-06-15 a las 14:30:22 al proyecto "Boda Ana"
- **THEN** se guarda como `2024-06-15__14-30-22__Boda_Ana.cr2`

#### Scenario: Ráfaga en el mismo segundo
- **WHEN** dos fotos tienen la misma fecha y hora de captura
- **THEN** una conserva el nombre limpio y la otra recibe el sufijo `-001`

#### Scenario: TIFF de escáner sin fecha de captura
- **WHEN** en escritorio una foto no tiene fecha de captura
- **THEN** se usa su fecha de modificación para el nombre

#### Scenario: Renombrado desactivado
- **WHEN** el ajuste "renombrar al importar" está desactivado
- **THEN** las fotos conservan su nombre original

### Requirement: Metadatos del fotógrafo
Si el ajuste "embeber metadatos al importar" está activo (por defecto sí), cada foto SHALL llevar autor, copyright, URL y email de contacto y condiciones de uso del fotógrafo activo, más la descripción `{descripción}@{año}` si el proyecto tiene descripción. Los campos vacíos SHALL omitirse. En Linux y Windows se SHALL escribir dentro del archivo; en Android, en un sidecar `.xmp` junto a la foto.

#### Scenario: Escritorio
- **WHEN** se importa en Linux con autor y copyright configurados
- **THEN** el RAW importado contiene esos valores en sus metadatos

#### Scenario: Android
- **WHEN** se importa en Android con autor configurado
- **THEN** junto a cada foto se crea un `.xmp` con el autor y la foto no se modifica

#### Scenario: Sin nada que escribir
- **WHEN** el renombrado está desactivado y no hay metadatos que escribir
- **THEN** las fotos se copian sin modificarse

### Requirement: Registro de fotos y metadatos EXIF
Cada foto importada SHALL registrarse en el proyecto con su ruta relativa al punto de montaje y los datos EXIF disponibles: cámara, formato, fecha de captura, dimensiones, tamaño, ISO, apertura, velocidad, compensación de exposición, focal, objetivo y rotación. Las fotos se SHALL registrar todas juntas; si el registro falla, se SHALL informar como fallidas.

#### Scenario: Import correcto
- **WHEN** se importan 120 fotos
- **THEN** el proyecto muestra 120 fotos con ISO, apertura y velocidad visibles en el panel de información

#### Scenario: Fallo al registrar
- **WHEN** falla la escritura en la base de datos del proyecto
- **THEN** el resultado del import lista todas las fotos del lote como fallidas con el motivo

### Requirement: Miniaturas
Tras registrar las fotos, el sistema SHALL generar para cada una una miniatura JPEG en `.thumbs/{id}.jpg` con la orientación ya aplicada. El usuario SHALL poder regenerar las miniaturas de un proyecto; al hacerlo, la rotación guardada SHALL corregirse con la del archivo cuando no coincidan.

#### Scenario: Grid tras importar
- **WHEN** termina un import
- **THEN** el grid del proyecto muestra la miniatura de cada foto con la orientación correcta

#### Scenario: Regenerar miniaturas
- **WHEN** el usuario regenera las miniaturas de un proyecto cuya rotación guardada difiere de la del archivo
- **THEN** se recrean las miniaturas y la rotación guardada pasa a ser la del archivo

### Requirement: Progreso del import
El import SHALL informar su progreso en fases (copiar, metadatos, registrar) y un registro de actividad legible, y al terminar SHALL informar cuántas fotos se importaron, cuántos videos se copiaron y qué archivos fallaron con su motivo.

#### Scenario: Seguimiento en curso
- **WHEN** hay un import en curso
- **THEN** la interfaz muestra la fase actual y las entradas del registro a medida que avanzan

### Requirement: Fecha de sesión inferida
Si el proyecto no tiene fecha de sesión y el import registró al menos una foto, el sistema SHALL tomar como fecha de sesión la fecha de captura más antigua y mover la carpeta del proyecto a la ruta de esa fecha. Si mover la carpeta falla, el import SHALL considerarse exitoso igualmente.

#### Scenario: Proyecto sin fecha
- **WHEN** se importan fotos de 2023-11-04 a un proyecto creado hoy sin fecha de sesión
- **THEN** el proyecto pasa a tener fecha de sesión `2023-11-04` y su carpeta se mueve a `lumik/2023/11/04_{nombre}`

#### Scenario: Proyecto con fecha
- **WHEN** el proyecto ya tiene fecha de sesión
- **THEN** ni la fecha ni la carpeta cambian
