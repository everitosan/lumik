# Design

## Context

Ver proposal.md (Why) y los specs `trash-bin`, `project-management`, `photo-trash`, `photo-filtering` y `photo-search`.

Estado actual que condiciona el enfoque:

- **Desktop**: `delete_project` (`apps/desktop/src-tauri/src/commands/project.rs`) libera sesiones, quita el proyecto del índice y del registro de proyectos abiertos, y hace `remove_dir_all`. `relocate_project_folder` ya resuelve "cerrar el SQLite, `rename` de la carpeta, quitar las carpetas vacías de arriba y reabrir", que es el mismo movimiento que necesita la papelera.
- **Escaneo**: `discover_projects_on_device` (`crates/lumik-core/src/db/mod.rs`) recorre `lumik/` hasta 4 niveles y ya se salta las carpetas que empiezan con `.` o `_`, así que `lumik/_trash/` queda fuera del dashboard del desktop sin cambios. El `walk` de la tablet (`apps/tablet/src/projects.ts`) solo se salta las que empiezan con `.`: entraría a `lumik/_trash/`.
- **Fotos**: `TrashPhoto`/`RestorePhoto` (`crates/lumik-core/src/application/use_cases/trash_photo.rs`) mueven original, `.xmp`, sidecar `._`, par y exportado con rollback. La tablet usa los mismos casos de uso sobre su copia espejada de `project.db` (`crates/lumik-mobile/src/disk.rs`). `get_project_trashed_photos` y `open_project_trash` ya listan las fotos con `deleted = 1`.
- **Base del proyecto**: solo `photo` y `photo_embedding` cuelgan de una foto. Miniatura y preview van por id en `.thumbs/` y `.previews/`.
- **Tablet**: cada acción del menú de proyecto usa `withProjectDb` (trae la base, toma el candado, la devuelve), con `force` para tomar el control. Las tarjetas del inicio ya guardan una copia de cada `project.db` en `covers/db/<key>/`.
- `project.deleted` existe en el esquema y el dashboard lo filtra, pero nada lo pone en 1.

## Goals / Non-Goals

**Goals:**
- Una sola fuente de verdad para "está en la papelera" que viaje con el disco y que un explorador de archivos muestre tal cual.
- Que desktop y tablet compartan las reglas de rutas y de borrado desde `lumik-core`.
- Que eliminar definitivamente sea idempotente: si se interrumpe, repetirlo termina el trabajo.

**Non-Goals:**
- Mostrar cuánto espacio ocupa la papelera.
- Reconciliar archivos que el usuario mueva a mano dentro o fuera de `_trash/`. Se respeta lo que haya en disco.

## Decisions

### 1. La ubicación de la carpeta es el estado del proyecto
Un proyecto está en la papelera si y solo si su carpeta está bajo `lumik/_trash/`. La fecha en que se mandó y su ruta original salen de la ruta: `lumik/_trash/{AAAA-MM-DD_HH-MM-SS}/{ruta}`, con la hora local del dispositivo y guiones porque exFAT no acepta `:`. `project.deleted` no se usa.

La carpeta del proyecto conserva su nombre, así que mandarla y restaurarla es un solo movimiento. En la tablet, cada operación de SAF sobre una carpeta tarda según cuántos archivos tiene: con el disco real, un proyecto de más de 1 000 fotos tardó 18.6 s en moverse y otros 18.6 s en renombrarse.

- *Alternativa (formato anterior):* `lumik/_trash/{ruta}__{fecha}`, con la fecha como sufijo del nombre. Se descarta por el doble movimiento en la tablet. Lo que ya esté en la papelera con ese formato se sigue listando y restaurando.
- *Alternativa:* un `.nomedia` en `lumik/` para que Android no indexe las fotos. Se descarta: no está probado que el índice sea la causa, y las fotos dejarían de verse en las galerías.

- *Alternativa:* marcar `project.deleted = 1` (o un `trashed_at`) y dejar la carpeta donde está. Se descarta: el proyecto seguiría viéndose vivo en `_media/`, y cada lector (escaneo del desktop, tablet, respaldo, versiones viejas de la app) tendría que respetar la bandera. Con la carpeta movida, cualquier versión del desktop ya lo ignora por el prefijo `_`.
- *Alternativa:* guardar la fecha también dentro de `project.db`. Se descarta para no tener dos fuentes de verdad. Si alguien renombra la carpeta a mano, manda el nombre.

Las reglas van en `domain/paths.rs`, junto a `trash_destination`:
- `project_trash_destination(rel_bajo_lumik, ahora)` agrega `_trash/{fecha}/` al inicio.
- `project_restore_destination(rel_en_trash)` quita `_trash/{fecha}/`. Del formato anterior, quita `_trash/` y un sufijo `__AAAA-MM-DD_HH-MM-SS` válido al final. Si no hay fecha reconocible, solo quita `_trash/`.
- `parse_trashed_at(rel_en_trash)` devuelve la fecha, de cualquiera de los dos formatos, para mostrarla y ordenar; o nada si no la hay.

### 2. Listar la papelera de un disco sin registrar nada
Un caso de uso `ListTrash` por disco montado devuelve:
- **Proyectos**: se recorre `lumik/_trash/` con la misma regla de profundidad que el escaneo. De cada `project.db` encontrado se lee nombre, fecha de sesión, número de fotos y portada, y se cierra. No entra al registro de proyectos abiertos, para que álbumes, respaldo, búsqueda y sesiones no lo vean.
- **Fotos sueltas**: las de los proyectos activos de ese disco que ya están abiertos (`get_project_trashed_photos`). Las fotos de proyectos en la papelera nunca se consultan, así que no se listan aparte.

El conteo del menú es la suma de todos los discos montados. Se recalcula al cambiar los dispositivos y después de cada acción de papelera. Nada se guarda en la base global.

- *Alternativa:* abrir los proyectos de la papelera con el `ProjectDatabase` normal y meterlos al registro con una marca. Se descarta: cualquier consumidor del registro podría tratarlos como activos.

### 3. Mandar un proyecto a la papelera reutiliza el movimiento de `relocate_project_folder`
Nuevo comando `trash_project`, que reemplaza a `delete_project`. Hace lo mismo que `delete_project` antes del borrado (`ensure_not_locked`, liberar sesiones con su candado, quitar del índice y del registro, cerrar el SQLite) y además quita las entradas pendientes de `backup_queue` de ese proyecto. Luego hace el `rename` a la ruta de la decisión 1 y limpia las carpetas vacías que quedan arriba, sin pasar de `lumik/`. Si el `rename` falla, el proyecto se reabre en su lugar, como en `relocate_project_folder`.

Se extrae de `relocate_project_folder` un helper "cerrar, mover, limpiar ancestros" que comparten los dos.

### 4. Restaurar un proyecto
`restore_project(device, rel_en_trash, new_name?)`:
1. Calcula el destino con `project_restore_destination`, o, si viene `new_name`, la carpeta que le tocaría a ese nombre con la fecha de sesión del proyecto (la misma regla que crear o editar un proyecto).
2. Si el destino existe, responde un error tipado de colisión con la ruta ocupada, sin mover nada. El frontend abre el modal con ese error.
3. Hace el `rename`, reabre, registra el proyecto y lo reindexa en la búsqueda.
4. Con `new_name`, aplica el mismo caso de uso que al editar el nombre: actualiza `project.name` y `RenameProjectFiles`. Esto pasa después de mover, con el proyecto ya abierto, para reusar el flujo de edición tal cual.

El modal valida en vivo con un comando de solo lectura `check_restore_target(device, rel_en_trash, new_name)`, que dice si el destino está libre, para no habilitar "Restaurar" con un nombre ocupado.

### 5. Restaurar una foto con otro nombre
`RestorePhoto` recibe un `new_stem: Option<String>` opcional. Antes de mover cualquier archivo calcula todos los destinos: original, `.xmp`, `._`, el JPG del par y el exportado, cada uno con su extensión. Si alguno está ocupado, devuelve un error de colisión que nombra el primero, y no mueve nada. Con `new_stem`, los destinos usan ese nombre y la foto actualiza `dng_path`, `camera_jpg_path` y `jpg_path` en la misma escritura que hoy limpia `deleted`. Así el siguiente `sync_exported` sigue enlazando el exportado, porque comparte el nombre base.

El rollback por pasos actual no cambia; solo cambian los destinos.

### 6. Eliminar definitivamente: archivos primero, luego registros
Nuevo caso de uso `PurgePhoto`, que solo acepta fotos con `deleted = 1` y rechaza cualquier otra:
1. Borra del disco, ignorando los que ya no existen: original, `.xmp`, `._`, el JPG del par con sus sidecars, el exportado en `_trash/_exported/` con su `._`, `.thumbs/{id}.jpg` y la preview `.previews/{id}.*`.
2. En una transacción, borra las filas de `photo_embedding` y la de `photo`.
3. Refresca el índice del proyecto. Las fotos en la papelera ya no estaban en él, pero así se sube la generación.

Archivos primero porque, si falla a la mitad, la foto sigue en la papelera y repetir la acción termina el trabajo (cubre el escenario "Archivo que ya no está"). Al revés quedarían archivos huérfanos en `_trash/` que ninguna pantalla muestra, que es justo la huella que no se quiere.

`FileStore::remove_file` deja de ser "solo para la entrega" y pasa a ser un borrado general de un archivo. Se implementa en `infrastructure/fs.rs` y en `DiskFiles` de la tablet.

`purge_project(device, rel_en_trash)` primero verifica que la ruta resuelta quede bajo `{mount}/lumik/_trash/`: es la guarda contra borrar un proyecto activo. Luego hace `remove_dir_all`, quita las carpetas vacías de arriba sin pasar de `lumik/_trash/`, y lo vuelve a quitar del índice como medida defensiva.

### 7. Vaciar papelera: secuencial, con reporte
`empty_trash(device)` elimina primero los proyectos en la papelera y después las fotos sueltas, uno por uno, y junta los fallos en `{ eliminados, fallidos: [{ elemento, razón }] }`. Cada foto de un proyecto activo usa la sesión de escritura del proyecto. Si el proyecto lo tiene otro dispositivo, esa foto falla con la razón del candado y se sigue con las demás. La papelera de un proyecto que está en la papelera no tiene candado que respetar: nadie puede abrirlo.

### 8. Candados
- `trash_project`, `restore_photo`, `purge_photo` y el restaurar con otro nombre de `restore_project` usan `write_session`/`ensure_not_locked`, igual que hoy `trash_photo`.
- Los proyectos dentro de `_trash/` no se abren para trabajar. Si traen un `.lumik-lock` viejo, se ignora al eliminarlos y se quita al restaurarlos, antes de reabrir.

### 9. UI del desktop
- `Sidebar.tsx`: `SectionButton` con el icono `trash` (ya existe en `@lumik/ui`) justo antes de Configuración, con el conteo. Se renderiza solo si el conteo es mayor que cero. Nueva sección `'trash'` en el `Layout`. Si el conteo llega a cero con `'trash'` activa, el `Layout` cambia a `'projects'`.
- El conteo sale de un hook `useTrashCount` que llama a `list_trash` de los discos montados al arrancar, al cambiar los dispositivos y después de cada acción que manda a la papelera, restaura o elimina, también desde `ProjectDetail` y el dashboard. Así la entrada aparece con la primera foto mandada a la papelera.
- Nueva página `Trash`: un bloque por disco con un encabezado (nombre del disco y "Vaciar papelera de {disco}"), una fila de `ProjectCard` y un grid de `PhotoCard`. Las cards reciben `onRestore` y `onPurge` en su menú, sin `onTrash`. Las tarjetas de proyecto muestran la fecha en que se mandaron.
- Modales: `TrashProjectModal` (reemplaza `DeleteProjectModal`; texto neutro y botón de peligro, como `TrashPhotoModal`), `PurgeModal` (proyecto o foto, con el aviso rojo de irreversibilidad y la nota del respaldo), `EmptyTrashModal` (con conteos) y `RestoreCollisionModal` (campo de nombre con `check_restore_target`).
- `ProjectDetail`: sale el `FilterToggle` "Papelera" y la fuente de lista de la papelera. `TrashPhotoModal` cambia su texto a "desde la sección Papelera".

### 10. Tablet
- `projects.ts`: `walk` se salta las carpetas que empiezan con `_`, como el desktop.
- Nuevo `trash.ts`: por cada árbol, recorre `lumik/_trash/` con el mismo `walk` para los proyectos y lee sus tarjetas con `projectCard` sobre una copia. Para las fotos sueltas usa `open_project_trash` sobre las copias de `project.db` que ya trae `covers.ts`.
- Mover a la papelera y restaurar un proyecto: un solo `Saf.move` (con `Saf.rename` solo al restaurar con otro nombre o desde el formato anterior), con la misma limpieza de carpetas vacías que `relocate`. Las rutas vienen de las funciones de `paths.rs`, expuestas por `lumik-mobile`.
- `MoveSheet.tsx`: la misma hoja para mandar y restaurar. Al mandar, confirma con poco texto y dos íconos; mientras se mueve, una carpeta salta de un ícono al otro en bucle (Reanimated; quieta con movimiento reducido) y se ven los segundos transcurridos; al terminar, una marca de hecho y se cierra sola. Restaurar abre la hoja ya moviendo, en sentido inverso. Un candado o una colisión cierran la hoja y abren su propio diálogo.
- Restaurar con otro nombre: después del movimiento, `editProjectDetails`, como al editar.
- Fotos: restaurar y eliminar definitivamente con `withProjectDb` (candado y `force`) y casos de uso del núcleo expuestos por `lumik-mobile` (`restore_photo` con `new_stem`, `purge_photo`) sobre `DiskFiles`.
- Eliminar un proyecto definitivamente: `Saf.delete` de su carpeta, solo si su URI está bajo `lumik/_trash/`.
- `HomeScreen.tsx`: "Papelera" en el `RAIL` justo antes de "Ajustes", solo si el conteo es mayor que cero. El conteo se recalcula con la misma regla que en el desktop. Si llega a cero con la sección abierta, vuelve a "Proyectos". Sale la vista de papelera de `ProjectScreen`/`AppScreen` (`trashGrid`, `fromTrash`, `onRestore`).

## Risks / Trade-offs

- [La papelera no libera espacio hasta vaciarse] → La sección muestra lo que hay por disco, y "Vaciar papelera de {disco}" queda a un toque. El respaldo en la nube nunca se toca.
- [Una tablet con un build anterior lista los proyectos de `lumik/_trash/` como activos] → La app de tablet sigue en desarrollo, sin beta pública. Conviene que su primera beta ya incluya este cambio; si no, se anota en las notas del release.
- [Desktop y tablet eliminan lo mismo a la vez] → Todo borrado tolera archivos y carpetas que ya no existen; el segundo termina sin error.
- [`Saf.move` en un proveedor que copia en vez de mover haría lento mandar un proyecto grande a la papelera] → `relocate` ya depende de lo mismo al cambiar la fecha de un proyecto. Hay que medirlo con el disco real en la tablet (tarea de prueba manual).
- [Sacar a mano la carpeta del proyecto de su carpeta de fecha dentro de `_trash/`] → Se restaura a la ruta sin `_trash/`, y la fecha se muestra como desconocida. No se pierde nada.
- [Tomar el control (`force`) en la tablet para eliminar una foto de un proyecto abierto en el desktop] → Es el mismo comportamiento que ya tienen las demás ediciones desde el menú de la tablet; el desktop ve que perdió el candado como hoy.

## Migration Plan

- No hay migración de datos: los proyectos existentes no están en la papelera, y las fotos con `deleted = 1` aparecen solas en la sección nueva.
- Si hay que revertir el release, las carpetas en `lumik/_trash/` quedan invisibles para el desktop viejo, que ya se salta el prefijo `_`. Se recuperan moviéndolas a mano a `lumik/`, o con el release nuevo.
- Actualizar `docs/useCases/flujo-papelera-foto.md` y agregar el flujo de papelera de proyecto, como pide la regla de diagramas de casos de uso.
