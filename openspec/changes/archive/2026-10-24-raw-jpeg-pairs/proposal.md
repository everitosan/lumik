# Proposal

## Why

Hay fotógrafos que disparan RAW+JPEG con recetas de cámara (por ejemplo, simulaciones de película de Fuji) y entregan el JPG tal cual: el revelado ya lo hizo la cámara y el RAW solo se guarda por si acaso. Hoy Lumik trata el RAW y el JPG de una toma como dos fotos sueltas: hay que revisar cada toma dos veces, aprobar una no mueve la otra, el estado "Respaldado" no cuadra nunca y el JPG aprobado no llega a `_exported/` (lo único que se respalda) sin pasar por un editor. Es buen momento: Lumik aún no tiene usuarios activos fuera del autor, así que no hace falta migrar proyectos.

## What Changes

- Al importar, un RAW y un JPG de la misma toma (mismo nombre de cámara sin extensión y misma fecha de captura) se registran como **una sola foto, un par**, aunque vengan de carpetas o tarjetas distintas (cámaras de doble ranura) o en imports separados. Un JPG que llega después se une a la foto de su RAW y hereda su calificación y su aprobación.
- Estrellas, color, tags, aprobación, rotación y papelera son del par. La calificación sigue guardándose en la base de datos del proyecto, y el sidecar de color para iPad se escribe para los dos archivos.
- El grid y el visor muestran un **indicador de formato** (`RAW+JPG`, `RAW`, `JPG`). Para un par, la miniatura y la vista previa salen del JPG, y en el visor el indicador cambia entre el JPG y el RAW.
- Nueva opción **"Al aprobar, entregar directamente"** con tres valores: *nada* (como hoy), *el JPG de los pares RAW+JPG* o *todo JPG de cámara* (pares y JPG sueltos). Tiene un valor global de fábrica para proyectos nuevos, y cada proyecto guarda el suyo, editable en "Editar proyecto" y en el paso de ajustes del import.
- Al aprobar con entrega activa, el RAW va a `_culled/` y el JPG del par se mueve a `_exported/`, enlazado a la foto y listo para respaldar; un JPG suelto se queda en `_culled/` y se entrega una copia, para no exponer su único original a una sobrescritura. Lumik anota una huella del JPG entregado. Al quitar la aprobación, o al cambiar la opción hacia *nada*, el JPG regresa solo si su huella coincide: una edición del fotógrafo nunca se mueve. Nada se borra de la nube.
- Al cambiar la opción en un proyecto con fotos aprobadas, un aviso ofrece aplicarla también a ellas, en los dos sentidos.
- El import cuenta tomas, no archivos: el paso de origen desglosa pares, RAW sueltos y JPG sueltos, e indica cuántos JPG se unirán a fotos que ya están en el proyecto. El renombrado da el mismo nombre a los dos archivos del par.
- **BREAKING** (sin migración): los proyectos importados antes de este cambio con RAW y JPG conservan sus fotos sueltas.

## Capabilities

### New Capabilities
- `photo-pairs`: el par RAW+JPG como una sola foto: cómo se detecta (en un import, entre fuentes y contra el proyecto), qué archivo se muestra y qué acciones afectan a los dos archivos.
- `camera-jpeg-delivery`: entregar el JPG de cámara al aprobar: la opción de tres valores, a dónde va cada archivo, la huella del JPG entregado, el regreso seguro y el aviso al cambiar la opción.

### Modified Capabilities
- `photo-culling`: aprobar ya no siempre manda el archivo a `_culled/`; depende de la opción de entrega del proyecto y del tipo de foto.
- `photo-import`: el paso de ajustes separa lo que aplica a la importación de lo que es del proyecto; el renombrado da el mismo nombre al par; el import cuenta tomas.
- `photo-viewer`: indicador de formato en el grid y en el visor, y vista del JPG o del RAW en un par.
- `app-settings`: nuevo ajuste global con el valor de fábrica de la entrega al aprobar.
- `project-management`: "Editar proyecto" incluye la opción de entrega del proyecto.
- `cloud-backup`: al recomprimir, el servidor gira los píxeles según la orientación y conserva los metadatos del original; los JPG de cámara verticales llegaban girados y sin firma.

## Impact

- Desktop (Rust): `import/pipeline.rs` (detección de pares antes del renombrado, unión con fotos existentes, reintentos), `application/use_cases/cull_photo.rs` (destinos según la opción, huella, regreso), escritura de metadatos en los dos archivos del par, papelera y rotación del par, enlace de `_exported/` en catalog-sync.
- Base de datos del proyecto (`project.db`): la foto guarda la ruta del JPG de cámara además del original, la huella del JPG entregado y la opción de entrega en `project_settings`; la base global guarda el valor de fábrica. Las columnas nuevas llevan `DEFAULT`, así que los `project.db` existentes siguen abriéndose.
- Frontend: wizard de import (origen, ajustes, resumen), grid, visor, Editar proyecto, Ajustes, textos `es`/`en`.
- Respaldo: sigue subiendo `_exported/` y el estado "Respaldado" pasa a cuadrar para proyectos con pares. En el servidor, la recompresión aplica la orientación EXIF a los píxeles y copia EXIF, XMP, IPTC e ICC al JPEG nuevo.
- Changes activos que también tocan `photo-import` (`import-feedback-and-errors`, `import-retry-failed`): este change no modifica sus requisitos; el reintento reúne el JPG con su RAW por la misma regla de emparejamiento.
