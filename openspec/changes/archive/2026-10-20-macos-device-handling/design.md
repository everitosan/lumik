# Design

## Context

Ver `proposal.md` (Why) para la motivación. Estado actual relevante:

- `devices.rs::scan_mounted_devices` ya funciona en macOS: `sysinfo` lista solo volúmenes visibles en Finder, `should_skip_disk` excluye `/`, `/System` y `/private`, y el UUID se obtiene del `VolumeUUID` de `diskutil info -plist`. Se verificó con `test_scan_devices`, que detecta `/Volumes/KINGSTON`.
- `device_watch.rs::start` no tiene rama macOS (`let _ = app;`), así que en macOS solo actúa el sondeo de 60 s del frontend (`hooks.ts`, `DESKTOP_POLL_INTERVAL`). El frontend ya escucha `devices-changed` y reescanea.
- `infrastructure/devices.rs::os_eject` cae en la rama `not(any(linux, windows, android))` y devuelve `Ok(())` sin desmontar.
- `commands/device.rs::eject_device` cierra solo los `project.db` del `device_uuid` pedido antes de llamar a `DeviceScanner::eject`.
- `photo.device_uuid` (`NOT NULL`) se escribe en `create_photos_batch` desde `ImportParams.device_uuid` y nunca se lee en lógica. El `device_uuid` del dashboard (`ProjectDashboard`) se llena en tiempo de ejecución con el dispositivo del escaneo, no con la BD. `project` no guarda el disco.
- Ninguna parte del server, el portal web, el catálogo ni `packages/ts-types` usa el UUID del dispositivo.
- Para exFAT, macOS calcula `VolumeUUID = UUIDv3(MD5(ns_HFS ++ serial_LE))` a partir del serial que Linux expone como `XXXX-XXXX` (verificado). Por eso los UUID divergen; esta divergencia se acepta (ver la spec `device-detection`).

## Goals / Non-Goals

**Goals:**
- Que el watcher de macOS tenga el mismo contrato que los de Linux y Windows: solo avisa con `devices-changed`, y `scan_mounted_devices` sigue siendo la fuente de verdad.
- Que la expulsión en macOS sea síncrona desde el punto de vista del comando: `os_eject` devuelve `Ok` solo cuando el disco quedó desmontado y expulsado, o `Err` con el mensaje del sistema.
- Que el corte de `photo.device_uuid` no toque el esquema ni migre los `project.db`.

**Non-Goals:**
- Reemplazar `sysinfo` o `diskutil` en el escaneo de macOS; el escaneo ya funciona.
- Cambiar el contrato del puerto `DeviceScanner` para Linux, Windows o Android más allá de la extensión descrita en D4.
- Cambios en el frontend: ya reacciona a `devices-changed` y ya muestra los errores de expulsión.

## Decisions

### D1. Binding de DiskArbitration: FFI mínimo propio sobre `core-foundation`

Se declara a mano el subconjunto necesario de DiskArbitration en un módulo `#[cfg(target_os = "macos")]` (por ejemplo `infrastructure/disk_arbitration.rs`): `DASessionCreate`, `DASessionScheduleWithRunLoop` y `DASessionUnscheduleFromRunLoop`, `DARegisterDiskDescriptionChangedCallback`, `DARegisterDiskDisappearedCallback`, `DADiskCreateFromVolumePath`, `DADiskCopyWholeDisk`, `DADiskUnmount`, `DADiskEject` y `DADissenterGetStatus`/`DADissenterGetStatusString`. Se enlaza con `#[link(name = "DiskArbitration", kind = "framework")]`. Los tipos y el run loop de CoreFoundation vienen de `core-foundation` / `core-foundation-sys`, que ya están en el `Cargo.lock` como dependencias transitivas de Tauri y se agregan como dependencias directas solo para macOS.

- *Alternativa: crate `objc2-disk-arbitration`.* Bindings generados y completos, pero es otra familia de crates con su propio modelo de tipos (`CFRetained`, etc.) para unas diez funciones. Se descarta por el tamaño del uso, igual que Windows usa `windows-sys` directo.
- *Alternativa: `diskutil` como subproceso para expulsar.* Más simple, pero no da el estado estructurado del rechazo y obliga a parsear texto localizado. Se descarta porque el framework ya queda enlazado por el watcher.

### D2. Watcher: cambio de `VolumePath` + desaparición, en un hilo con su propio CFRunLoop

`device_watch::start` gana una rama `#[cfg(target_os = "macos")]` que crea un hilo dedicado (`watch_macos`). En ese hilo:

1. `DASessionCreate`, y se programa la sesión en `CFRunLoopGetCurrent()`.
2. Se registra `DARegisterDiskDescriptionChangedCallback` filtrando por la clave `kDADiskDescriptionVolumePathKey`. Ese callback salta cuando un volumen se **monta o desmonta**, que es lo que el escaneo puede ver.
3. Se registra `DARegisterDiskDisappearedCallback` para desconexiones físicas sin desmontaje previo.
4. `CFRunLoopRun()` bloquea el hilo durante toda la vida de la app.

Cada callback hace un debounce de 300 ms y emite `devices-changed`, como en Linux. El `AppHandle` llega al callback por el puntero de contexto (`Box::into_raw`, que vive para siempre igual que el hilo).

- *Por qué no `DARegisterDiskAppearedCallback`:* salta cuando el disco aparece en el bus, **antes** de que macOS lo monte, así que un escaneo inmediato todavía no vería `/Volumes/X`. Además, al registrarlo se dispara una vez por cada disco ya presente.
- *Alternativa: notificaciones `NSWorkspace` de montaje y desmontaje.* Requieren el notification center de AppKit, ligado al hilo principal que maneja Tauri. Se descarta.

### D3. Expulsión: desmontar el disco completo y expulsar, esperando los callbacks con un run loop temporal

`os_eject` en macOS:

1. Lanza un hilo propio y espera su resultado con `join`: `eject_device` es un comando síncrono y Tauri v2 lo ejecuta en el hilo principal, donde correr un run loop anidado bloquearía la UI. En ese hilo crea una sesión DA y la programa en su run loop.
2. `DADiskCreateFromVolumePath(mount_point)` y luego `DADiskCopyWholeDisk` para obtener el disco físico.
3. `DADiskUnmount(whole, kDADiskUnmountOptionWhole, cb)`. El callback guarda el resultado (`None` o un dissenter) y detiene el run loop.
4. `CFRunLoopRunInMode(kCFRunLoopDefaultMode, 30 s, false)` hasta que llegue el callback o venza el tiempo.
5. Si el desmontaje tuvo éxito: `DADiskEject(whole, kDADiskEjectOptionDefault, cb)` con la misma espera. Un rechazo al expulsar, después de un desmontaje exitoso, se registra en el log pero se devuelve `Ok`: el volumen ya no está montado y el disco es seguro de retirar, igual que en Linux, donde se ignora el resultado de `power-off`.
6. Si `DADiskCreateFromVolumePath` falla porque el volumen ya no existe, se devuelve `Ok` (escenario "Dispositivo ya desmontado").

El error se arma con `DADissenterGetStatusString` cuando existe, y si no con el código de estado traducido (`kDAReturnBusy` → "el disco está en uso por otro proceso"). La spec pide el proceso que bloquea "si el sistema lo reporta": la cadena del dissenter suele incluir el PID y el nombre. Si no los trae, el mensaje queda genérico.

- *Alternativa: `kDADiskUnmountOptionDefault` sobre el volumen.* Desmontaría solo esa partición y dejaría el disco físico encendido. No cumple "seguro de retirar".

### D4. Disco con varias particiones: cerrar los proyectos de todos los volúmenes del mismo disco físico

Como D3 desmonta todo el disco físico, `eject_device` debe cerrar los `project.db` de **todas** las particiones Lumik de ese disco, no solo la del UUID pedido. Si no, la otra partición queda ocupada por Lumik mismo.

El puerto `DeviceScanner` gana `fn devices_on_same_physical_disk(&self, device_uuid: &str) -> Vec<String>`, con una implementación por defecto que devuelve solo `[device_uuid]` (Linux, Windows y Android quedan igual). En macOS se resuelve con `DADiskCopyWholeDisk` para cada dispositivo escaneado, comparando el BSD name del disco completo. `eject_device` marca como `ejecting` y cierra los proyectos de todos esos UUID, y al terminar emite `devices-changed` como hoy.

- *Alternativa: agregar `whole_disk` a `DetectedDevice`.* Expone al frontend un detalle de la plataforma y cambia el tipo serializado. Se descarta.

### D5. `photo.device_uuid`: escribir cadena vacía, sin tocar el esquema

`ImportPhotos` deja de copiar `ImportParams.device_uuid` a `CreatePhoto.device_uuid` y escribe `""` (la columna es `NOT NULL`). `ImportParams.device_uuid` y el campo de `ImportRequest` se quitan en Rust porque solo servían para llenar la foto. El frontend sigue enviando `device_uuid` y serde lo ignora, así que no hay cambio en TS. El tipo `Photo` (Rust y TS) conserva el campo para leer las filas existentes. En `project_schema.sql` se agrega un comentario de obsolescencia junto a la columna y al índice `idx_photo_device`. El índice se conserva porque quitarlo sería una migración en cada `project.db`, que es lo que se quiere evitar.

- *Alternativa: eliminar la columna con una migración.* Toca cada `project.db` del disco externo y no aporta comportamiento. Ya está fuera de alcance en la propuesta.

## Risks / Trade-offs

- [El hilo del watcher muere o el registro falla en silencio] → Se registra un `warn!` como en Linux y Windows, y el sondeo de 60 s sigue cubriendo la detección.
- [Un callback de DA se ejecuta dentro del run loop mientras la app cierra] → El hilo es *detached* y el `AppHandle` vive tanto como el proceso. `emit` sobre una app que se está cerrando solo falla y se ignora (`let _ =`).
- [Spotlight (`mds`) o QuickLook mantienen el disco ocupado justo después de importar] → El rechazo llega como dissenter y se informa al usuario, que puede reintentar. No se fuerza el desmontaje (`kDADiskUnmountOptionForce`) para no arriesgar datos.
- [`exiftool -stay_open` mantiene archivos abiertos en el disco] → Ese proceso abre los archivos por comando y no conserva handles, y su directorio de trabajo es el de la app, no el disco. Se verifica en las tareas expulsando justo después de importar.
- [Tiempo de espera de 30 s vencido sin callback] → Se devuelve `Err("tiempo de espera agotado al expulsar")`. El dispositivo vuelve a listarse en el siguiente escaneo, como en el escenario de fallo del sistema.
- [Proyectos duplicados entre discos] → Limitación aceptada y documentada en la spec `project-management`. `refresh_open_projects` conserva el primero que inserta en el registry.
- [Las fotos importadas en Linux conservan `ECD0-9D14` y las nuevas quedan con `""`] → Es inofensivo, porque ningún flujo lee la columna. Queda así por decisión (sin migración).

## Migration Plan

No hay migración de datos. El despliegue es la siguiente versión del desktop. Para volver atrás basta con revertir el binario: las filas con `device_uuid = ""` son válidas para el esquema actual y las versiones anteriores no leen ese campo.

## Open Questions

- ¿La cadena de `DADissenterGetStatusString` trae siempre el nombre del proceso en Monterey (macOS 12) o solo en versiones recientes? Solo afecta qué tan detallado es el mensaje de error, no el comportamiento.
