# Flujo: Candado del proyecto

Lumik Desktop · Linux · Windows · macOS · Android

Un proyecto vive en el disco externo y el disco se mueve entre equipos. Para que dos instalaciones
de Lumik no modifiquen el mismo proyecto sin saberlo, quien trabaja en él deja un candado en su
carpeta: `.lumik-lock`, un JSON con la instalación (`install_id`, `install_name`), la hora de
inicio (`since`), la generación de `project.db` y `pending` (cambios que aún no llegaron al disco,
para el modo espejado de la tablet; en desktop siempre es `false`).

El candado se toma por **sesión de trabajo**: la vista del proyecto abierta o una operación que
escribe en él (importar, aprobar, calificar, papelera, respaldar, editar, archivar…). Las sesiones
se cuentan por proyecto: la primera escribe el candado y la última lo quita. Listar en el dashboard
solo lo lee.

## Vista general

```mermaid
flowchart LR
    A["Abrir proyecto<br/>o empezar una escritura"]
    B["leer .lumik-lock"]
    C{"¿de quién?"}
    D["escribir candado propio<br/>sesiones +1"]
    E["solo lectura<br/>aviso + Tomar el control"]
    F["trabajar"]
    G["cerrar / terminar<br/>sesiones -1<br/>la última quita el candado"]

    A --> B --> C
    C -- "nadie o propio" --> D --> F --> G
    C -- "otra instalación" --> E

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3000
    class A,E,F ui
    class D,G be
    class B db
    class C dec
```

## 1. Abrir la vista del proyecto

```mermaid
sequenceDiagram
    autonumber
    participant UI as ProjectDetail
    participant API as api.ts
    participant BE as open_project_session
    participant S as ProjectSessions
    participant FS as .lumik-lock

    UI->>API: openProjectSession(id)
    API->>BE: invoke
    BE->>S: open_view(id, carpeta, generación)
    S->>FS: leer
    alt libre o propio (cierre inesperado)
        S->>FS: escribir candado propio (temporal + renombrar)
        S-->>BE: Open
        BE-->>UI: access = open
    else de otra instalación
        S-->>BE: ReadOnly(candado)
        BE-->>UI: access = read_only + holder
        UI->>UI: ProjectLockBanner (suave si pending=false, fuerte si true)
        UI->>UI: sin importar, respaldar, papelera, rotar, calificar ni portada
    end
    Note over UI: al salir: closeProjectSession(id)<br/>la última sesión quita el candado
```

## 2. Escrituras: cada comando revisa el candado

```mermaid
flowchart TB
    A["comando que escribe<br/>(save_photo_culled, trash_photo, start_import…)"]
    B["AppState::write_session(id)"]
    C{"candado"}
    D["error project_locked:nombre<br/>sin tocar disco ni base"]
    E["api.ts emite<br/>lumik:project-locked"]
    F["la vista vuelve a mirar<br/>y pasa a solo lectura"]
    G["caso de uso<br/>(mueve archivos, UPDATE)"]
    H["drop: generación +1<br/>sesiones -1"]

    A --> B --> C
    C -- "ajeno (otro tomó el control)" --> D --> E --> F
    C -- "libre o propio" --> G --> H

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3000
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d0d0d
    class E,F ui
    class A,B,G,H be
    class C dec
    class D err
```

`delete_project` y `edit_project` cierran la base para borrar o mover la carpeta, así que no
sostienen una sesión: solo comprueban que el candado no sea ajeno (`ensure_not_locked`). Al mover
la carpeta, el candado viaja dentro y `ProjectSessions::relocate` actualiza la ruta.

## 3. Tomar el control

```mermaid
sequenceDiagram
    autonumber
    actor U as Fotógrafo
    participant D as Desktop
    participant FS as .lumik-lock (disco)
    participant T as Tablet

    Note over T,FS: la tablet abrió el proyecto y el disco se desconectó sin cerrar
    U->>D: abre el proyecto
    D->>FS: leer → candado de "Tablet de Ana"
    D-->>U: solo lectura + "Tomar el control"
    U->>D: Tomar el control
    D->>FS: escribir candado propio
    D-->>U: trabaja con normalidad
    Note over T,FS: el disco vuelve a la tablet
    T->>FS: antes de escribir: leer → candado del desktop
    T-->>T: solo lectura, no escribe en la base del disco
```

Desde el dashboard no hay vista abierta: editar, archivar o eliminar un proyecto ajeno muestra
`TakeControlDialog`; al confirmar se toma el control y se suelta enseguida, así el proyecto queda
libre y la acción sigue con normalidad.

Expulsar un disco suelta primero los candados propios de sus proyectos y después cierra sus bases.
Cerrar la app suelta todos.

## Código

| Pieza | Archivo |
|-------|---------|
| Modelo y estados del candado | `src-tauri/src/domain/lock.rs` |
| Sesiones por proyecto | `src-tauri/src/application/project_session.rs` |
| Puerto `ProjectLockStore` | `src-tauri/src/application/ports.rs` |
| Escritura atómica y nombre del equipo | `src-tauri/src/infrastructure/lock.rs` |
| `write_session`, `ensure_not_locked` | `src-tauri/src/commands.rs` |
| `open_project_session`, `close_project_session`, `take_project_control` | `src-tauri/src/commands/project.rs` |
| Identidad de la instalación | `src-tauri/src/db/queries.rs` (`install_identity`), `commands/settings.rs` |
| Expulsión | `src-tauri/src/application/use_cases/eject_device.rs` |
| Aviso y diálogo | `src/components/ProjectLockNotice.tsx` |
| Vista en solo lectura | `src/pages/Project/ProjectDetail/ProjectDetail.tsx`, `PhotoDetail.tsx` |
| Marca en el dashboard | `packages/ui/.../ProjectCard.tsx` (`lockedBy`), `src/pages/Project/Projects/Projects.tsx` |
