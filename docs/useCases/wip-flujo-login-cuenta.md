# WIP · Flujo: Login / Cuenta de fotógrafo (SaaS)

> **Estado: WIP.** Diseño propuesto, aún no implementado.

Autenticación del fotógrafo contra el server SaaS. La clave del diseño es que **hay una sola
entidad `photographer`**: el `creator_id` local y el `photographer_id` del server son la misma
identidad.

```mermaid
sequenceDiagram
    autonumber
    actor U as Fotógrafo
    participant FE as Desktop / Web (frontend)
    participant AC as Cliente HTTP (AuthClient)
    participant SV as Server SaaS (axum)
    participant DB as catalog.db

    U->>FE: Pantalla login/registro (email + contraseña)
    FE->>AC: comando login()
    AC->>SV: POST /v1/auth/login
    SV->>DB: lee photographer (email, pass_hash)
    DB-->>SV: hash almacenado
    Note over SV: valida credenciales con argon2id
    SV->>SV: emite JWT (access + refresh)
    SV-->>AC: devuelve access + refresh token
    Note over AC: guarda tokens en settings<br/>creator_id === photographer_id
    AC-->>FE: sesión iniciada
    Note over AC: interceptor: ante un 401<br/>refresca con el refresh token
```

## Notas

| Punto | Detalle |
|-------|---------|
| Identidad única | Una sola entidad `photographer`. El server es dueño de `email` y `pass_hash`; el id local (`creator_id`) y el `photographer_id` del server son la MISMA identidad. |
| Hashing | `argon2id` en el server. |
| Renovación | El `AuthClient` lleva un interceptor que, ante un `401`, renueva con el refresh token de forma transparente. |
| Almacenamiento | Los tokens quedan en los *settings* locales de la app. |
