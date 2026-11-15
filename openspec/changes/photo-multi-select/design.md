# Design

## Context

Ver proposal.md (Why) y los specs `photo-marking`, `photo-trash`, `photo-albums` y `trash-bin`.

Estado actual que condiciona el enfoque:

- **Desktop**: `ProjectDetail` arma `sortedPhotos` (filtros + orden) y lo pinta en un grid virtual. El clic de la tarjeta abre el visor (`setSelectedPhotoId`). Los atajos del proyecto viven en un `keydown` de `document`; `project.back` es `Escape`.
- **`PhotoCard`** recibe `onClick?: () => void`, sin el evento, así que hoy no puede distinguir Ctrl/Cmd/Shift. `ProjectCard` está igual.
- **Álbumes**: `AddToAlbumDialog` ya recibe `AlbumTarget = { items: NewAlbumItem[] }` y `add_to_album` agrega una lista y devuelve cuántas agregó. Para varias fotos basta con pasarle más elementos.
- **Papelera**: `trash_photo`, `restore_photo` (con `new_stem`), `purge_photo`, `restore_project` y `purge_project` actúan sobre un solo elemento (change `trash-section`). `restore_*` falla con `restore_collision:{nombre}` si el lugar está ocupado.
- **Tablet**: `ProjectScreen` usa el toque largo para abrir el menú de la tarjeta, que también tiene su botón ⋮. Cada acción de la sesión (`session.trashPhoto`) recarga las listas y devuelve la base al disco. En el inicio, cada acción sobre fotos de la papelera trae y devuelve `project.db` (`withProjectDb`). La tablet no tiene álbumes.

## Goals / Non-Goals

**Goals:**
- Un solo modelo de marcas que comparten el grid del proyecto y la sección "Papelera", en el desktop y en la tablet.
- Lotes que nunca dejan a medias el estado de una foto: cada elemento se mueve con su caso de uso de siempre (con su rollback) y el lote solo los encadena.
- En la tablet, una sola ida y vuelta de `project.db` por proyecto y por lote, no una por foto.

**Non-Goals:**
- Un caso de uso de lote en el núcleo con rollback de todo el lote: si la foto 7 de 12 falla, las 6 anteriores quedan en la papelera (spec: se informa y se sigue).
- Barra de progreso por foto. Los lotes son de decenas de fotos y cada movimiento es un `rename` en el mismo disco.

## Decisions

### 1. Marcas como estado de la vista, por id
Las marcas son un `Set<string>` de ids más un ancla (el último id marcado con clic o con Ctrl/Cmd+clic), en el estado de `ProjectDetail`, de `TrashPage` y de sus pares de la tablet. No van a la base ni a `project_settings`.

- El rango de Shift+clic se calcula sobre la lista visible en orden (`sortedPhotos`), del ancla a la foto pulsada, y se suma a lo que ya estaba marcado. Si el ancla ya no está visible, Shift+clic marca solo la foto pulsada.
- Las marcas se vacían con un `useEffect` sobre los filtros, el orden y el modo de vista, al cambiar `projectId`, y después de cada acción de la barra. Así, una marca nunca queda en una foto que no se ve.
- En la papelera, el id de un elemento es `photo:{project_id}:{photo_id}` o `project:{device_uuid}:{rel_path}`. El rango de Shift+clic se calcula dentro de la lista del disco (proyectos primero, después fotos, como se pintan).

*Alternativa:* guardar las marcas en un contexto global para que sobrevivan al cambiar de vista. Se descarta porque la spec pide limpiarlas al cambiar de proyecto o de sección.

### 2. `PhotoCard` y `ProjectCard`: estado "marcada" y clic con el evento
- `onClick` pasa a recibir el evento (`(e: MouseEvent | KeyboardEvent) => void`). Los usos actuales siguen compilando porque ignoran el argumento.
- Prop nueva `marked?: boolean`: borde de color primario y casilla marcada.
- Prop nueva `onToggleMark?: () => void`: si viene, la tarjeta muestra la casilla al pasar el ratón (y siempre que esté marcada). Pulsarla hace `stopPropagation`, como el botón ⋮, para no abrir el visor.
- El componente no decide qué hace un clic: `ProjectDetail` mira `e.ctrlKey || e.metaKey` (con `hasPlatformModifier` de `hooks.ts`), `e.shiftKey` y si ya hay marcas, y elige entre marcar, marcar un rango o abrir el visor.

### 3. Esc: primero las marcas
En el `keydown` de `ProjectDetail`, si `project.back` coincide y hay marcas, se vacían y se hace `return` antes de regresar. `TrashPage` tiene su propio `keydown` que solo limpia marcas. El visor tiene su `photo_detail.close` aparte y no cambia.

### 4. Barra de acciones de la página
Una barra fija abajo del grid (encima del `ProjectDetailFooter`), solo con marcas. Es exclusiva de cada página, así que vive en su carpeta (`ProjectDetail/MarkedBar.tsx` y la de `Trash/`), por la regla de componentes del desktop.

- **Agregar a álbum**: `setAlbumTarget({ items: marcadas.map(...) })`, con el mismo `NewAlbumItem` que arma hoy el menú de una foto. Al cerrar el diálogo con éxito se vacían las marcas y se muestra "Se agregaron N" con el número que devuelve `add_to_album`.
- **Mover a la papelera**: `TrashPhotosModal` (como `TrashPhotoModal`, pero con el conteo) → `trash_photos`.
- En solo lectura (`readOnly`), la barra no ofrece "Mover a la papelera".

### 5. Comandos de lote en el desktop: un reporte por elemento
```rust
struct BatchFailure { id: String, name: String, reason: String }
struct BatchReport { done: Vec<String>, failed: Vec<BatchFailure> }
```
- `trash_photos(project_id, photo_ids) -> BatchReport`: una `write_session` para todo el lote (un solo candado y una sola subida de generación) y `TrashPhoto::execute` por foto.
- `restore_trash_items(items) -> BatchReport` y `purge_trash_items(items) -> BatchReport`, con `items: Vec<TrashItemRef>`, donde `TrashItemRef = Photo { project_id, photo_id } | Project { device_uuid, rel_path }` (serde con `kind`). Las fotos se agrupan por proyecto para abrir una `write_session` por proyecto. Los proyectos usan `restore_project` y `purge_trashed_project` sin `new_name`.
- En restaurar, un error que empieza con `restore_collision:` se reporta con `reason = "collision"`, y el frontend lo traduce a "su lugar está ocupado". Un candado ajeno llega como `project_locked:{dispositivo}` y se traduce con `projectLockHolder`.
- Los comandos de un elemento (`trash_photo`, `restore_photo`, `purge_photo`, …) no cambian; los lotes reutilizan sus funciones internas.

*Alternativa:* que el frontend llame N veces al comando de un elemento. Se descarta: serían N sesiones (N candados y N subidas de generación), N eventos `lumik:trash-changed` y N recálculos de `list_trash`.

### 6. Reporte al usuario
Al terminar un lote con fallos, la página muestra un aviso con "N no se pudieron …" y la lista de nombres con su motivo, con el mismo estilo del reporte de `EmptyTrashModal`. Sin fallos, se vacían las marcas y la lista se recarga sola (evento `lumik:trash-changed`).

### 7. Tablet
- **`ProjectScreen`**: un estado `marking: Set<number>` (índices en `photos`, como el resto de la pantalla). El toque largo entra al modo marcar con esa foto (deja de abrir el menú; el menú sigue en ⋮). En el modo, el toque marca o desmarca, y "Listo" sale. En solo lectura no se entra al modo, porque la barra de la tablet no tendría acciones.
- **Barra** abajo, en lugar del pie de contadores mientras dura el modo: "N fotos marcadas", "Mover a la papelera" (con confirmación con el conteo) y "Listo".
- **`session.trashPhotos(list)`**: `TrashPhoto` por foto sobre la misma copia de trabajo, y una sola `reload()` y `giveBack()` al final. Devuelve las fallidas.
- **`TrashScreen`**: el mismo modo marcar sobre proyectos y fotos, con barra "Restaurar" / "Eliminar definitivamente". En `projectActions.ts`, `restoreTrashedPhotos(p, ids)` y `purgeTrashedPhotos(p, ids)` hacen un solo `withProjectDb` por proyecto con el bucle dentro. `AppScreen` agrupa las fotos por proyecto y junta los fallos.

### 8. Vocabulario
"Marcar" / "fotos marcadas" / "elementos marcados" en la UI y en el código (`marked`, `onToggleMark`, `MarkedBar`), para no chocar con `culled` / "aprobada" / "selección". Las claves i18n van bajo `projectDetail.marked.*` y `trash.marked.*`.

## Risks / Trade-offs

- [Ctrl+clic en macOS es clic secundario] → Se usa `hasPlatformModifier`: Cmd en macOS, Ctrl en las demás, igual que los atajos.
- [Un lote largo bloquea la UI del desktop] → Los comandos de lote corren en `spawn_blocking`, como `edit_project`, y la barra muestra "Moviendo…" mientras tanto.
- [En la tablet, el toque largo cambia de significado] → El menú sigue en el botón ⋮ de cada tarjeta, que ya existe. Se anota en la guía de gestos si menciona el toque largo.
- [Marcar fotos en un grid virtual] → Las marcas van por id, no por nodo DOM, así que una tarjeta que sale y vuelve a entrar al viewport se pinta marcada.
- [El orden de archivado con `trash-section`] → Esta delta agrega requisitos a `trash-bin`, que crea `trash-section`. Archivar `trash-section` primero (lo dice la propuesta y lo repite `tasks.md`).

## Migration Plan

Sin migración de datos: las marcas no se guardan. Si se revierte, desaparecen la barra y los comandos de lote, y las acciones de una foto siguen igual.
