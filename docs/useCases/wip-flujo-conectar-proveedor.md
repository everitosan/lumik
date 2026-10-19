# WIP · Flujo: Conectar proveedor de nube (por fotógrafo)

> **Estado: WIP.** Diseño propuesto, aún no implementado.

Cada fotógrafo conecta sus propias cuentas de nube. La app OAuth (`client_id` / `client_secret`)
es **una sola, de Lumik, y vive en el server**; el `refresh_token` resultante es **por fotógrafo**
y se guarda cifrado.

## Camino OAuth (Google Drive, Dropbox, OneDrive)

```mermaid
sequenceDiagram
    autonumber
    participant W as Dashboard web (logueado)
    participant SV as Server SaaS + rclone
    participant P as Proveedor de nube
    participant DB as catalog.db

    W->>SV: Clic 'Conectar Google Drive'<br/>POST /v1/providers/gdrive/connect
    Note over SV: state ligado a photographer_id
    SV->>SV: arma auth_url
    SV-->>W: auth_url
    W->>P: navegador → pantalla de consentimiento
    P->>P: login + aceptar (cuenta del fotógrafo)
    P-->>SV: redirect + code → GET /oauth/callback
    SV->>P: canjea code
    P-->>SV: refresh_token
    SV->>DB: guarda remote(photographer_id, 'gdrive', token cifrado)
```

## Camino sin OAuth (Mega, Nextcloud)

```mermaid
flowchart LR
    paste["Dashboard web:<br/>pegar credenciales"]
    store["Server: guardar como remote<br/>(mismo formato, token cifrado)"]
    db[("catalog.db · tabla remote")]

    paste -->|sin OAuth| store --> db

    classDef ui fill:#f3e5f5,stroke:#8e24aa,color:#2a0d33
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#d1c4e9,stroke:#5e35b1,color:#1f1033
    class paste ui
    class store be
    class db db
```

## Notas

| Punto | Detalle |
|-------|---------|
| App OAuth | `client_id` / `client_secret` son de Lumik, uno solo, en el server. El fotógrafo nunca los ve. |
| Token por fotógrafo | El `refresh_token` resultante es por fotógrafo y se guarda **cifrado** en `catalog.db`. |
| `state` | Va ligado al `photographer_id` para amarrar el callback al fotógrafo correcto. |
| Proveedores sin OAuth | Mega y Nextcloud se conectan pegando credenciales y desembocan en el mismo registro `remote`. |
