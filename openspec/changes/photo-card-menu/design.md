# Design

## Context

Ver proposal.md (Why) y los specs `photo-trash`, `photo-viewer` y `photo-filtering`.

Lo que ya existe y condiciona el enfoque:

- `ProjectCard` (`packages/ui`) ya tiene un menú ⋮ en portal, con cierre por clic fuera, `Escape` y scroll, y cada handler opcional agrega su item. Si no llega ningún handler, el botón no se muestra.
- `tauri-plugin-opener` ya es dependencia. `infrastructure/folder.rs` usa `open_path` con un `#[cfg(target_os = "android")]` que devuelve error. El plugin también trae `reveal_item_in_dir`.
- La tabla `photo` ya tiene la columna `deleted`, pero nadie la pone a `1`. `get_project_photos` filtra `deleted = 0`, y de esa consulta salen el grid, el respaldo (`backup.rs`) y el conteo y estado del dashboard (que hace su propio `JOIN ... f.deleted = 0`).
- `CullPhoto` (`application/use_cases/cull_photo.rs`) es el precedente: mueve el archivo con el puerto `FileStore`, mueve el `.xmp` y el sidecar AppleDouble con `FinderTagWriter::move_sidecar`, y si falla la BD revierte el movimiento.
- Todo lo que lee `_exported/` (`sync_exported` al abrir un proyecto, el respaldo de exportados y los conteos) escanea solo `{proyecto}/_exported/`, sin entrar en subcarpetas.
- La portada se guarda como `cover_photo_path = ".thumbs/{photo_id}.jpg"`.
- En proyectos, "Archivar" ya significa ocultar sin cambiar nada más (spec `project-management`) y "Eliminar" es permanente.

## Goals / Non-Goals

**Goals:**
- Mover archivos de forma atómica desde el punto de vista del usuario: o se movió todo y cambió el estado, o no cambió nada.
- Reutilizar los patrones existentes (menú de `ProjectCard`, caso de uso con puertos y rollback, `FilterToggle` en la `FilterBar`) en lugar de crear infraestructura nueva.

**Non-Goals:**
- Sincronizar con el servidor (pendiente, ver proposal).
- Reconciliar el disco con la BD: si alguien mueve archivos a mano dentro o fuera de `_trash/`, Lumik no lo detecta.
- Menú contextual con clic derecho y atajo de teclado para mover a la papelera. Se pueden agregar después sobre los mismos handlers.

## Decisions

### 1. Menú en `PhotoCard` con el patrón de `ProjectCard`
`PhotoCard` recibe handlers opcionales (`onRevealInFolder`, `onTrash`, `onRestore`) y muestra un item por cada uno. El botón ⋮ va a la derecha del header de 29 px, sin cambiar `PHOTO_CARD_HEADER_HEIGHT`, para no tocar el cálculo del grid virtualizado. Los clics en el botón y en el dropdown hacen `stopPropagation`, porque la card entera es `role="button"`.
- La plataforma la decide `ProjectDetail`: en Android no pasa `onRevealInFolder`. Así la UI kit no sabe nada de plataformas, igual que con "Abrir carpeta" en `ProjectCard`.
- Se extrae a un componente compartido (`CardMenu`) la lógica del dropdown que hoy vive dentro de `ProjectCard`, para no duplicar la gestión del portal y del cierre. *Alternativa:* copiarla en `PhotoCard`. Se descarta porque son unas 80 líneas idénticas.

### 2. Revelar con `reveal_item_in_dir`
Nueva función `reveal_file` en `infrastructure/folder.rs`, junto a `open_folder`, con la misma separación por `cfg`. El comando `reveal_photo(project_id, photo_id)` resuelve `mount_point + dng_path` en el backend para que el frontend nunca maneje rutas absolutas. En Linux el plugin usa el D-Bus `org.freedesktop.FileManager1`.
- *Alternativa:* `open_path` sobre la carpeta padre. Se descarta porque no selecciona el archivo, que es justo lo que se pidió.

### 3. Estructura en disco: `_trash/` y `_trash/_exported/`
El original va a `{proyecto}/_trash/{nombre}` y el exportado a `{proyecto}/_trash/_exported/{nombre}`.
- La subcarpeta evita choques cuando el original es JPG y el exportado tiene el mismo nombre.
- Como los escaneos de `_exported/` no son recursivos, el exportado en la papelera no se vuelve a enlazar ni se respalda, sin cambiar esos escaneos.
- La regla de rutas vive en `domain/paths.rs`, como `cull_destination`. Al mover a la papelera, el destino se calcula a partir de la ruta actual. Al restaurar, la carpeta sale de `photo.culled` (`_culled` o `_media`), así que no hace falta guardar el origen en la BD.
- El nombre va en inglés, como `_media`, `_culled` y `_exported`.
- *Alternativa:* `_trash/_media/` y `_trash/_culled/` para no consultar `culled`. Se descarta porque `culled` ya es la fuente de verdad y una carpeta plana es más fácil de revisar a mano.

### 4. Caso de uso `TrashPhoto` / `RestorePhoto` con rollback por pasos
Un caso de uso nuevo en `application/use_cases/`, con los mismos puertos que `CullPhoto` (`PhotoRepository`, `FileStore`, `FinderTagWriter`). Cada movimiento se apila en una lista de hechos. Si falla un movimiento obligatorio o la BD, se deshace la lista en orden inverso (como `rename_project_files`).
- **Obligatorios:** el original y, si existe, el exportado.
- **Mejor esfuerzo:** el `.xmp` y los `._` (igual que en `CullPhoto`).
- Antes de mover nada se comprueba que ningún destino exista. Así se cumple "sin sobrescribir" y se evita el caso en que `rename` reemplaza el archivo en Linux.
- `PhotoRepository` gana `set_deleted(id, deleted, dng_path, jpg_path)`, que actualiza las tres columnas en una sola sentencia. El nombre sigue a la columna `deleted` (ver decisión 8).
- Al mover a la papelera, si `cover_photo_path == ".thumbs/{id}.jpg"`, se pone a `NULL` en la misma operación de BD. Al restaurar no se recupera la portada: el usuario puede volver a elegirla.
- `CullPhoto` rechaza fotos con `deleted = true` antes de calcular el destino.

### 5. Ruta del exportado: confiar en `jpg_path`, con alternativa por convención
`update_photo_paths_prefix` (que se usa al renombrar o mover un proyecto) reescribe `dng_path` pero no `jpg_path`. Para fotos fuera de la papelera da igual, porque `sync_exported` vuelve a enlazar al abrir el proyecto. Para una en la papelera, `jpg_path` quedaría desactualizado y no hay nada que lo corrija. Por eso:
- `update_photo_paths_prefix` también reescribe `jpg_path` con el mismo prefijo, lo que corrige de paso la desalineación actual.
- Al restaurar, el exportado se busca en `_trash/_exported/` del proyecto del original, con el nombre de archivo de `jpg_path`. Así no depende de que `jpg_path` esté al día. Si no está ahí pero `jpg_path` existe en disco (por ejemplo, porque el exportado no se movió a la papelera), se conserva sin moverlo. Si no está en ningún lado, se restaura el original y se limpia `jpg_path` (el próximo `sync_exported` lo enlazará si el JPG aparece).

### 6. Consulta y filtro de papelera
- Nuevo `get_project_trashed_photos` (mismas columnas, `deleted = 1`); el badge del toggle usa el largo de esa lista. `get_project_photos` no cambia, así que el respaldo y el grid normal siguen excluyendo las fotos en la papelera sin tocarlos.
- En `ProjectDetail`, el toggle "Papelera" (un `FilterToggle` en la `FilterBar`) cambia la fuente de la lista. Los demás filtros se aplican igual en el cliente. No se guarda en `project_settings` (vive en el estado React).
- Con el toggle activo, cada card recibe `onRestore` en lugar de `onTrash`. El visor sigue disponible para revisar la foto. El atajo de aprobar lo rechaza el backend (decisión 4) y el frontend no aplica la actualización optimista si `photo.deleted`.

### 7. Modal de confirmación
Un `TrashPhotoModal` en `ProjectDetail`, con la estructura de `DeleteProjectModal` (`Modal` + `Button`, `loading`, `error`) pero sin el aviso rojo de irreversibilidad. El botón de confirmar usa la variante de peligro, y el texto nombra el archivo y dice que se moverá a la papelera del proyecto (`_trash/`). La acción no es optimista: la foto sale del grid cuando el comando responde, porque implica mover archivos y puede fallar. Restaurar no pide confirmación.

### 8. Vocabulario: "papelera", no "eliminar" ni "archivar"
La acción se llama "Mover a la papelera", el filtro "Papelera" y la carpeta `_trash/`. Todo lo que ve el usuario o expone una API (textos, claves i18n, props de `PhotoCard`, comandos Tauri, casos de uso, modal) usa ese vocabulario.
- *"Eliminar"* se descarta porque en proyectos es permanente. Usar la misma palabra para algo reversible daría dos niveles de gravedad con un mismo nombre.
- *"Archivar"* se descarta porque en proyectos significa ocultar sin cambiar nada más, y sugiere conservar. Esta acción mueve archivos y saca la foto del respaldo, que es lo contrario.
- *"Descartar"* se descarta porque se confunde con "no aprobada" (`culled = false`).
- La papelera es la idea que todo el mundo asocia a borrar con vuelta atrás, y deja nombre natural para un futuro "Vaciar papelera".
- La columna `deleted` de la BD no se renombra: ya existe en todas las BD de proyecto y renombrarla exigiría una migración que no aporta nada. `set_deleted` y `photo.deleted` siguen ese nombre interno.

## Risks / Trade-offs

- [El portal y el iPad siguen mostrando la foto que está en la papelera] → Aceptado y documentado como pendiente en el proposal. Los archivos remotos se conservan.
- [`reveal_item_in_dir` en Linux depende de que el gestor de archivos implemente `FileManager1` (Nautilus, Dolphin y Nemo lo hacen). En otros puede no seleccionar el archivo] → Se propaga el error al usuario. Si en las pruebas aparece un gestor sin soporte, se puede añadir un fallback a `open_path` de la carpeta padre sin cambiar el spec.
- [`_trash/` crece sin límite y ocupa disco] → Fuera de alcance. El fotógrafo puede vaciarla a mano porque son archivos normales. Si molesta, se hará un "Vaciar papelera" con confirmación fuerte en otro cambio.
- [Mover entre carpetas del mismo volumen es un `rename` atómico. Si `_trash/` quedara en otro volumen (no pasa, siempre está bajo el proyecto) no lo sería] → Sin mitigación necesaria.
- [Renombrar un proyecto (`rename_project_files`) no renombra los archivos en `_trash/`] → Es coherente con el spec actual, que solo renombra `_media/` y `_culled/`. Una foto restaurada conserva el nombre antiguo.
- [Una `_deleted/` creada durante las pruebas con el nombre anterior no la verá el código nuevo] → Antes de aplicar el cambio de vocabulario, restaurar esas fotos o mover la carpeta a mano a `_trash/`. No hay versión publicada con `_deleted/`, así que no hace falta migración.

## Migration Plan

No hay migración de datos: la columna `deleted` ya existe en todas las BD de proyecto con valor `0`. Para hacer rollback del cambio basta con revertir el código. Las fotos que hubieran quedado en la papelera seguirían ocultas (con `deleted = 1`), con sus archivos en `_trash/`.
