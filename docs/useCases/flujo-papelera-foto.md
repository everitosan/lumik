# Flujo: Papelera y restaurar foto

Lumik Desktop · Linux · Windows · Android

Mover una foto a la papelera es un **borrado lógico**: la fila queda con `deleted = 1` y sus
archivos se apartan dentro del proyecto, sin borrar nada. El original va a `_trash/` y su JPG
exportado a `_trash/_exported/` (así un original JPG y su exportado con el mismo nombre no se
pisan, y los escaneos de `_exported/` no lo vuelven a enlazar ni a respaldar). Restaurar hace lo
inverso.

Igual que en el [culling](flujo-culling.md), los archivos se mueven **antes** del `UPDATE`, y si
algo falla se devuelven a su sitio.

> La papelera no se propaga todavía al catálogo del servidor: el portal web y el iPad siguen
> mostrando la foto. Las copias ya respaldadas se conservan.

## Vista general

```mermaid
flowchart LR
    A["Menú de la card<br/>'Mover a la papelera' → modal"]
    B["invoke<br/>'trash_photo'"]
    C["fs::rename<br/>→ _trash/<br/>→ _trash/_exported/"]
    D["UPDATE photo<br/>deleted + rutas<br/>(+ quitar portada)"]
    E["Grid sin la foto<br/>filtro 'Papelera' +1"]

    A --> B --> C --> D --> E

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    class A,E ui
    class B ipc
    class C be
    class D db
```

## 1. Disparadores en la UI

```mermaid
flowchart TD
    start(["Usuario en el grid<br/>del proyecto"])
    menu["PhotoCard — menú ⋮<br/>• Mostrar en carpeta (no en Android)<br/>• Mover a la papelera (vista normal)<br/>• Restaurar (filtro 'Papelera')"]
    reveal["invoke('reveal_photo')<br/>explorador del SO con el archivo seleccionado"]
    modal["TrashPhotoModal<br/>'se moverá a la papelera (_trash/) · podrás restaurarla'<br/>Cancelar / Mover a la papelera"]
    cancel(["Cancelar: no cambia nada"])
    inv_trash["invoke('trash_photo',<br/>{ projectId, photoId })"]
    inv_res["invoke('restore_photo',<br/>{ projectId, photoId })<br/>sin confirmación"]
    next(["continúa en §2"])

    start --> menu
    menu -->|Mostrar en carpeta| reveal
    menu -->|Mover a la papelera| modal
    modal -->|Cancelar| cancel
    modal -->|Mover a la papelera| inv_trash --> next
    menu -->|Restaurar| inv_res --> next

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    class start,next,cancel term
    class menu,modal ui
    class reveal,inv_trash,inv_res ipc
```

## 2. Movimiento de archivos y persistencia (backend)

```mermaid
flowchart TD
    be["Rust: trash_photo() / restore_photo()<br/>→ TrashPhoto / RestorePhoto"]
    get_photo["PhotoRepository::get(photo_id)"]
    noop_q{"¿Ya está en el<br/>estado pedido?"}
    noop_ok["No-op<br/>return Ok()"]
    paths["Calcular destinos (domain::paths)<br/>Papelera: original → _trash/ · exportado → _trash/_exported/<br/>Restaurar: original → _culled/ o _media/ (según culled)<br/>· exportado → _exported/"]
    free_q{"¿Algún destino<br/>ya existe?"}
    conflict["Error 'Ya existe un archivo…'<br/>Nada se mueve"]
    move["Mover original (obligatorio)<br/>+ .xmp y ._nombre (mejor esfuerzo)<br/>Mover exportado (obligatorio) + ._nombre"]
    move_q{"¿Error al mover<br/>algo obligatorio?"}
    db_update["UPDATE photo SET deleted, dng_path, jpg_path<br/>Al mover a la papelera, en la misma transacción:<br/>UPDATE project SET cover_photo_path = NULL<br/>si era la portada"]
    db_q{"¿Error<br/>en BD?"}
    rollback["ROLLBACK<br/>deshacer los movimientos<br/>en orden inverso"]
    ok["return Ok() → IPC response"]
    next(["continúa en §3"])

    n_exp["Restaurar busca el exportado en<br/>_trash/_exported/ del proyecto actual,<br/>aunque jpg_path haya quedado viejo.<br/>Si no está en ningún lado, jpg_path = NULL."]

    be --> get_photo --> noop_q
    noop_q -->|Sí| noop_ok --> next
    noop_q -->|No| paths --> free_q
    free_q -->|Sí| conflict
    free_q -->|No| move --> move_q
    move_q -->|Error| rollback
    move_q -->|OK| db_update --> db_q
    db_q -->|Error| rollback
    db_q -->|OK| ok --> next
    conflict -.->|error al frontend| next
    rollback -.->|error al frontend| next
    paths -.- n_exp

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    classDef note fill:#fff9c4,stroke:#f9a825,color:#3d3300
    class next term
    class be,get_photo,noop_ok,paths,move,ok be
    class db_update db
    class noop_q,free_q,move_q,db_q dec
    class conflict,rollback err
    class n_exp note
```

## 3. Respuesta al usuario

```mermaid
flowchart TD
    trash_ok["Papelera OK: override { deleted: true }<br/>la foto sale del grid sin recargar (conserva el scroll)<br/>se cierra el modal · se recarga la lista de la papelera"]
    res_ok["Restaurar OK: override { deleted: false }<br/>la foto sale de la vista 'Papelera'<br/>se recarga la lista de fotos activas"]
    trash_err["Error al mover a la papelera: se muestra en el modal<br/>la foto sigue en el grid"]
    res_err["Error al restaurar: alerta<br/>la foto sigue en la papelera"]
    fin(["Disco y BD alineados:<br/>la foto está en _trash/ (en la papelera)<br/>o de vuelta en _media/ · _culled/ (restaurada)"])

    trash_ok --> fin
    res_ok --> fin
    trash_err --> fin
    res_err --> fin

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    class fin term
    class trash_ok,res_ok ui
    class trash_err,res_err err
```

## Notas

| Punto | Detalle |
|-------|---------|
| Nunca se borra | La papelera solo mueve archivos dentro del proyecto. Vaciar `_trash/` es manual. |
| Por qué "papelera" | En proyectos, "Eliminar" es permanente y "Archivar" oculta sin cambiar nada; esta acción es reversible pero saca la foto del respaldo. La columna de BD sigue siendo `deleted`. |
| Sin sobrescribir | Antes de mover se comprueba que ningún destino exista. Si existe, la operación falla sin cambios. |
| Efectos de `deleted = 1` | `get_project_photos` la excluye, así que desaparece del grid, del conteo del dashboard y del respaldo. |
| Aprobación bloqueada | `CullPhoto` rechaza una foto en la papelera; en el visor no hay actualización optimista. |
| Filtro 'Papelera' | Cambia la fuente del grid a `get_project_trashed_photos`. No se guarda en `project_settings`. |
| Mostrar en carpeta | `reveal_item_in_dir` de `tauri-plugin-opener`. En Linux depende de `org.freedesktop.FileManager1`. |

## Código

| Pieza | Archivo |
|-------|---------|
| Menú de la card | `packages/ui/src/components/CardMenu/CardMenu.tsx`, `PhotoCard/PhotoCard.tsx` |
| UI del flujo | `apps/desktop/src/pages/Project/ProjectDetail/ProjectDetail.tsx`, `TrashPhotoModal.tsx`, `ProjectDetailFooter.tsx` |
| Comandos | `apps/desktop/src-tauri/src/commands/photo.rs` (`trash_photo`, `restore_photo`, `get_project_trashed_photos`, `reveal_photo`) |
| Casos de uso | `apps/desktop/src-tauri/src/application/use_cases/trash_photo.rs` |
| Reglas de rutas | `apps/desktop/src-tauri/src/domain/paths.rs` |
| Persistencia | `apps/desktop/src-tauri/src/db/queries.rs` (`set_photo_deleted`, `get_project_trashed_photos`) |
