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
