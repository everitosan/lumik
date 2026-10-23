# Design

## Context

Ver proposal.md (Why). Estado actual que condiciona el diseño:

- **Una fila por archivo.** La tabla `photo` del `project.db` guarda el original en `dng_path` (en `_media/` o `_culled/`) y, en `jpg_path`, el JPG final enlazado desde `_exported/`. Ese enlace lo hace catalog-sync al abrir el proyecto, por nombre sin extensión. Hoy un RAW y su JPG de cámara son dos filas.
- **Aprobar** (`application/use_cases/cull_photo.rs`) mueve `dng_path` entre `_media/` y `_culled/`, junto con el sidecar `{stem}.xmp` y el AppleDouble `._{nombre}`, y revierte el movimiento si falla la BD.
- **Import en escritorio** (`import/pipeline.rs`): cada archivo se copia solo a su carpeta de staging. exiftool firma y renombra (`-FileName<…%-03c.%le`), y los choques de nombre se resuelven al mover a `_media/` con `free_name` (sufijo `-001`…). En Android el renombrado y el XMP se hacen archivo por archivo. La fecha de captura solo se lee del EXIF al registrar, después de copiar.
- **Respaldo:** sube solo `_exported/`. El proyecto queda "Respaldado" si `_exported/` tiene al menos tantas fotos como `_culled/`.
- Hay dos changes activos sobre `photo-import` (`import-feedback-and-errors`, `import-retry-failed`). Este change no toca sus requisitos, pero sí el mismo pipeline.

## Goals / Non-Goals

**Goals:**
- Un par es una fila de `photo`. El resto de la app (grid, filtros, contadores, respaldo, estado del proyecto) sigue funcionando sobre filas sin saber de pares, salvo donde se decide qué archivo mostrar o mover.
- El emparejamiento se decide una sola vez, antes de renombrar, y es el mismo en un import, entre imports y en un reintento.

**Non-Goals:**
- Emparejar RAW con TIFF, HEIF o más de un JPG por toma.
- Enlazar exports renombrados por el editor (`DSCF1234_01.jpg`). Siguen respaldándose sin enlazar, como hoy.
- Juntar los pares de proyectos importados antes de este cambio.
- Cambiar el portal web: muestra lo que hay en `_exported/`.

## Decisions

### D1. Modelo de datos: el par es una fila con dos archivos

Columnas nuevas en `photo` (con `DEFAULT`, sin migrar datos):

| Columna | Uso |
|---|---|
| `camera_jpg_path` | Ruta relativa del JPG de cámara de un par (en `_media/`, `_culled/` o `_exported/`). `NULL` si no es par. |
| `delivered_size`, `delivered_sha256` | Huella del JPG entregado. `NULL` si no hay entrega. |
| `delivery_kind` | `NULL`, `moved` (JPG de un par) o `copied` (copia de un JPG suelto). Indica cómo deshacer la entrega. |

- **Qué guarda cada fila.** En un par, `dng_path` es el RAW y `original_format` el del RAW. En un JPG suelto, `dng_path` es el JPG, como hoy.
- **`jpg_path` mantiene su significado:** el JPG final en `_exported/`. Al entregar, `jpg_path` apunta al JPG entregado. Si después el fotógrafo exporta encima, `jpg_path` no cambia: lo que cambia es la huella.
- **Opción de entrega del proyecto:** `delivery_mode TEXT NOT NULL DEFAULT 'none' CHECK(IN ('none','pairs','all_camera_jpg'))`, en `project_settings`.
- **Valor de fábrica global:** se guarda en `settings` de la base global, con `'none'` por defecto.

*Alternativa:* una tabla `photo_file` con N archivos por foto. Es más general (permitiría TIFF+JPG o más archivos por foto), pero obliga a tocar todas las consultas, el portal y el catálogo. Dos columnas cubren el caso real.

### D2. Clave de emparejamiento y momento en que se decide

- **Clave:** `(stem en minúsculas, DateTimeOriginal al segundo)` sobre los nombres de origen, antes de renombrar.
- **Agrupación:** grupos de exactamente un RAW y un JPEG forman par; cualquier otra combinación queda en sueltos.
- **Lectura de la fecha:** se lee al añadir archivos en el paso de origen, con exiftool en lote (`-fast2 -DateTimeOriginal -j`; en Android, rawler). Así el desglose del paso de origen y el del import son el mismo cálculo.
- **Archivos sin fecha:** usan la `ModifyDate` del archivo, como ya hace el renombrado.
- **Unión con el proyecto:** para un archivo suelto se busca en el `project.db` la otra mitad. La consulta exige `camera_jpg_path IS NULL`, `original_format` del tipo contrario, `capture_date` igual al segundo y stem de origen igual.
- **Stem de origen:** los archivos ya están renombrados en el proyecto, así que se guarda una columna nueva `source_stem` (`DSCF1234`) al registrar.

*Alternativa:* emparejar después de copiar, por el nombre final. Se descartó porque el contador de ráfagas (`-001`) se asigna por archivo y puede desfasar el RAW y el JPG de una misma toma.

### D3. Renombrado y choques de nombre por par

- **Escritorio:** exiftool sigue firmando y renombrando cada archivo en staging, y el RAW y el JPG de una toma salen con el mismo nombre base (misma fecha). El cambio está al mover: en lugar de `free_name` por archivo, se usa `free_stem` por grupo. Este busca el primer sufijo (`""`, `-001`…) en el que no exista en `_media/`, `_culled/` ni `_exported/` ningún archivo con ese stem, y mueve los dos archivos con él.
- **Unión con un RAW ya importado:** el JPG toma directamente el stem del RAW del proyecto.
- **Android:** la misma función, con el nombre calculado en `pipeline_metadata`.

### D4. Aprobar y entregar

`CullPhoto` decide el destino a partir de `(tipo de foto, delivery_mode)` (tabla de la spec `camera-jpeg-delivery`).

**Al aprobar con entrega:**

| Tipo | Paso |
|---|---|
| Par | Mueve el RAW (con `{stem}.xmp` y su AppleDouble) a `_culled/` y el JPG (con su AppleDouble) a `_exported/`. |
| JPG suelto | Mueve el JPG a `_culled/` y lo copia a `_exported/`. |

Después calcula SHA-256 y tamaño del JPG entregado y guarda en una sola transacción `culled`, rutas, `jpg_path`, huella, `delivery_kind` y `workflow_status = 'edited'`.

**Al quitar la aprobación:** se compara la huella con el archivo de `_exported/`.

| Caso | Paso |
|---|---|
| Coincide, `moved` | El JPG vuelve junto al RAW. |
| Coincide, `copied` | Se borra la copia. Es el único borrado que hace Lumik, y solo de un archivo que él copió y que sigue igual. |
| No coincide | Se deja en `_exported/`, la foto queda enlazada (`jpg_path`) y sigue `edited`. |

**Rollback:** se mantiene la regla actual de revertir los movimientos si falla la BD. Con varios movimientos, se revierten en orden inverso.

**Si `_exported/` ya tiene un archivo con ese nombre:** no se sobrescribe. El JPG va a `_culled/`, y la foto queda enlazada al exportado existente.

*Alternativa:* entregar siempre por copia (también en pares). Se descartó por duplicar espacio en la mayoría de las fotos; el RAW ya protege al par.

### D5. Cambiar la opción con fotos aprobadas

Un comando calcula el plan antes de aplicar nada: cuántas fotos se entregan, cuántas regresan, cuántas son ediciones del fotógrafo y cuántas tienen `backup_status = 'uploaded'`. Con ese plan, la interfaz muestra el aviso. Si se acepta, se ejecuta foto por foto con las mismas operaciones de D4. Un fallo en una foto se informa y no detiene las demás, igual que en el import.

### D6. Unión de un JPG a una foto aprobada

Al registrar un JPG que se une a una foto con `culled = 1`, el import lo deja en `_culled/` junto al RAW. Después ejecuta el paso de entrega de D4 para esa foto con el `delivery_mode` vigente. Así hay un solo camino de entrega.

### D7. Escritura de metadatos en el par

- **Estrellas, color y tags:** se escriben en los dos archivos con la misma cola de escritura que hoy, dos destinos por foto.
- **Rotación:** se escribe en los dos y la miniatura se regenera una vez.
- **AppleDouble (Finder tags):** se genera para los dos archivos.
- **Si falla en uno de los dos:** se reintenta como hoy, sin bloquear la interfaz.

### D8. Vistas previas y visor

- **Miniatura y `.previews/{id}.jpg`:** salen del JPG del par, que ya es JPEG, así que no hace falta extraer nada.
- **Vista del RAW en el visor:** se cachea aparte en `.previews/{id}-raw.jpg`, la primera vez que se pide.
- **El conmutador del indicador** es estado de la interfaz: no se guarda y vuelve al JPG al cambiar de foto.

## Risks / Trade-offs

- [Leer la fecha de captura en el paso de origen agrega espera con cientos de archivos en un disco mecánico] → exiftool en lote con `-fast2` (solo cabeceras), en segundo plano. El desglose aparece cuando termina y no bloquea seguir al paso de ajustes.
- [Dos fotos sueltas que deberían ser par por un reloj desajustado entre RAW y JPG (no pasa en una misma cámara)] → Se aceptan como sueltas; el indicador `RAW`/`JPG` lo hace visible.
- [El hash al entregar agrega lectura de cada JPG (pocos MB)] → Se calcula al mover. Es despreciable frente al import.
- [Escribir metadatos en dos archivos duplica el trabajo de la cola] → Los JPG se escriben rápido. La cola ya es asíncrona.
- [Borrar la copia de un JPG suelto rompe "nunca borrar"] → Solo se borra si coincide la huella con lo que Lumik copió, y el original sigue en el proyecto. La regla queda acotada en la spec de `photo-culling`.
- [Conflicto con los changes activos sobre el pipeline del import] → Implementar después de archivar `import-feedback-and-errors` e `import-retry-failed`, o rebasar sobre ellos. El reintento usa la misma unión de D2.

## Migration Plan

- Sin migración de datos. Las columnas nuevas llevan `DEFAULT` y se crean con la migración aditiva existente (`CREATE TABLE IF NOT EXISTS` / `ALTER TABLE ADD COLUMN`).
- Los proyectos existentes quedan con `delivery_mode = 'none'` y se comportan como hoy.
- Revertir es volver a la versión anterior. Las columnas nuevas se ignoran; los pares ya importados se verían como una foto (el RAW) y su JPG quedaría en disco sin registrar.

## Open Questions

- Atajo de teclado para cambiar entre JPG y RAW en el visor (¿`r`?). Se decide al implementar, junto con los demás atajos configurables.
