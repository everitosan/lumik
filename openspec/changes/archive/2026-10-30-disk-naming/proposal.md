# Proposal

## Why

En la tablet Android, el formulario de crear proyecto y la sección "Discos" muestran todos los discos del hub con la misma descripción genérica de Android ("Unidad USB …"), así que no se sabe cuál es cuál. La etiqueta del sistema de archivos, que el desktop sí muestra, no está disponible: en la Huawei DBR-W09 Android la deja vacía (`fsLabel=`) incluso en discos exFAT que sí tienen nombre en la compu, y una app no puede leer el dispositivo de bloque para sacarla.

## What Changes

- **Nombre guardado en el disco**: un archivo `lumik/.disk.json` con el nombre del disco y su origen (`label`, copiado de la etiqueta, o `user`, puesto a mano).
- **El desktop copia la etiqueta al archivo**: al detectar un disco con escritura y con carpeta `lumik/`, si el sistema operativo da la etiqueta y el archivo falta o tiene otro nombre, lo escribe. Así la tablet muestra la etiqueta real de cualquier disco de Lumik que haya pasado por una compu.
- **Prioridad del nombre** en todas las apps: etiqueta del sistema operativo > `lumik/.disk.json` > descripción genérica.
- **El desktop usa el archivo** para nombrar los discos sin etiqueta. Incluye la app Tauri en Android, que hoy solo dice "SD Card" o "USB Drive".
- **La tablet pide un nombre** al dar acceso a un disco que no tiene `lumik/.disk.json`, y permite cambiarlo después desde "Discos" mientras el nombre sea `user`.
- **Descripción genérica más útil en la tablet**: tipo (disco o tarjeta de cámara, según tenga `lumik/` o `DCIM/`), capacidad y espacio libre, número de proyectos y el uuid corto.
- **BREAKING (spec)**: la regla "el sistema no escribe archivos en el disco para identificarlo" deja de ser absoluta. Conectar un disco sin carpeta `lumik/` sigue sin escribir nada.

## Capabilities

### New Capabilities

Ninguna.

### Modified Capabilities

- `device-detection`: la identidad sigue siendo el UUID, pero se permite escribir `lumik/.disk.json` en discos con `lumik/`. Se añaden la prioridad del nombre y el nombre puesto a mano en la tablet, y se reemplaza "Nombre amigable en Android".

## Impact

- `crates/lumik-core`: nuevo `domain/disk_name.rs` (formato del archivo, prioridad del nombre y cuándo reescribirlo), con tests.
- `apps/desktop/src-tauri`: `devices.rs` (etiqueta real separada del nombre mostrado; leer el archivo para los discos sin etiqueta, en escritorio y Android), `commands.rs::refresh_open_projects` (escribir el archivo) y una escritura atómica en `infrastructure/`.
- `apps/tablet-spike`: `modules/saf` (capacidad y espacio libre del volumen), `PrototypeScreen.tsx`, `HomeScreen.tsx`, `ProjectsDashboard.tsx` (descripción, formulario de nombre y renombrar).
- Docs: `apps/desktop/CLAUDE.md` (sección de discos) y el spike de tablet (nota en la 3.1).
- Sin cambios en `project.db`, en `lumik.db` ni en la API del servidor.
