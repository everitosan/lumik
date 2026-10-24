# WIP · Flujo: Respaldar `_exported` (desktop)

> **Estado: WIP.** Diseño propuesto, aún no implementado.

Sube los JPG de `_exported/` a los remotes del fotógrafo, con fan-out a varios proveedores en
una sola petición y deduplicación por hash.

```mermaid
sequenceDiagram
    autonumber
    participant D as Desktop (backup_project)
    participant SV as Server SaaS
    participant C as Nube (remotes del fotógrafo)
    participant DB as catalog.db

    D->>D: sync_exported (_exported ↔ photo, set jpg_path)
    D->>D: list_pending_backup
    D->>D: selector de proveedores (targets: uno/varios/todos)
    D->>SV: POST /v1/backup (multipart)<br/>file + hash + targets
    SV->>SV: resuelve remotes del fotógrafo + path (ProjectFolder)
    alt el hash ya existe en el remote
        SV->>DB: skip — marca backup_status sin volver a subir
    else hash nuevo
        SV->>C: rclone copy (fan-out a cada target)
        C-->>SV: Lumik/{año}/{mes}/{día}_{slug}/_exported/
        SV->>DB: backup_status=uploaded, backup_url, provider
    end
    SV-->>D: results[] por proveedor
    D->>D: update backup_* + badge por-foto
```

## Notas

| Punto | Detalle |
|-------|---------|
| Estado por-foto | `backup_status = uploaded` solo cuando **todos** los targets subieron; `failed` si alguno falla. El detalle viene en `results[]`, un entry por proveedor. |
| Deduplicación | Si el hash ya está en el remote, se omite la subida y solo se actualiza el estado. |
| Ruta en la nube | `Lumik/{año}/{mes}/{día}_{slug}/_exported/`, espejo de la estructura local. |
| Precondición | El catálogo debe existir: ver [`ensure_catalog_synced`](wip-flujo-sync-catalogo.md). |

## Seguimiento desde la barra lateral

El respaldo corre en `BackupProvider` (a nivel de app), así que se sigue desde cualquier pantalla.
Cada job aparece como una tarjeta en la sección **Respaldos** de la barra lateral
(`nombre del proyecto · subidos/total`). La sección solo se muestra mientras hay jobs (en curso o
con errores), sin depender de la sesión. Al pulsarla se abre el modal de detalle archivo por
archivo, montado una sola vez en `BackupModalHost`, igual que desde el proyecto.

```mermaid
stateDiagram-v2
    [*] --> EnCurso: startBackup o reanudado con sesión
    EnCurso --> [*]: todos subidos, la tarjeta desaparece
    EnCurso --> ConErrores: algún archivo falló, sigue en la cola
    ConErrores --> EnCurso: Reintentar solo los fallidos
    ConErrores --> ConErrores: Reintentar sin el disco, avisa
    ConErrores --> Descartado: Descartar
    Descartado --> EnCurso: próximo arranque, la cola lo reanuda
```

```mermaid
flowchart TD
    close(["Cerrar la ventana"])
    running_q{"¿Hay jobs<br/>subiendo?"}
    exit_now(["Se cierra sin preguntar"])
    ask["Diálogo: 'N respaldos en curso'<br/>Seguir respaldando / Cerrar de todos modos"]
    keep(["La app sigue abierta<br/>y la subida continúa"])
    destroy(["Se cierra: el respaldo queda en la cola<br/>y se reanuda al arrancar"])

    close --> running_q
    running_q -->|No| exit_now
    running_q -->|Sí| ask
    ask -->|Seguir respaldando| keep
    ask -->|Cerrar de todos modos| destroy
```

| Punto | Detalle |
|-------|---------|
| Barra lateral | Respaldos aparece solo con jobs y se oculta al terminar el último. Respaldos y Almacenamiento se pueden colapsar (estado en `global_db`). Colapsada, Respaldos muestra `en curso · subidos/total` o cuántos terminaron con errores. Se expande sola al lanzar un respaldo, no al reanudarlo. |
| Sin sesión | Respaldar muestra un aviso con "Ir a Ajustes" (pestaña Respaldo) en lugar del modal; la cola no se reanuda hasta iniciar sesión (ver [login](wip-flujo-login-cuenta.md)). |
| Confirmación al cerrar | `onCloseRequested` en el frontend + `ask` del plugin de diálogo; cerrar usa `destroy()` (`core:window:allow-destroy`). |

## Borrar el respaldo del proyecto

Solo desde el **portal web** (menú de la tarjeta → "Borrar respaldo", en proyectos con fotos
subidas), nunca desde la app de escritorio, para no borrar un respaldo por error. Hay que escribir
el nombre del proyecto para confirmar. Borra la carpeta del proyecto en la nube y sus fotos del
catálogo; el disco del fotógrafo no se toca y el siguiente respaldo sube todo de nuevo.

```mermaid
sequenceDiagram
    autonumber
    actor U as Fotógrafo
    participant W as Portal web (DeleteBackupModal)
    participant SV as Server (DELETE /v1/projects/{id}/backup)
    participant C as Nube (remotes del fotógrafo)
    participant DB as Postgres (catálogo)

    U->>W: Borrar respaldo → escribe el nombre del proyecto
    W->>SV: DELETE /v1/projects/{id}/backup
    SV->>DB: backup_dir del proyecto (del fotógrafo, o 404)
    opt hay backup_dir
        loop cada remote
            SV->>C: rclone purge {backup_dir}<br/>(no encontrada = ya borrada, no es error)
            SV->>C: rclone rmdir carpeta del proyecto si quedó vacía
        end
    end
    alt un remote falla
        SV-->>W: error (el catálogo no se toca, se puede reintentar)
    else todo bien
        SV->>DB: DELETE photo del proyecto<br/>backup_dir = NULL, workflow_status = NULL
        SV-->>W: deleted_photos, purged_remotes
        W->>W: recarga los proyectos
    end
```

| Punto | Detalle |
|-------|---------|
| Para qué | Volver a subir un proyecto cuyo respaldo se borró a mano en la nube: el catálogo lo seguía dando por subido. |
| Mega | `rclone purge` manda los archivos a la papelera del proveedor (sin `hard_delete`). |
| Proyecto en el catálogo | Se queda (el portal lo muestra sin fotos); solo se borran sus fotos, el ancla `backup_dir` y el estado fijado. |
| Desktop | No cambia su estado local: el proyecto sigue mostrando sus proveedores hasta el siguiente respaldo. El modal de respaldo consulta el catálogo, así que ofrece subir todo de nuevo. |
| Código | Server `backup::handlers::delete_backup`, `Rclone::purge`/`rmdir`; web `components/DeleteBackupModal.tsx`, `pages/Dashboard.tsx`, `backupApi.deleteBackup`; `ProjectCard.onDeleteBackup` en `@lumik/ui`. |
