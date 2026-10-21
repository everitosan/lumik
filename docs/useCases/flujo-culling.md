# Flujo: Culling de imagen

Lumik Desktop · Linux · Windows · Android

Marcar una foto como *culled* mueve el archivo entre `_media/` y `_culled/` y actualiza
`dng_path` en la base. El movimiento de archivo ocurre **antes** del `UPDATE`, y si la base
falla se revierte el archivo a su ruta original.

> El rating (estrellas, color labels, tags) comparte el sidebar pero es un comando distinto y
> no toca el sistema de archivos: ver [flujo-rating-estrellas.md](flujo-rating-estrellas.md).

## Vista general

```mermaid
flowchart LR
    A["SPACEBAR / checkbox<br/>(optimista)"]
    B["invoke<br/>'save_photo_culled'"]
    C["fs::rename<br/>_media/ ↔ _culled/"]
    D["UPDATE photo<br/>culled + dng_path"]
    E["Badge + filtro<br/>'Solo aprobadas'"]

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
    be["Rust: save_photo_culled() — commands.rs"]
    get_photo["project_db.get_photo(&photo_id)<br/>Leer registro actual"]
    noop_q{"¿photo.culled<br/>== culled?"}
    noop_ok["No-op<br/>return Ok()"]
    paths["Calcular rutas<br/>current_path = project_dir + dng_path<br/>target_dir = culled ? _culled/ : _media/<br/>new_dng_path = target_dir + filename"]
    mkdir["fs::create_dir_all(target_dir)<br/>Garantiza que _culled/ o _media/ existen"]
    move["fs::rename(current_path, target_path) — DNG / RAW<br/>Si existe .xmp → fs::rename(xmp_src, xmp_dest)<br/>(best-effort, error ignorado)"]
    move_q{"¿Error<br/>al mover?"}
    move_err["Retornar error:<br/>'Error al mover archivo'<br/>BD intacta · Archivo intacto"]
    db_update["UPDATE photo<br/>SET culled = ?, dng_path = ?<br/>WHERE id = ?"]
    db_q{"¿Error<br/>en BD?"}
    rollback["ROLLBACK<br/>fs::rename de vuelta<br/>al path original"]
    ok["return Ok() → IPC response"]
    next(["continúa en §3"])

    n_xmp["El sidecar .xmp solo existe en Android<br/>(creado al importar por xmp.rs ·<br/>write_xmp_sidecar())<br/>Contiene: Artist · Copyright · Rights · contacto"]
    n_path["dng_path actualizado refleja<br/>la ubicación real del archivo,<br/>relativa a la carpeta del proyecto:<br/>→ _culled/foto.dng<br/>→ _media/foto.dng"]

    be --> get_photo --> noop_q
    noop_q -->|Sí| noop_ok --> next
    noop_q -->|No| paths --> mkdir --> move --> move_q
    move_q -->|Error| move_err
    move_q -->|OK| db_update --> db_q
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
    class be,get_photo,noop_ok,paths,mkdir,move,ok be
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

## Notas

| Punto | Detalle |
|-------|---------|
| Orden de operaciones | Archivo primero, base después. Si el `fs::rename` falla no se toca la base; si la base falla, el archivo se devuelve a su sitio. |
| No-op | Si `photo.culled` ya coincide con el valor pedido, el comando retorna `Ok()` sin mover nada. |
| Sidecar `.xmp` | Se mueve junto al RAW, pero su error se ignora (best-effort). Solo existe en Android. |
| Filtro 'Solo aprobadas' | Es client-side (`Ctrl+C`), no re-consulta la base. |
