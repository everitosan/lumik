# photo-pairs Specification

## Purpose
Trata el RAW y el JPG de una misma toma como una sola foto del proyecto, para que el fotógrafo que dispara RAW+JPEG revise, califique y apruebe cada toma una vez.

## Requirements

### Requirement: Detección del par al importar
Al importar, un archivo RAW y un JPEG SHALL formar un par cuando tienen el mismo nombre de cámara sin extensión (sin distinguir mayúsculas) y la misma fecha de captura al segundo. El par SHALL detectarse entre todos los archivos de la importación, aunque estén en carpetas o tarjetas distintas, y antes de renombrarlos. Un par SHALL registrarse como una sola foto.

#### Scenario: RAW y JPG en la misma tarjeta
- **WHEN** se importan `DSCF1234.RAF` y `DSCF1234.JPG`, capturados el 2026-10-20 a las 18:04:11
- **THEN** el proyecto registra una sola foto con los dos archivos

#### Scenario: Cámara de doble ranura
- **WHEN** en una misma importación se seleccionan `SLOT1/DSCF1234.RAF` y `SLOT2/DSCF1234.JPG` con la misma fecha de captura
- **THEN** se registran como un solo par

#### Scenario: Mismo nombre, otra sesión
- **WHEN** se importan `DSCF0001.RAF` capturado en 2025 y `DSCF0001.JPG` capturado en 2026
- **THEN** no forman par y se registran como dos fotos sueltas

#### Scenario: Archivos sin pareja
- **WHEN** se importan `DSCF0007.RAF` sin JPG y `DSCF0008.JPG` sin RAW
- **THEN** se registran como un RAW suelto y un JPG suelto

#### Scenario: Más de dos archivos con la misma clave
- **WHEN** se importan dos RAW y un JPG con el mismo nombre y la misma fecha de captura
- **THEN** no se forma ningún par y los tres se registran como fotos sueltas

### Requirement: Unión con fotos que ya están en el proyecto
Un JPEG que llega en una importación posterior o en un reintento SHALL unirse a la foto del proyecto que tenga un RAW suelto con su misma clave, en lugar de registrarse como foto nueva, y SHALL guardarse junto a ese RAW. La foto SHALL conservar sus estrellas, color, tags, aprobación y rotación. Lo mismo SHALL aplicarse a un RAW que llega para un JPG suelto.

#### Scenario: Segunda tarjeta en otro import
- **WHEN** el proyecto ya tiene `DSCF1234.RAF` suelto con 4 estrellas y se importa `DSCF1234.JPG` con la misma fecha de captura
- **THEN** la foto pasa a ser un par, sigue con 4 estrellas y el grid no muestra una tarjeta nueva

#### Scenario: JPG que falló y se reintenta
- **WHEN** en una importación el RAW se importó y su JPG falló, y el usuario reintenta los fallidos
- **THEN** el JPG se une a la foto de su RAW

#### Scenario: Aviso antes de importar
- **WHEN** en el paso de origen hay 300 JPG cuyo RAW ya está en el proyecto
- **THEN** el paso de origen indica que esos 300 JPG se unirán a fotos existentes

### Requirement: El par es una sola foto
Un par SHALL ocupar una sola tarjeta en el grid y contar como una foto en los contadores del proyecto. Estrellas, color, tags, aprobación, rotación y papelera SHALL aplicarse al par. Como en cualquier foto, la calificación SHALL guardarse en la base de datos del proyecto, y el sidecar de etiquetas de color para iPad y Mac SHALL escribirse para los dos archivos del par.

#### Scenario: Calificar un par
- **WHEN** con los sidecars para iPad activos, el usuario da 5 estrellas y el color rojo a un par
- **THEN** la foto queda con 5 estrellas y la etiqueta roja, y existen los sidecars `._{nombre}` del RAW y del JPG con la etiqueta roja

#### Scenario: Contador del proyecto
- **WHEN** un proyecto tiene 300 pares, 3 RAW sueltos y 1 JPG suelto
- **THEN** el proyecto muestra 304 fotos

#### Scenario: Rotar un par
- **WHEN** el usuario rota un par 90°
- **THEN** la rotación se aplica a los dos archivos y la miniatura cambia una sola vez

### Requirement: Papelera de un par
Mover un par a la papelera SHALL mover a `_trash/` sus dos archivos y sus sidecars; restaurarlo SHALL devolverlos juntos a donde estaban.

#### Scenario: Mover un par a la papelera
- **WHEN** el usuario mueve a la papelera el par `DSCF1234.RAF` + `DSCF1234.JPG`
- **THEN** los dos archivos quedan en `_trash/` y la foto aparece en la papelera una sola vez

#### Scenario: Restaurar un par
- **WHEN** el usuario restaura ese par
- **THEN** los dos archivos vuelven a su carpeta anterior y la foto vuelve a ser un par

### Requirement: Archivo visible del par
La miniatura y la vista previa de un par SHALL sacarse de su JPG. "Mostrar en carpeta" en el menú de un par SHALL seleccionar su JPG, que es la imagen que muestra el grid, esté donde esté.

#### Scenario: Miniatura de un par
- **WHEN** se importa un par con receta de película en el JPG
- **THEN** la miniatura del grid muestra el JPG, con la receta aplicada

#### Scenario: Mostrar en carpeta un par entregado
- **WHEN** el JPG de un par aprobado ya está en `_exported/` y el usuario elige "Mostrar en carpeta"
- **THEN** el explorador de archivos abre `_exported/` con ese JPG seleccionado
