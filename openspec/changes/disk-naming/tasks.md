# Tasks

## 1. Núcleo

- [x] 1.1 Crear `crates/lumik-core/src/domain/disk_name.rs` con `DiskName` (`name`, `source: Label | User`, `updated_at`), `to_json`/`from_json` (versión 1; `None` si el JSON es inválido, la versión no se conoce o el nombre viene vacío), `display_name` y `label_update` (design §1). Tests: ida y vuelta, archivo dañado o con versión desconocida, prioridad etiqueta > archivo > respaldo, `label_update` con el archivo ausente, con otro nombre (también si su origen es `user`) y con el mismo nombre (`None`). Verificar con `cargo test -p lumik-core disk_name`.

## 2. Desktop

- [x] 2.1 Añadir `label: Option<String>` a `DetectedDevice` y llenarlo por plataforma (Linux por `by-label`, Windows por `GetVolumeInformationW`, macOS con el nombre de `/Volumes/<X>`, Android `None`). Mantener el tipo de TS en `apps/desktop/src/lib/types.ts` (y `packages/ts-types` si lo replica). Verificar con `cargo test` del desktop y `pnpm --filter @lumik/desktop build`.
- [x] 2.2 Crear `infrastructure/disk_name.rs` con `FsDiskNameStore` (`read(mount)` y `write(mount, &DiskName)` atómico, con temporal y `rename`, como `FsLockStore`). Tests con un directorio temporal: escribir, leer, sin restos del temporal, y archivo dañado = `None`. Verificar con `cargo test` del desktop.
- [x] 2.3 Calcular `DetectedDevice.name` con `display_name(label, archivo, respaldo actual)` en `scan_mounted_devices` (escritorio) y `scan_mounted_devices_android` (respaldo: `friendly_volume_name`). Ajustar el test existente de `friendly_volume_name` si cambia y añadir uno del escenario "Disco con nombre guardado". Verificar con `cargo test` del desktop.
- [x] 2.4 En `refresh_open_projects`, para cada disco que no esté en expulsión, tenga escritura, tenga `label` y tenga `{mount}/lumik/`, escribir `lumik/.disk.json` cuando `label_update` lo pida; si falla, `warn!` y seguir. Verificar en Linux con un disco real: primer escaneo crea el archivo; segundo escaneo no lo reescribe (misma fecha de modificación); un disco sin `lumik/` queda sin archivo; un `.disk.json` con origen `user` y otro nombre pasa a `label`.
- [x] 2.5 Actualizar `apps/desktop/CLAUDE.md`: `lumik/.disk.json` en "Estructura de carpetas en disco externo", `domain/disk_name.rs` e `infrastructure/disk_name.rs` en la estructura, y la regla del nombre en "Identidad". Verificar leyendo el diff de la sección.

## 3. Tablet Android (`apps/tablet-spike`)

- [x] 3.1 En `crates/lumik-mobile`, exponer `disk_name_view(json)` y `user_disk_name_json(name)` (design §5), y regenerar `@lumik/core-native`. Verificar con `cargo test -p lumik-mobile` y que el build de release de Android compila.
- [x] 3.2 En `modules/saf`, añadir `totalBytes` y `freeBytes` a `volumes()` con `StatFs(volume.directory)` y, si fallan en la Huawei, con las columnas de capacidad de la raíz de `DocumentsContract` (se omiten si no hay dato). Verificar en la DBR-W09 con el disco y la SD en el hub: los números coinciden con Ajustes → Almacenamiento.
- [x] 3.3 Descripción genérica: tipo por contenido (`lumik/` → "Disco", `DCIM/` → "Tarjeta de cámara", sin acceso → "Disco"), capacidad, espacio libre y número de proyectos, más el uuid corto en cualquier nombre; `diskLabel()` usa `disk_name_view` del `.disk.json` y si no hay, la descripción genérica. Verificar en la tablet que el formulario de crear proyecto, el inicio y "Discos" distinguen el disco de la SD.
- [ ] 3.4 Modal "Nombre del disco" después de `requestVolumeAccess` cuando el volumen tiene escritura y no hay `.disk.json` (sugerencia = descripción genérica; "Guardar" crea `lumik/` si hace falta y escribe el archivo como binario; "Omitir" no escribe). Verificar en la tablet: nombrar un disco nuevo, desconectar y reconectar (sale con el nombre y sin modal), y que "Omitir" no deja archivo.
- [x] 3.5 "Cambiar nombre" en el menú del disco en "Discos": solo con origen `user`; con origen `label` mostrar que es la etiqueta del disco. Verificar en la tablet con un disco de cada origen.
- [x] 3.6 Anotar el resultado en la 3.1 de `openspec/changes/tablet-app-spike/tasks.md` (incluido que `fsLabel` viene vacío en la Huawei). Verificar leyendo la nota.

## 4. Integración

- [x] 4.1 Ida y vuelta con los discos reales: conectar en el desktop (Linux) los dos discos del hub (EEFB-F27B y 0D6F-0A43), comprobar que se crea `lumik/.disk.json` con su etiqueta, y luego conectarlos a la tablet y ver los mismos nombres sin modal. Nombrar en la tablet un disco sin etiqueta, abrirlo en el desktop y ver ese nombre. Correr `cargo test`, `pnpm --filter @lumik/desktop build` y `openspec validate disk-naming --strict`.

## Workflow follow-up

- Archivar el change tras la prueba 4.1 y sincronizar el delta con `openspec/specs/device-detection/spec.md`.
- La tablet es un spike: cuando exista la app de tablet real, sus specs deben heredar los requisitos "Nombrar un disco en la tablet" y "Descripción genérica en la tablet".
