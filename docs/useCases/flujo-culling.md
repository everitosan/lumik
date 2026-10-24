# Flujo: Culling de imagen

Lumik Desktop · Linux · Windows · Android

Marcar una foto como *culled* mueve el archivo entre `_media/` y `_culled/` y actualiza
`dng_path` en la base. El movimiento de archivo ocurre **antes** del `UPDATE`, y si la base
falla se revierten los movimientos en orden inverso.

Según la opción **"Al aprobar, entregar directamente"** del proyecto (`project_settings.delivery_mode`:
`none` / `pairs` / `all_camera_jpg`), aprobar también entrega el JPG de cámara a `_exported/`:
el JPG de un par RAW+JPG se **mueve** y un JPG suelto se **copia**. Lumik guarda la huella
(tamaño + SHA-256) del JPG entregado; quitar la aprobación lo deshace solo si sigue igual.
Ver §4.

> El rating (estrellas, color labels, tags) comparte el sidebar pero es un comando distinto y
> no toca el sistema de archivos: ver [flujo-rating-estrellas.md](flujo-rating-estrellas.md).

## Vista general

```mermaid
flowchart LR
    A["SPACEBAR / checkbox<br/>(optimista)"]
    B["invoke<br/>'save_photo_culled'"]
    C["fs::rename<br/>_media/ ↔ _culled/"]
    C2["Entrega del JPG de cámara<br/>a _exported/ (según la opción)"]
    D["UPDATE photo<br/>culled + rutas + entrega"]
    E["Badge + filtro<br/>'Solo aprobadas'"]

    A --> B --> C --> C2 --> D --> E

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    class A,E ui
    class B ipc
    class C,C2 be
    class D db
```

## 1. Disparadores en la UI

```mermaid
flowchart TD
    start(["Usuario en galería<br/>del proyecto"])
    grid["ProjectDetail — grid virtual de fotos<br/>• Fotos agrupadas por fecha de captura<br/>• Ctrl+C: filtro 'Solo aprobadas' (client-side, sin query a BD)<br/>• Clic en foto → abre PhotoDetail"]
    detail["PhotoDetail.tsx<br/>• SPACEBAR → toggle culled<br/>• Vista canvas con zoom / pan"]
    sidebar["PhotoSidebar.tsx — checkbox 'Aprobar foto'<br/>Actualización optimista: setLocalCulled(culled)<br/>→ status '↻ Guardando…'"]
    invoke["invoke('save_photo_culled',<br/>{ photoId, projectId, culled: bool })"]
    next(["continúa en §2"])

    start --> grid --> detail --> sidebar --> invoke --> next

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    class start,next term
    class grid,detail,sidebar ui
    class invoke ipc
```

## 2. Movimiento de archivo y persistencia (backend)

```mermaid
flowchart TD
    be["Rust: save_photo_culled() — commands/photo.rs<br/>CullPhoto { delivery: project_db.delivery_mode() }"]
    get_photo["project_db.get_photo(&photo_id)<br/>Leer registro actual"]
    noop_q{"¿photo.culled<br/>== culled?"}
    noop_ok["No-op<br/>return Ok()"]
    paths["Calcular rutas<br/>current_path = project_dir + dng_path<br/>target_dir = culled ? _culled/ : _media/<br/>new_dng_path = target_dir + filename"]
    mkdir["fs::create_dir_all(target_dir)<br/>Garantiza que _culled/ o _media/ existen"]
    move["fs::rename(current_path, target_path) — DNG / RAW<br/>Si existe .xmp → fs::rename(xmp_src, xmp_dest)<br/>(best-effort, error ignorado)"]
    move_q{"¿Error<br/>al mover?"}
    move_err["Retornar error:<br/>'Error al mover archivo'<br/>BD intacta · Archivo intacto"]
    deliver["Entrega o regreso del JPG de cámara<br/>(ver §4)"]
    db_update["UPDATE photo — update_photo_cull<br/>culled, dng_path, camera_jpg_path, jpg_path,<br/>workflow_status, delivered_size/sha256, delivery_kind"]
    db_q{"¿Error<br/>en BD?"}
    rollback["ROLLBACK<br/>deshace en orden inverso<br/>(renames y la copia entregada)"]
    ok["return Ok() → IPC response"]
    next(["continúa en §3"])

    n_xmp["El sidecar .xmp solo existe en Android<br/>(creado al importar por xmp.rs ·<br/>write_xmp_sidecar())<br/>Contiene: Artist · Copyright · Rights · contacto"]
    n_path["dng_path actualizado refleja<br/>la ubicación real del archivo,<br/>relativa a la carpeta del proyecto:<br/>→ _culled/foto.dng<br/>→ _media/foto.dng"]

    be --> get_photo --> noop_q
    noop_q -->|Sí| noop_ok --> next
    noop_q -->|No| paths --> mkdir --> move --> move_q
    move_q -->|Error| move_err
    move_q -->|OK| deliver --> db_update --> db_q
    db_q -->|Error| rollback
    db_q -->|OK| ok --> next
    move_err -.->|error al frontend| next
    rollback -.->|error al frontend| next
    move -.- n_xmp
    db_update -.- n_path

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    classDef note fill:#fff9c4,stroke:#f9a825,color:#3d3300
    class next term
    class be,get_photo,noop_ok,paths,mkdir,move,deliver,ok be
    class db_update db
    class noop_q,move_q,db_q dec
    class move_err,rollback err
    class n_xmp,n_path note
```

## 3. Respuesta al usuario

```mermaid
flowchart TD
    res["status → '✓ Guardado' (2 s) → idle<br/>onPhotoChanged({ culled }) → actualizar grid"]
    res_err["En error: setLocalCulled(!culled)<br/>→ revertir + status '✕ Error'"]
    fin(["Foto con badge de aprobada y nueva ubicación en BD<br/>Archivo (+ sidecar .xmp si existe) en _culled/<br/>Filtro 'Solo aprobadas' actualizado client-side"])

    res --> fin
    res_err --> fin

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    class fin term
    class res ui
    class res_err err
```

## 4. Entrega del JPG de cámara

```mermaid
flowchart TD
    start(["Original ya movido<br/>(§2)"])
    dir{"¿Aprobar o<br/>quitar aprobación?"}

    kind{"delivery_for(opción, par, JPG suelto)"}
    pair_free{"¿_exported/ tiene ya<br/>un archivo con ese nombre?"}
    pair_move["Par: mover el JPG de cámara<br/>_media/ → _exported/ (+ ._ sidecar)<br/>jpg_path = camera_jpg_path = _exported/x.jpg"]
    pair_keep["Par sin entrega (o nombre ocupado):<br/>JPG de cámara → _culled/ junto al RAW"]
    solo_copy["JPG suelto con 'todo JPG de cámara':<br/>copiar _culled/x.jpg → _exported/x.jpg<br/>(el original nunca queda expuesto)"]
    mark["Huella: tamaño + SHA-256<br/>delivery_kind = moved / copied<br/>workflow_status = edited"]
    nothing["Sin entrega"]

    fp{"¿Coincide la huella<br/>del JPG en _exported/?"}
    back_moved["moved: el JPG vuelve a _media/<br/>jpg_path = NULL · status imported"]
    back_copied["copied: la copia se borra después<br/>del UPDATE (único borrado de Lumik)"]
    keep_edit["Es una edición del fotógrafo:<br/>se queda en _exported/, enlazada<br/>(se limpia la huella)"]
    culled_back["JPG de cámara en _culled/ → _media/"]
    fin(["UPDATE (§2)"])

    start --> dir
    dir -->|Aprobar| kind
    kind -->|moved| pair_free
    pair_free -->|No| pair_move --> mark --> fin
    pair_free -->|Sí| pair_keep --> fin
    kind -->|copied| solo_copy --> mark
    kind -->|ninguna| nothing --> fin
    kind -.->|par con opción 'nada'| pair_keep
    dir -->|Quitar| fp
    fp -->|Sí| back_moved --> fin
    fp -->|Sí, copia| back_copied --> fin
    fp -->|No| keep_edit --> fin
    fp -->|Sin entrega| culled_back --> fin

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    class start,fin term
    class pair_move,pair_keep,solo_copy,mark,nothing,back_moved,back_copied,culled_back be
    class dir,kind,pair_free,fp dec
    class keep_edit err
```

Cambiar la opción con fotos ya aprobadas pasa por `plan_project_delivery_change` (conteo para el
aviso, por tamaño) y `set_project_delivery_mode { apply_to_approved }` (`ChangeDeliveryMode`,
que reusa `CullPhoto::deliver_approved` / `return_delivery` foto por foto, verificando la huella
completa). Un JPG que se une a una foto ya aprobada en el import se entrega con
`CullPhoto::deliver_joined`.

## Notas

| Punto | Detalle |
|-------|---------|
| Orden de operaciones | Archivo primero, base después. Si el `fs::rename` falla no se toca la base; si la base falla, el archivo se devuelve a su sitio. |
| No-op | Si `photo.culled` ya coincide con el valor pedido, el comando retorna `Ok()` sin mover nada. |
| Sidecar `.xmp` | Se mueve junto al RAW, pero su error se ignora (best-effort). Solo existe en Android. |
| Filtro 'Solo aprobadas' | Es client-side (`Ctrl+C`), no re-consulta la base. |
| Pares RAW+JPG | Una sola foto: `dng_path` es el RAW y `camera_jpg_path` su JPG; los dos se mueven juntos. |
| Nada se borra en la nube | Regresar un JPG ya respaldado no toca su copia en la nube. |
