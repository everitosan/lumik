# Flujo: Papelera de fotos (mover, restaurar, eliminar definitivamente)

Lumik Desktop · Linux · Windows · macOS · Android (tablet)

Borrar una foto tiene **dos pasos**. Primero se manda a la papelera: es un borrado lógico, la fila
queda con `deleted = 1` y sus archivos se apartan dentro del proyecto sin borrar nada. El original
va a `_trash/` y su JPG exportado a `_trash/_exported/` (así un original JPG y su exportado con el
mismo nombre no se pisan, y los escaneos de `_exported/` no lo vuelven a enlazar ni a respaldar).
Después, desde la sección **"Papelera"** de la app, se restaura o se **elimina definitivamente**.
Lumik nunca vacía la papelera por su cuenta.

Igual que en el [culling](flujo-culling.md), los archivos se mueven **antes** del `UPDATE`, y si
algo falla se devuelven a su sitio.

Un par RAW+JPG va a la papelera, vuelve y se elimina **con sus dos archivos**: el JPG de cámara se
aparta en `_trash/` junto al RAW (o, si ya se entregó, viaja como exportado a `_trash/_exported/`).

Los proyectos completos siguen su propio flujo: [Papelera de proyectos](flujo-papelera-proyecto.md).
La sección "Papelera" muestra los dos.

> La papelera no se propaga al catálogo del servidor: el portal web y el iPad siguen mostrando la
> foto, y eliminarla definitivamente no toca su respaldo en la nube (se borra desde el portal).

Varias fotos a la vez (mandar a la papelera, restaurar o eliminar en lote): ver
[Marcar fotos y actuar en lote](flujo-marcar-fotos.md).

## Vista general

```mermaid
flowchart LR
    A["Grid del proyecto<br/>'Mover a la papelera' → modal"]
    B["invoke('trash_photo')<br/>→ TrashPhoto"]
    C["Sección 'Papelera'<br/>agrupada por disco"]
    D["invoke('restore_photo')<br/>→ RestorePhoto"]
    E["invoke('purge_photo')<br/>→ PurgePhoto"]
    F["De vuelta en _media/ · _culled/"]
    G["Sin archivos ni registro"]

    A --> B --> C
    C -->|Restaurar| D --> F
    C -->|Eliminar definitivamente| E --> G

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    class A,C ui
    class B,D,E ipc
    class F,G term
```

## 1. Disparadores en la UI

```mermaid
flowchart TD
    grid(["Grid del proyecto"])
    menu["PhotoCard — menú ⋮<br/>• Mostrar en carpeta (no en Android)<br/>• Mover a la papelera"]
    modal["TrashPhotoModal<br/>'se moverá a la papelera (_trash/)<br/>· podrás restaurarla o eliminarla desde la sección Papelera'"]
    inv_trash["invoke('trash_photo')"]
    section(["Sección 'Papelera'<br/>(solo existe si hay algo)"])
    menu2["PhotoCard — menú ⋮<br/>• Restaurar<br/>• Eliminar definitivamente"]
    inv_res["invoke('restore_photo', { newStem: null })<br/>sin confirmación"]
    collision["RestoreCollisionModal<br/>pide otro nombre base;<br/>valida en vivo con check_photo_restore"]
    inv_res2["invoke('restore_photo', { newStem })"]
    purge_modal["PurgeModal<br/>'no se puede deshacer'<br/>+ 'el respaldo en la nube no se toca'"]
    inv_purge["invoke('purge_photo')"]
    empty["'Vaciar papelera de {disco}'<br/>EmptyTrashModal → invoke('empty_trash')"]

    grid --> menu -->|Mover a la papelera| modal -->|Confirmar| inv_trash
    section --> menu2
    menu2 -->|Restaurar| inv_res
    inv_res -->|restore_collision:archivo| collision -->|Restaurar| inv_res2
    menu2 -->|Eliminar definitivamente| purge_modal -->|Confirmar| inv_purge
    section --> empty

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    class grid,section term
    class menu,modal,menu2,collision,purge_modal,empty ui
    class inv_trash,inv_res,inv_res2,inv_purge ipc
```

## 2. Mover a la papelera y restaurar (backend)

```mermaid
flowchart TD
    be["Rust: trash_photo() / restore_photo()<br/>→ TrashPhoto / RestorePhoto::execute_as(new_stem)"]
    get_photo["PhotoRepository::get(photo_id)"]
    noop_q{"¿Ya está en el<br/>estado pedido?"}
    noop_ok["No-op<br/>return Ok()"]
    paths["Calcular destinos (domain::paths)<br/>Papelera: original → _trash/ · exportado → _trash/_exported/<br/>· JPG de cámara de un par → _trash/ (si no es el exportado)<br/>Restaurar: original y JPG de cámara → _culled/ o _media/ (según culled)<br/>· exportado → _exported/ · con new_stem, todos con ese nombre base"]
    free_q{"¿Algún destino<br/>ya existe?"}
    conflict["Papelera: error 'Ya existe…'<br/>Restaurar: restore_collision:{archivo}<br/>Nada se mueve"]
    move["Mover original (obligatorio)<br/>+ .xmp y ._nombre (mejor esfuerzo)<br/>Mover exportado y JPG de cámara (obligatorios) + ._nombre"]
    move_q{"¿Error al mover<br/>algo obligatorio?"}
    db_update["UPDATE photo SET deleted, dng_path, jpg_path, camera_jpg_path<br/>Al mover a la papelera, en la misma transacción:<br/>UPDATE project SET cover_photo_path = NULL<br/>si era la portada"]
    db_q{"¿Error<br/>en BD?"}
    rollback["deshacer los movimientos<br/>en orden inverso"]
    ok["return Ok()<br/>api.ts avisa lumik:trash-changed"]

    be --> get_photo --> noop_q
    noop_q -->|Sí| noop_ok
    noop_q -->|No| paths --> free_q
    free_q -->|Sí| conflict
    free_q -->|No| move --> move_q
    move_q -->|Error| rollback
    move_q -->|OK| db_update --> db_q
    db_q -->|Error| rollback
    db_q -->|OK| ok

    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    class be,get_photo,noop_ok,paths,move,ok be
    class db_update db
    class noop_q,free_q,move_q,db_q dec
    class conflict,rollback err
```

## 3. Eliminar definitivamente (backend)

```mermaid
flowchart TD
    be["Rust: purge_photo() → write_session (candado)<br/>→ PurgePhoto::execute"]
    lock_q{"¿Otro dispositivo<br/>tiene el proyecto?"}
    locked["project_locked:{dispositivo}<br/>nada cambia"]
    trashed_q{"¿deleted = 1?"}
    refuse["Error: solo se elimina<br/>lo que está en la papelera"]
    files["Borrar del disco, ignorando los que ya no están:<br/>original + .xmp + ._nombre · JPG del par + sidecars<br/>· exportado en _trash/_exported/ + ._nombre<br/>· .thumbs/{id}.jpg · .previews/{id}.jpg y {id}-raw.jpg<br/>(solo dentro de _trash/, .thumbs/ y .previews/)"]
    file_q{"¿Falló un borrado<br/>de un archivo que sigue ahí?"}
    keep["Error; la foto sigue en la papelera<br/>repetirlo termina el trabajo"]
    db["Transacción: DELETE photo_embedding · DELETE photo"]
    ok["Sin rastro en el proyecto"]

    be --> lock_q
    lock_q -->|Sí| locked
    lock_q -->|No| trashed_q
    trashed_q -->|No| refuse
    trashed_q -->|Sí| files --> file_q
    file_q -->|Sí| keep
    file_q -->|No| db --> ok

    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    class be,files be
    class db db
    class lock_q,trashed_q,file_q dec
    class locked,refuse,keep err
    class ok term
```

## Notas

| Punto | Detalle |
|-------|---------|
| Dos pasos | Mandar a la papelera nunca borra. Solo "Eliminar definitivamente" o "Vaciar papelera", desde la sección, borran archivos. |
| Archivos primero | `PurgePhoto` borra los archivos antes que la fila: si se interrumpe, la foto sigue listada y repetir termina. Al revés quedarían archivos huérfanos invisibles. |
| Sin sobrescribir | Antes de mover se comprueba que ningún destino exista (incluido el `.xmp`). Al restaurar, la colisión abre el modal que pide otro nombre. |
| Efectos de `deleted = 1` | `get_project_photos` la excluye: desaparece del grid, del conteo del dashboard y del respaldo. El grid del proyecto nunca muestra fotos en la papelera. |
| Aprobación bloqueada | `CullPhoto` rechaza una foto en la papelera. |
| Entrada del menú | `TrashContext` recalcula `list_trash` al cambiar los discos y con el evento `lumik:trash-changed` que emite `api.ts` tras cada acción. |
| Respaldo | Eliminar definitivamente no toca la nube: el aviso del modal lo dice. |

## Código

| Pieza | Archivo |
|-------|---------|
| Menú de la card | `packages/ui/src/components/CardMenu/CardMenu.tsx`, `PhotoCard/PhotoCard.tsx` (`onTrash`, `onRestore`, `onPurge`) |
| Mandar a la papelera | `apps/desktop/src/pages/Project/ProjectDetail/ProjectDetail.tsx`, `TrashPhotoModal.tsx` |
| Sección "Papelera" | `apps/desktop/src/pages/Trash/TrashPage.tsx`, `TrashModals.tsx`, `apps/desktop/src/lib/TrashContext.tsx` |
| Comandos | `apps/desktop/src-tauri/src/commands/photo.rs` (`trash_photo`, `restore_photo`, `check_photo_restore`, `reveal_photo`), `commands/trash.rs` (`list_trash`, `purge_photo`, `empty_trash`) |
| Casos de uso | `crates/lumik-core/src/application/use_cases/trash_photo.rs` (`TrashPhoto`, `RestorePhoto`, `PurgePhoto`) |
| Reglas de rutas | `crates/lumik-core/src/domain/paths.rs` |
| Persistencia | `crates/lumik-core/src/db/queries.rs` (`set_photo_deleted`, `purge_trashed_photo`, `get_project_trashed_photos`) |
