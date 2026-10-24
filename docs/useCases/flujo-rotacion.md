# Flujo: Rotación de imagen

Lumik Desktop · Linux · Windows · Android

La rotación se aplica primero en el canvas (optimista, sin re-render de React), se persiste en
`project.db`, se regenera la miniatura, y la escritura al archivo original ocurre en un hilo de
fondo que el comando **no espera**.

## Vista general

```mermaid
flowchart LR
    A["↺ / ↻<br/>canvas optimista"]
    B["invoke<br/>'save_photo_rotation'"]
    C["UPDATE photo<br/>SET rotation"]
    D["Regenerar<br/>.thumbs/"]
    E["Hilo de fondo:<br/>EXIF / XMP"]
    F["'✓ Guardado'"]

    A --> B --> C --> D --> E
    D --> F

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef plat fill:#e1d5e7,stroke:#9673a6,color:#2a1a33
    class A,F ui
    class B ipc
    class C db
    class D be
    class E plat
```

## 1. Disparo y actualización optimista (frontend)

```mermaid
flowchart TD
    start(["Usuario en PhotoDetailView<br/>revisando foto en canvas"])
    trigger["Trigger de rotación<br/>• Botones ↺ / ↻ en la barra del visor<br/>• Keybindings rotateLeft / rotateRight (configurables)<br/>• rotateLeft() / rotateRight() expuestos<br/>vía useImperativeHandle"]
    calc["Calcular nueva rotación absoluta<br/>rotateRight → newRot = (localRotation + 90) % 360<br/>rotateLeft → newRot = (localRotation + 270) % 360"]
    optimistic["Actualización optimista: el canvas re-renderiza con<br/>ctx.rotate() — sin re-render de React (zoom/pan son refs)<br/>requestAnimationFrame() garantiza que el canvas dibuja<br/>antes de que empiece el invoke()<br/>setSaveState('saving') → indicador '↻ Guardando…'"]
    invoke["api.savePhotoRotation() → invoke('save_photo_rotation',<br/>{ photoId, projectId, rotation: newRot })"]
    next(["continúa en §2"])

    note["Modelo de rotación en canvas<br/>• RAW: preview en .previews/ con Orientation=1<br/>(exiftool lo hace strip)<br/>• JPEG: sin caché permanente — temp file en /tmp/, se descarta<br/>• El canvas aplica ctx.rotate(rad) manualmente usando<br/>photo.rotation de la BD<br/>• El browser/WebView nunca auto-rota (Orientation=1 en ambos)<br/>• Sin flash entre fotos"]

    start --> trigger --> calc --> optimistic --> invoke --> next
    trigger -.- note

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef note fill:#fff9c4,stroke:#f9a825,color:#3d3300
    class start,next term
    class trigger,calc,optimistic ui
    class invoke ipc
    class note note
```

## 2. Validación, base de datos y miniatura (backend)

```mermaid
flowchart TD
    be["Rust: save_photo_rotation()<br/>Validar rotation ∈ {0, 90, 180, 270}"]
    val_q{"¿Válido?"}
    val_err["Error: 'Rotación inválida'<br/>→ retornar al frontend"]
    get_photo["project_db.get_photo(&photo_id)<br/>Leer old_rotation desde la BD"]
    delta["delta = (rotation − old_rotation + 360) % 360<br/>Grados que debe girar la miniatura"]
    db_update["UPDATE photo SET rotation = ? WHERE id = ?<br/>db/queries.rs :: update_photo_rotation()<br/>Transacción síncrona<br/>WAL mode activo (PRAGMA journal_mode=WAL)<br/>→ ~50 ms en SD card (antes ~1200 ms)"]
    regen["regenerate_rotated_thumbnail() [solo si delta ≠ 0]<br/>• Leer .thumbs/{photo_id}.jpg existente<br/>• image crate: rotate90 / rotate180 / rotate270 según delta<br/>• Sobreescribir el mismo archivo — sin exiftool, solo píxeles"]
    next(["continúa en §3"])

    be --> val_q
    val_q -->|No| val_err
    val_q -->|Sí| get_photo --> delta --> db_update --> regen --> next

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    classDef note fill:#fff9c4,stroke:#f9a825,color:#3d3300
    class next term
    class be,get_photo,delta be
    class db_update db
    class regen note
    class val_q dec
    class val_err err
```

## 3. Escritura al archivo (hilo de fondo) y respuesta

La escritura al RAW original corre en `std::thread::spawn`. El hilo principal **no espera**:
retorna `Ok()` de inmediato.

```mermaid
flowchart TD
    plat_q{"Plataforma"}
    desktop_write["Linux / Windows<br/>exiftool -IFD0:Orientation={val} -n<br/>-overwrite_original {dng}<br/>Mapa EXIF: 90°→6 · 180°→3 · 270°→8 · 0°→1<br/>Escribe en el original en el disco externo<br/>(en un par, también en su JPG)"]
    android_write["Android<br/>xmp::update_xmp_orientation(&dng, rotation)<br/>Escribe / actualiza el sidecar {dng}.xmp<br/>Sin exiftool — Rust puro"]
    thread["Hilo background — std::thread::spawn<br/>El main thread NO espera<br/>→ return Ok() inmediato"]
    ret["return Ok() — IPC response<br/>(antes de que el hilo de escritura termine)"]
    fe_ok["setSaveState('saved') → indicador '✓ Guardado' 2 s → idle<br/>El canvas ya muestra la rotación correcta<br/>(actualizado optimistamente antes del IPC)<br/>Miniatura en galería ya rotada (.thumbs/ sobreescrito)"]
    fe_err["En error: setLocalRotation(old_rotation) — revertir canvas<br/>setSaveState('error') → '✕ Error al guardar'"]
    fin(["Foto rotada en canvas y galería · BD actualizada<br/>DNG / sidecar XMP actualizado en segundo plano"])

    plat_q -->|Linux/Win| desktop_write --> ret
    plat_q -->|Android| android_write --> ret
    thread --> ret
    ret --> fe_ok --> fin
    fe_err --> fin

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef plat fill:#e1d5e7,stroke:#9673a6,color:#2a1a33
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    class fin term
    class fe_ok ui
    class ret be
    class desktop_write,android_write,thread plat
    class plat_q dec
    class fe_err err
```

## Notas

| Punto | Detalle |
|-------|---------|
| `rotation` es absoluta | La BD guarda 0/90/180/270. `delta` solo se usa para saber cuánto girar la miniatura existente. |
| Nadie auto-rota | Tanto el preview RAW como el temp JPEG se escriben con `Orientation=1`, así que el canvas es la única fuente de rotación visible. |
| Miniatura por píxeles | `.thumbs/` se reescribe girando los píxeles, sin tocar EXIF. |
| Escritura asíncrona | Si falla la escritura al DNG/XMP, la BD y la UI ya quedaron actualizadas; el error no vuelve al frontend. |
| Mapa EXIF | 0°→1, 90°→6, 180°→3, 270°→8. |
| Pares RAW+JPG | `RotatePhoto` devuelve los dos archivos y la escritura asíncrona orienta ambos; la miniatura se gira una vez. |

## Archivos de código

| Fase | Archivos |
|------|----------|
| Disparo (§1) | `src/pages/Project/PhotoDetail/PhotoDetailView.tsx` — `rotateLeft()`, `rotateRight()`, `useImperativeHandle`, `PhotoViewer`; canvas `ctx.rotate(rad)` |
| Atajos (§1) | `src/lib/hooks.ts` — `DEFAULT_KEYBINDINGS`: `photo_detail.rotateLeft` / `rotateRight` |
| IPC (§1) | `src/lib/api.ts` — `savePhotoRotation(photoId, projectId, rotation)` |
| Comando (§2) | `src-tauri/src/commands.rs` — `save_photo_rotation()` (~L606), `regenerate_rotated_thumbnail()` (~L572), `read_exif_rotation()` (~L487) |
| Query (§2) | `src-tauri/src/db/queries.rs` — `update_photo_rotation()`, `UPDATE photo SET rotation` |
| Android (§3) | `src-tauri/src/import/xmp.rs` — `update_xmp_orientation()`, sidecar `{dng}.xmp` en Rust puro |
