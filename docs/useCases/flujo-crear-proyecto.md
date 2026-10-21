# Flujo: Crear Proyecto

Lumik Desktop · Linux · Windows · Android

Alta de un proyecto desde la pantalla *Proyectos*: el formulario exige nombre y dispositivo
destino, el backend verifica que ese dispositivo esté montado y crea la carpeta del proyecto
con su propia `project.db`.

## Vista general

```mermaid
flowchart LR
    A["Formulario<br/>CreateProjectModal"]
    B["invoke<br/>'create_project'"]
    C["Resolver dispositivo<br/>por UUID"]
    D["Crear carpeta<br/>+ project.db"]
    E["Refrescar dashboard"]

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

## 1. Formulario y validación (frontend)

```mermaid
flowchart TD
    start(["Usuario en pantalla<br/>Proyectos"])
    trigger["Clic en 'Nuevo Proyecto'<br/>(o atajo de teclado N)"]
    modal["Abrir CreateProjectModal"]
    form["Formulario<br/>• Nombre del proyecto — requerido<br/>• Descripción — opcional<br/>• Fecha de sesión, flatpickr — opcional<br/>• Dispositivo destino, DriveSelector — requerido"]
    validate{"¿Formulario<br/>válido?"}
    val_error["Mostrar errores de<br/>validación en formulario"]
    invoke["api.createProject()<br/>invoke('create_project', { project })"]
    next(["continúa en §2"])

    start --> trigger --> modal --> form --> validate
    validate -->|No| val_error
    val_error -.->|corregir| form
    validate -->|Sí| invoke --> next

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    class start,next term
    class trigger,modal,form ui
    class invoke ipc
    class validate dec
    class val_error err
```

## 2. Resolución del dispositivo destino (backend)

La detección del UUID de volumen es lo único que cambia por plataforma; el resultado se
normaliza siempre a `uuid + nombre + mount_point`, más `read_only` si el sistema lo
montó sin escritura (en ese caso el proyecto no se crea).

```mermaid
flowchart TD
    cmd["Rust: commands.rs ::<br/>create_project(state, CreateProject)"]
    scan["scan_mounted_devices()"]

    subgraph PLAT["Detección de UUID de dispositivo"]
        direction TB
        plat_q{"Plataforma"}
        linux["Linux<br/>/dev/disk/by-uuid/<br/>o blkid -s UUID"]
        windows["Windows<br/>GetVolumeInformationW()<br/>serial XXXX-XXXX"]
        android["Android<br/>/proc/mounts (vfat/exfat)<br/>UUID = dir en /storage/UUID/"]
        macos["macOS<br/>diskutil info (punto de montaje<br/>o, si falla, disco BSD p. ej. disk4s4)"]
    end

    merge["Lista de dispositivos disponibles<br/>(uuid + nombre + mount_point + read_only)"]
    mounted_q{"¿Dispositivo con<br/>ese UUID montado?"}
    mount_error["Error: Device not mounted<br/>→ retornar error a frontend"]
    ro_q{"¿Montado en<br/>solo lectura?"}
    ro_error["Error: El dispositivo está montado<br/>en solo lectura → nada se escribe"]
    err_end(["Error visible en UI"])
    build_path["Construir ruta del proyecto<br/>{mount_point}/lumik/{año}/{mes}/{día}_{slug}/"]
    gen_uuid["Generar project_id (UUID v4)"]
    next(["continúa en §3"])

    cmd --> scan --> plat_q
    plat_q -->|Linux| linux --> merge
    plat_q -->|Windows| windows --> merge
    plat_q -->|Android| android --> merge
    plat_q -->|macOS| macos --> merge
    merge --> mounted_q
    mounted_q -->|No| mount_error --> err_end
    mounted_q -->|Sí| ro_q
    ro_q -->|Sí| ro_error --> err_end
    ro_q -->|No| build_path --> gen_uuid --> next

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef plat fill:#e1d5e7,stroke:#9673a6,color:#2a1a33
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d1210
    class next term
    class cmd,scan,merge,build_path,gen_uuid be
    class plat_q,linux,windows,android,macos plat
    class mounted_q,ro_q dec
    class mount_error,ro_error,err_end err
```

## 3. Persistencia y cierre

```mermaid
flowchart TD
    db_create["ProjectDatabase::create()<br/>1. fs::create_dir_all(project_dir)<br/>2. Abrir SQLite → project.db<br/>3. Ejecutar PROJECT_SCHEMA — inicializar tablas<br/>4. INSERT INTO project (id, name, description,<br/>created_at, session_date, creator_id)"]
    dirs["Crear directorio _exported/<br/>(vacío, reservado para exportaciones)"]
    global_db[("register_or_update_device()<br/>→ global lumik.db · tabla device")]
    open_map["Agregar ProjectDatabase al<br/>mapa open_projects (en memoria)"]
    return_fe["Retornar struct Project al frontend (vía IPC)"]
    close_modal["Cerrar modal + llamar<br/>get_projects_dashboard() para refrescar la lista"]
    fin(["Nuevo proyecto visible<br/>en pantalla Proyectos"])

    note_disk["Ver 'Estructura en disco' abajo"]
    note_global["Ver 'Ruta de la base global' abajo"]

    db_create --> dirs --> global_db --> open_map --> return_fe --> close_modal --> fin
    db_create -.- note_disk
    global_db -.- note_global

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef note fill:#fff9c4,stroke:#f9a825,color:#3d3300
    class fin term
    class close_modal ui
    class return_fe ipc
    class db_create,dirs,global_db,open_map db
    class note_disk,note_global note
```

## Estructura en disco

Lo que este flujo crea está marcado con `← nuevo`; el resto aparece al importar.

```
{mount_point}/lumik/
└── {año}/{mes}/{día}_{slug}/
    ├── project.db        ← nuevo
    ├── _exported/        ← nuevo
    ├── _media/
    ├── .thumbs/
    └── .previews/        (solo archivos RAW)
```

## Ruta de la base global

| Plataforma | Ruta de `lumik.db` |
|------------|--------------------|
| Linux | `~/.local/share/com.lumik.app/` |
| Windows | `%APPDATA%\com.lumik.app\` |
| Android | `/data/data/com.lumik.app/` |

## Notas

| Punto | Detalle |
|-------|---------|
| Validación | Solo *nombre* y *dispositivo destino* son obligatorios. |
| Caso de error | Si el dispositivo con ese UUID no está montado, el comando retorna error y la UI lo muestra **sin crear nada**. |
| Dos bases | `project.db` vive en el disco externo junto a las fotos; `lumik.db` es global por instalación y solo registra dispositivos. |
