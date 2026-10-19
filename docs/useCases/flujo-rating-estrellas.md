# Flujo: Rating de estrellas, color labels y tags

Lumik Desktop · Linux · Windows · Android

Los tres controles del sidebar (estrellas, color labels, tags) comparten un único comando.
Cada cambio envía **el estado completo de los tres valores**, no solo el campo que cambió.

## 1. Sidebar de rating → IPC (frontend)

```mermaid
flowchart TD
    start(["Usuario en PhotoDetail<br/>revisando una foto"])

    subgraph SB["PhotoSidebar.tsx — sección Rating (sidebar derecho, 300 px, scroll)"]
        direction LR
        stars["★ Estrellas (1-5)<br/>• Botones de 1 a 5 estrellas<br/>• Teclas 1-5 del teclado<br/>• Clic en estrella ya activa → 0 (limpiar)<br/>• Valor 0 = sin rating"]
        colors["● Color labels<br/>• 5 colores: Rojo · Amarillo · Verde<br/>Azul · Morado<br/>• Multi-select (varios activos)<br/>• Almacenados como IDs: '1,3,5'"]
        tags["Tags / Etiquetas<br/>• Texto libre, separadas por coma<br/>• Ej: 'boda,mejor,editar'<br/>• Sin autocompletado<br/>• null = sin etiquetas"]
    end

    any["Cualquier cambio en stars · color labels · tags"]
    optimistic["Actualización optimista: la UI refleja el cambio<br/>de inmediato (estado React local)<br/>setSaveState('saving') → indicador '↻ Guardando…'"]
    send_all["Se envían siempre los 3 valores juntos<br/>(el estado completo, no solo el campo que cambió):<br/>{ stars: número actual, colorLabel: string | null,<br/>tags: string | null }"]
    invoke["api.savePhotoRating() → invoke('save_photo_rating',<br/>{ photoId, projectId, stars, colorLabel, tags })"]
    next(["continúa en §2"])

    start --> stars --> any
    start --> colors --> any
    start --> tags --> any
    any --> optimistic --> send_all --> invoke --> next

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    class start,next term
    class stars,colors,tags,any,optimistic,send_all ui
    class invoke ipc
```

## 2. Backend, persistencia y respuesta

```mermaid
flowchart TD
    be["Rust: save_photo_rating(state, photo_id,<br/>project_id, stars, color_label, tags)"]
    get_db["state.project_db(&project_id)<br/>Obtener ProjectDatabase del mapa open_projects"]
    val_q{"¿stars ∈ [0..5]?"}
    val_err["Error: 'Stars inválidas: N'<br/>→ retornar error al frontend"]
    db_update["project_db.update_photo_rating()<br/><br/>UPDATE photo<br/>SET stars = ?1, color_label = ?2, tags = ?3<br/>WHERE id = ?4<br/><br/>Transacción SQLite atómica<br/>Sin movimiento de archivo"]
    db_q{"¿Error<br/>en BD?"}
    db_err["Error: retornar<br/>al frontend"]
    ok["return Ok() → IPC response"]
    fe_ok["setSaveState('saved') → indicador '✓ Guardado'<br/>visible 2 s → vuelve a idle<br/>onPhotoChanged({ stars, colorLabel, tags })<br/>→ actualiza photoOverrides en el grid (sin re-query)"]
    fe_err["En error: revertir valores locales de<br/>stars / colorLabel / tags<br/>+ setSaveState('error') → '✕ Error al guardar'"]
    fin(["Rating visible en sidebar, tarjeta de galería y futuros filtros<br/>Persistido en project.db · Independiente del dispositivo montado"])

    sidecar["Efecto adicional (solo color labels)<br/>Linux/Windows · si finder_tags_sidecar = ON:<br/>→ escribe sidecar AppleDouble ._nombre<br/>(Finder tags de macOS/iPad) al DNG y al JPG<br/>→ invalida .Spotlight-V100 del volumen<br/>(el iPad reindexa al reconectar)<br/>Best-effort: el rating no falla si esto falla<br/>stars/tags: sin XMP ni exiftool"]

    be --> get_db --> val_q
    val_q -->|Inválido| val_err -.-> fe_err
    val_q -->|Válido| db_update --> db_q
    db_q -->|Error| db_err -.-> fe_err
    db_q -->|OK| ok --> fe_ok
    db_update -.- sidecar
    fe_ok --> fin
    fe_err --> fin

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    classDef note fill:#fff9c4,stroke:#f9a825,color:#3d3300
    class fin term
    class fe_ok ui
    class be,get_db,ok be
    class db_update db
    class val_q,db_q dec
    class val_err,db_err,fe_err err
    class sidecar note
```

## Notas

| Punto | Detalle |
|-------|---------|
| Formato en BD | `color_label` son IDs separados por coma (`"1,3,5"`); `tags` es texto libre separado por coma (`"boda,mejor"`); `null` cuando no hay valor. |
| Sin movimiento de archivo | A diferencia del [culling](flujo-culling.md), el rating nunca toca el sistema de archivos del proyecto. |
| Finder tags para iPad | Solo para color labels, solo en Linux/Windows y solo con `finder_tags_sidecar` activo. Hay que invalidar `.Spotlight-V100` para que el iPad reindexe y pueda filtrar. |
| Independiente del dispositivo | El rating vive en `project.db`, junto a las fotos, así que viaja con el disco externo. |

## Archivos de código

| Fase | Archivos |
|------|----------|
| Sidebar (§1) | `src/pages/Project/PhotoDetail/PhotoSidebar.tsx` — sección rating: estrellas, colores, tags (~líneas 290–350) |
| Atajos (§1) | `src/lib/hooks.ts` — `DEFAULT_KEYBINDINGS`: `photo_detail.stars_0` … `stars_5` |
| Optimismo (§1) | `src/pages/Project/PhotoDetail/PhotoDetail.tsx` — `handleRatingChange()`, `setSaveState()` (~líneas 186–214) |
| IPC (§1) | `src/lib/api.ts` — `savePhotoRating(photoId, projectId, stars, colorLabel, tags)` (~líneas 94–102) |
| Comando (§2) | `src-tauri/src/commands.rs` — `save_photo_rating()` → `sync_color_sidecar()` · `src-tauri/src/apple_tags.rs` — sidecar AppleDouble |
| Estado (§2) | `src-tauri/src/db/mod.rs` — `ProjectDatabase`, mapa `open_projects` (~líneas 185–238) |
| Query (§2) | `src-tauri/src/db/queries.rs` — `update_photo_rating()`, `UPDATE photo SET stars, color_label, tags` (~líneas 533–540) |
