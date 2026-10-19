# WIP · Flujo: Respaldo desde iPad/web + reconciliación

> **Estado: WIP.** Diseño propuesto, aún no implementado.

El fotógrafo sube desde campo contra fotos **ya publicadas** en el catálogo por la desktop.
Más tarde, al abrir el proyecto en la desktop, el estado se reconcilia y aparece el badge.

```mermaid
sequenceDiagram
    autonumber
    participant I as iPad / Web dashboard
    participant SV as Server SaaS + rclone
    participant C as Nube
    participant D as Desktop (más tarde)

    I->>SV: login → JWT
    I->>SV: GET /v1/projects (solo del fotógrafo)
    SV-->>I: lista de proyectos
    I->>I: elige proyecto/foto (photoId del catálogo)
    I->>SV: POST /v1/backup
    SV->>SV: resuelve remotes + path
    SV->>C: rclone copy
    C-->>SV: archivo en la nube
    SV->>SV: actualiza catalog, backup_status=uploaded

    Note over D: más tarde, en el escritorio
    D->>D: abre proyecto (disco) → ensure_catalog_synced
    D->>SV: reconcile: GET /backup/status
    SV-->>D: backup_status por foto
    D->>D: badge 'uploaded' ✔
```

## Notas

| Punto | Detalle |
|-------|---------|
| Alcance del MVP | El iPad **solo** sube contra fotos ya publicadas por la desktop (usa el `photoId` del catálogo). Las fotos que nazcan solo en el iPad quedan fuera del MVP. |
| Reconciliación | El badge en desktop no es inmediato: aparece al abrir el proyecto y correr [`ensure_catalog_synced`](wip-flujo-sync-catalogo.md) + `GET /backup/status`. |
| Scoping | `GET /v1/projects` devuelve solo los proyectos del fotógrafo del JWT. |
