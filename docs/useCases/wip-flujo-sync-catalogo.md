# WIP · Flujo: `ensure_catalog_synced` (al abrir un proyecto)

> **Estado: WIP.** Diseño propuesto, aún no implementado.

Operación **idempotente** que corre al abrir un proyecto. Garantiza la precondición de que el
catálogo exista en el server antes de poder respaldar desde cualquier dispositivo.

```mermaid
sequenceDiagram
    autonumber
    participant D as Desktop (al abrir proyecto)
    participant SV as Server SaaS
    participant DB as catalog.db / project.db

    D->>D: abrir proyecto
    alt no hay sesión o no hay red
        D->>D: modo solo-local (sin sincronizar)
    else hay sesión + red
        D->>SV: GET /v1/projects/{id}
        SV->>SV: ¿existe en el catálogo del fotógrafo?
        SV-->>D: 200 o 404
        alt 404 — no existe
            D->>SV: publish: POST /sync/projects + /photos
            SV->>DB: upsert project + photos<br/>(photographer_id del JWT)
        else 200 — ya existe
            D->>SV: reconcile: GET /backup/status
            SV-->>D: backup_* por foto
            D->>DB: actualiza project.db en disco
        end
        D->>D: grid con estado por-foto
    end
```

## Notas

| Punto | Detalle |
|-------|---------|
| Idempotente | Se puede llamar en cada apertura de proyecto sin efectos duplicados. |
| Precondición | Es lo que habilita respaldar desde iPad/web: el catálogo tiene que existir primero. |
| Scoping | El `photographer_id` siempre sale del JWT, nunca del cliente. |
| Sin red | Degrada a modo solo-local; no bloquea la apertura del proyecto. |
| Dos direcciones | `publish` sube el catálogo (404); `reconcile` baja los `backup_*` y actualiza `project.db` en disco (200). |
