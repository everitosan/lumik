# WIP · Panorama — Respaldo de `_exported` a la nube (SaaS multi-tenant)

> **Estado: WIP.** Diseño propuesto, aún no implementado.

Mapa del subsistema completo de respaldo. Cada flujo de detalle tiene su propio documento:

- [Login / cuenta](wip-flujo-login-cuenta.md)
- [Conectar proveedor](wip-flujo-conectar-proveedor.md)
- [`ensure_catalog_synced`](wip-flujo-sync-catalogo.md)
- [Respaldar `_exported` (desktop)](wip-flujo-respaldar-exported.md)
- [Respaldo desde iPad/web](wip-flujo-respaldo-ipad.md)

## 1. Componentes por capa

```mermaid
flowchart TB
    subgraph ACT["Fotógrafo"]
        direction LR
        a1(["Fotógrafo (escritorio)"]) ~~~ a2(["Fotógrafo (campo / iPad)"])
    end

    subgraph DESK["Lumik Desktop — Tauri · Linux/Windows/Android"]
        direction LR
        d_login["login → JWT<br/>(AuthClient)"] ~~~ d_sync["sync_exported<br/>(_exported ↔ photo)"] ~~~ d_ensure["ensure_catalog_synced<br/>(al abrir proyecto)"] ~~~ d_backup["backup_project<br/>(sube _exported)"] ~~~ d_badge["badge por-foto<br/>(pending/uploaded/failed)"] ~~~ d_disk[("project.db — disco externo<br/>_media / _culled / _exported")]
    end

    subgraph WEB["Lumik Web / iPad — dashboard"]
        direction LR
        w_login["login → JWT"] ~~~ w_list["lista proyectos<br/>GET /v1/projects"] ~~~ w_upload["sube contra photoId<br/>POST /v1/backup"] ~~~ w_connect["conectar proveedor<br/>(OAuth / credenciales)"]
    end

    subgraph SRV["Lumik Server SaaS — axum + rclone"]
        direction LR
        s_auth["Auth<br/>/v1/auth/* (argon2id, JWT)"] ~~~ s_sync["sync / listado<br/>(scoped por photographer_id)"] ~~~ s_backup["POST /v1/backup<br/>resuelve remotes"] ~~~ s_rclone["rclone rcd<br/>(copy / fan-out)"] ~~~ s_oauth["OAuth connect/callback"] ~~~ s_db[("catalog.db — Postgres<br/>photographer · project · photo · remote")]
    end

    subgraph CLOUD["Nube de terceros (por fotógrafo)"]
        direction LR
        c1["Google Drive"] ~~~ c2["Dropbox"] ~~~ c3["OneDrive"] ~~~ c4["Mega"] ~~~ c5["Nextcloud"]
    end

    ACT ~~~ DESK ~~~ WEB ~~~ SRV ~~~ CLOUD

    classDef actor fill:#eceff1,stroke:#546e7a,color:#1a2327
    classDef ui fill:#b3e5fc,stroke:#0288d1,color:#07293a
    classDef web fill:#e1bee7,stroke:#8e24aa,color:#2a0d33
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#d1c4e9,stroke:#5e35b1,color:#1f1033
    classDef cloud fill:#c8e6c9,stroke:#388e3c,color:#10281a
    class a1,a2 actor
    class d_login,d_sync,d_ensure,d_backup,d_badge ui
    class w_login,w_list,w_upload,w_connect web
    class s_auth,s_sync,s_backup,s_rclone,s_oauth be
    class d_disk,s_db db
    class c1,c2,c3,c4,c5 cloud
```

## 2. Rutas del flujo (numeradas)

```mermaid
flowchart TB
    d_login["Desktop: login → JWT"]
    d_sync["Desktop: sync_exported"]
    d_disk[("project.db<br/>disco externo")]
    d_ensure["Desktop: ensure_catalog_synced"]
    d_backup["Desktop: backup_project"]
    d_badge["Desktop: badge por-foto"]

    w_login["Web/iPad: login → JWT"]
    w_list["Web/iPad: lista proyectos"]
    w_upload["Web/iPad: POST /v1/backup"]
    w_connect["Web/iPad: conectar proveedor"]

    s_auth["Server: Auth /v1/auth/*"]
    s_sync["Server: sync / listado<br/>scoped por photographer_id"]
    s_backup["Server: POST /v1/backup<br/>resuelve remotes"]
    s_rclone["Server: rclone rcd"]
    s_oauth["Server: OAuth connect/callback"]
    s_db[("catalog.db<br/>Postgres")]

    gdrive["Google Drive"]
    nextcloud["Nextcloud"]

    d_login -->|login| s_auth
    w_login -->|login| s_auth
    d_sync -.->|1 · lee/actualiza| d_disk
    d_ensure -->|2 · check / publish / reconcile| s_sync
    d_backup -->|3 · POST archivo| s_backup
    w_list --> s_sync
    w_upload --> s_backup
    w_connect --> s_oauth

    s_sync --> s_db
    s_backup -->|actualiza backup_*| s_db
    s_oauth -.->|guarda token| s_db
    s_backup --> s_rclone
    s_rclone -->|copy| gdrive
    s_rclone -.->|fan-out| nextcloud
    s_backup -.->|recibo → badge| d_badge

    classDef ui fill:#b3e5fc,stroke:#0288d1,color:#07293a
    classDef web fill:#e1bee7,stroke:#8e24aa,color:#2a0d33
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#d1c4e9,stroke:#5e35b1,color:#1f1033
    classDef cloud fill:#c8e6c9,stroke:#388e3c,color:#10281a
    class d_login,d_sync,d_ensure,d_backup,d_badge ui
    class w_login,w_list,w_upload,w_connect web
    class s_auth,s_sync,s_backup,s_rclone,s_oauth be
    class d_disk,s_db db
    class gdrive,nextcloud cloud
```

## Notas

| Punto | Detalle |
|-------|---------|
| Multi-tenant | Todo endpoint de listado o sync va *scoped* por `photographer_id`, siempre tomado del JWT. |
| Qué se respalda | Solo `_exported/` (los JPG entregables), no los RAW de `_media/`. |
| Fan-out | Un `POST /v1/backup` puede copiar a varios remotes a la vez vía `rclone rcd`. |
| Remotes por fotógrafo | La tabla `remote` de `catalog.db` guarda un token cifrado por fotógrafo y proveedor. |
| Dos bases | `project.db` (SQLite, en el disco externo, junto a las fotos) y `catalog.db` (Postgres, en el server). |
