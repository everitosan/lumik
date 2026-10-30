# Design

## Context

- Desktop: `devices.rs::scan_mounted_devices` llena `DetectedDevice.name` con la etiqueta (`/dev/disk/by-label` en Linux, `GetVolumeInformationW` en Windows) o, si no hay, con el nombre del punto de montaje (`generate_device_name`). En macOS no se consulta la etiqueta: el nombre sale de `/Volumes/<X>`, que es la etiqueta. En la app Tauri de Android sale de `friendly_volume_name` ("SD Card" o "USB Drive").
- `refresh_open_projects` (`commands.rs`) recorre los discos montados en cada escaneo (al conectar y en el sondeo) y ya distingue los de solo lectura.
- Tablet (`apps/tablet-spike`): el nombre viene de `StorageVolume.getDescription()` (`modules/saf`, `volumes()`), que en la Huawei es genérico. `dumpsys mount` y logcat confirman `fsLabel=` vacío en dos discos exFAT con nombre.
- El candado ya marca el camino para un JSON compartido: el formato vive en `lumik-core` (`domain/lock.rs`, `to_json` y `from_json`), el desktop lo escribe de forma atómica (`infrastructure/lock.rs`) y la tablet lo usa por `lumik-mobile` con uniffi, leyendo y escribiendo por SAF.

## Goals / Non-Goals

**Goals:**
- Un solo formato de `lumik/.disk.json` y una sola regla de prioridad y reescritura, en el núcleo y con tests.
- Escribir solo cuando hace falta: ni en cada sondeo ni en discos sin `lumik/`.

**Non-Goals:**
- Leer la etiqueta por la API oculta de Android (`VolumeInfo.fsLabel`): en la Huawei viene vacía.
- Cambiar la etiqueta real del sistema de archivos desde Lumik.
- Usar `.disk.json` como identidad del disco: la identidad sigue siendo el UUID de cada máquina.
- Tablet iPad: el spike sigue en pausa.

## Decisions

### 1. Formato en `lumik-core::domain::disk_name`

```json
{ "version": 1, "name": "SEAGATE", "source": "label", "updated_at": "2026-10-30T13:34:40Z" }
```

`DiskName { name, source: Label | User, updated_at }` con `to_json` y `from_json`. `from_json` devuelve `None` si el JSON es inválido, la versión no se conoce o el nombre viene vacío tras quitar espacios. Hay dos funciones puras:
- `display_name(label: Option<&str>, file: Option<&DiskName>, fallback) -> String`: la prioridad del spec.
- `label_update(label: &str, file: Option<&DiskName>, now) -> Option<DiskName>`: `Some` cuando hay que reescribir (falta el archivo o el nombre es otro).

*Alternativa:* lógica separada en Rust (desktop) y en TS (tablet). Se descarta por lo mismo que con `.lumik-lock`: dos implementaciones del mismo JSON tarde o temprano divergen.

### 2. `DetectedDevice.label` separado de `name`

Se añade `label: Option<String>` con la etiqueta real:
- Linux: `get_volume_label_linux`.
- Windows: `get_volume_label_windows`.
- macOS: el nombre de la carpeta en `/Volumes/`.
- Android (Tauri): `None`.

`name` pasa a calcularse con `display_name(label, archivo, fallback)`. Hace falta separarlos para saber si hay etiqueta real: en Linux el respaldo del nombre puede ser el UUID (udisks monta en `/media/<user>/<UUID>`) y en Windows "Drive E", y ninguno de los dos se debe copiar al disco.

*Alternativa:* adivinar desde `name` si es etiqueta. Se descarta por frágil.

*macOS:* si hay dos discos con la misma etiqueta, macOS monta el segundo como `/Volumes/SEAGATE 1`. Se acepta ese nombre tal cual: es lo que el usuario ve en Finder.

### 3. El desktop escribe en `refresh_open_projects`

Dentro del bucle por dispositivo, después del filtro de solo lectura y de los que se están expulsando: si `label` es `Some` y existe `{mount}/lumik/`, se lee `lumik/.disk.json` y, si `label_update` devuelve `Some`, se escribe de forma atómica (temporal y `rename`, como `FsLockStore`). Lo hace un `FsDiskNameStore` en `infrastructure/disk_name.rs` con `read(mount)` y `write(mount, &DiskName)`.

La lectura de `scan_mounted_devices` para el nombre usa el mismo store. Leer un archivo pequeño por disco en cada sondeo (cada 60 s, o cada 10 s en Android) cuesta poco. La escritura solo ocurre cuando cambia el nombre. Un fallo al escribir solo se registra en el log (`warn`): no debe romper el escaneo.

*Alternativa:* escribir solo al crear un proyecto o importar. Se descarta: el usuario prefiere que cualquier disco de Lumik que pase por la compu quede nombrado para la tablet.

### 4. Android Tauri lee el archivo

`scan_mounted_devices_android` lee `{mount}/lumik/.disk.json` (la app tiene acceso al almacenamiento) y usa `friendly_volume_name` solo si no hay archivo. Nunca escribe: no tiene etiqueta que copiar.

### 5. Tablet: `lumik-mobile` + SAF

`lumik-mobile` expone por uniffi:
- `disk_name_view(json: Option<String>) -> Option<DiskNameView { name, from_label }>`
- `user_disk_name_json(name) -> String`

El JS lee y escribe el archivo por SAF con `readFdString` y `writeFdString`. Nunca se sobrescribe: en la Huawei, sobrescribir un archivo existente por SAF lo dejó vacío. Se escribe `.disk.json.tmp`, se borra el anterior, se renombra el temporal y se vuelve a leer para comprobar el nombre. El archivo se crea como binario (`application/octet-stream`); con `application/json`, Android le añade la extensión, el mismo problema que con el candado.

- **Cuándo se pide el nombre:** en `grantVolume` (`PrototypeScreen.tsx`), después de `requestVolumeAccess`, si el volumen tiene escritura y no tiene `.disk.json`, se abre un modal "Nombre del disco" con la sugerencia de la descripción genérica, y los botones "Guardar" y "Omitir". El mismo modal aparece en el menú del disco en "Discos" ("Cambiar nombre"), solo si `from_label` es falso.
- **Descripción genérica:** tipo según el contenido (`lumik/` o `DCIM/`, leído por SAF desde la raíz del permiso) y número de proyectos (ya se cuentan al buscar proyectos).
- **Capacidad y espacio libre:** en Kotlin, `StatFs(volume.directory)` (API 30 o más). Si falla o devuelve 0, el campo se omite. Si en la Huawei no funciona, se prueba `DocumentsContract` (columnas `COLUMN_CAPACITY_BYTES` y `COLUMN_AVAILABLE_BYTES` de la raíz de `ExternalStorageProvider`).

`SafVolume` suma `totalBytes?` y `freeBytes?`. `diskLabel()` pasa a usar `display_name` (sin etiqueta) y añade el uuid corto.

## Risks / Trade-offs

- [Escribir en un disco solo por conectarlo sorprende o falla en discos llenos] → Solo en discos que ya tienen `lumik/` y escritura, solo cuando cambia el nombre, de forma atómica y con el fallo solo en el log.
- [Se renombra el disco en otra compu sin Lumik] → La próxima vez que lo vea un desktop con Lumik se corrige; mientras tanto la tablet muestra el nombre anterior.
- [El usuario nombra el disco en la tablet y una compu lo cambia por la etiqueta] → Es la regla acordada ("respetar el del disco"). Si eso pasa, el menú de la tablet explica que el nombre es la etiqueta.
- [Dos tablets o equipos escriben a la vez] → Gana la última escritura. Solo cambia un nombre y no hay datos que perder.
- [`StatFs` sobre `/storage/<uuid>` falla con SAF puro] → La capacidad es opcional en el spec. Se prueba la alternativa de `DocumentsContract`.
- [`version` desconocida en el futuro] → Se trata como si no hubiera archivo. El desktop la sobrescribiría al copiar la etiqueta, así que el formato debe crecer añadiendo campos sin subir la versión, salvo que el cambio rompa la compatibilidad.

## Migration Plan

No hay datos que migrar: los discos sin `.disk.json` se comportan como hoy y el desktop los va nombrando conforme los detecta. Para volver atrás basta con quitar el código: `.disk.json` es un archivo extra que ninguna versión anterior lee.
