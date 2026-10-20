# WIP · Flujo: Login / Cuenta de fotógrafo (SaaS)

> **Estado: WIP.** Diseño propuesto, aún no implementado.

Autenticación del fotógrafo contra el server SaaS. La clave del diseño es que **hay una sola
entidad `photographer`**: el `creator_id` local y el `photographer_id` del server son la misma
identidad.

```mermaid
sequenceDiagram
    autonumber
    actor U as Fotógrafo
    participant PR as Proyecto (desktop)
    participant FE as Ajustes → Respaldo / Web (frontend)
    participant AC as Cliente HTTP (AuthClient)
    participant SV as Server SaaS (axum)
    participant DB as catalog.db

    opt Respaldar sin sesión (desktop)
        U->>PR: Respaldar
        PR-->>U: aviso "Necesitas una cuenta de respaldo"
        U->>PR: Ir a Ajustes
        PR->>FE: abre Ajustes en la pestaña Respaldo
    end
    U->>FE: URL del servidor + login o registro (email + contraseña)<br/>(en web: pantalla de login)
    Note over FE: una URL editada y sin guardar<br/>se guarda antes de autenticar
    FE->>AC: comando login()
    AC->>SV: POST /v1/auth/login
    SV->>DB: lee photographer (email, pass_hash)
    DB-->>SV: hash almacenado
    Note over SV: valida credenciales con argon2id
    SV->>SV: emite JWT (access + refresh)
    SV-->>AC: devuelve access + refresh token
    Note over AC: guarda tokens en settings<br/>creator_id === photographer_id
    AC-->>FE: sesión iniciada
    Note over FE: BackupSessionProvider actualiza la sesión:<br/>Ajustes muestra email + Cerrar sesión<br/>y se reanuda la cola de respaldos pendiente
    Note over AC: interceptor: ante un 401<br/>refresca con el refresh token
    U->>FE: Ajustes → Respaldo → Cerrar sesión
    FE->>AC: comando logout()
    AC->>SV: POST /v1/auth/logout (revoca el refresh)
    Note over AC: borra la sesión local aunque el server no responda
    AC-->>FE: sin sesión → Ajustes vuelve al formulario
```

## Notas

| Punto | Detalle |
|-------|---------|
| Identidad única | Una sola entidad `photographer`. El server es dueño de `email` y `pass_hash`; el id local (`creator_id`) y el `photographer_id` del server son la MISMA identidad. |
| Hashing | `argon2id` en el server. |
| Renovación | El `AuthClient` lleva un interceptor que, ante un `401`, renueva con el refresh token de forma transparente. |
| Almacenamiento | Los tokens quedan en los *settings* locales de la app. |
| Dónde se gestiona (desktop) | URL del servidor, login, registro y logout en Ajustes → Respaldo. Pulsar Respaldar en un proyecto sin sesión muestra un aviso con "Ir a Ajustes", que abre esa pestaña. La barra lateral no muestra datos de la cuenta. |
| Cola pendiente | La cola de respaldos se reanuda la primera vez que hay sesión en la ejecución (al arrancar o al iniciar sesión); sin sesión no se intenta subir nada. |
