# Plan — Respaldo de `_exported` (backup a la nube)

> Documento vivo. Marca cada checkpoint (`- [x]`) al completarlo.
> Ámbito: `apps/server/` (nuevo) + `apps/desktop/` (integración) + `apps/web/` (fase final).

## 1. Decisiones tomadas

| Decisión | Elección | Consecuencia |
|----------|----------|--------------|
| ¿Quién sube a la nube? | **Servidor centralizado** | La credencial vive en UN lugar (el server). Desktop y web hacen `POST` del archivo; el server sube. Habilita ambas vías. |
| ¿Qué se respalda? | **`_exported` con estado por-foto** | Se usan las columnas `backup_*` de `photo`; el estado se ve en el grid. Requiere reconciliar `_exported` ↔ filas `photo`. |
| ¿Cómo hablar con los proveedores? | **rclone** | Un binario abstrae gdrive/dropbox/onedrive/mega/nextcloud con una sola config (`rclone.conf`). |
| ¿Cuántos proveedores a la vez? | **Multi-proveedor seleccionable** | El usuario conecta N proveedores (cada uno = un remote rclone); en cada backup elige uno, varios o **todos** → el server hace fan-out (`rclone copy` a cada remote). |
| Stack del servidor | **Rust + axum** | Mismo toolchain que desktop, puede reusar reglas de rutas del dominio (`ProjectFolder::path`); rclone como subproceso (`tokio::process`). |
| Reconciliación cross-device (iPad/web) | **Modelo B — catálogo en el server** | El server mantiene un catálogo de proyectos/fotos. La desktop lo publica; el iPad/web listan proyectos y suben contra un `photoId` real; la desktop sincroniza el estado al reconectar. Requiere BD propia en el server + protocolo de sync. |
| Despliegue e identidad | **SaaS con cuentas (multi-tenant)** | Un server hospedado para muchos fotógrafos. Login (email+contraseña) → JWT. Cada usuario tiene su catálogo y **sus propios** remotes de nube, aislados por `photographer_id`. La desktop se identifica con el JWT, y así el server sabe qué credenciales/remotes usar. |

---

## 2. Estado actual (terreno ya preparado)

- **Schema reservado**: `photo` ya tiene `backup_status` (`pending`/`uploaded`/`failed`), `backup_url`, `backup_date`, `backup_retries` + índice `idx_photo_backup` (`db/project_schema.sql`, `db/models.rs:59`). No hay que migrar el schema para el estado por-foto.
- **`_exported/` ya se crea** por proyecto (`commands/project.rs:97-100`).
- **Arquitectura limpia con puertos**: patrón establecido (`MetadataTool`, `ImageProcessor`, `FinderTagWriter`). El backup entra como un puerto nuevo sin tocar `domain/`.
- **Settings globales** key/value (`db/global_schema.sql:41`) → guarda URL del server + tokens de sesión (JWT).
- **`photographer` ya existe local** (`db/global_schema.sql:10`) con `creator_id` en cada proyecto → puente hacia la cuenta del server (ver §4).

### Gap a resolver
Los archivos de `_exported` **no tienen fila en `photo`**: se generan fuera de la app (edición externa). Para tener "estado por-foto" hay que **reconciliar** primero: emparejar cada `_exported/IMG_0001.jpg` con su fila `photo` por *stem* del nombre, fijar `jpg_path` y `workflow_status='edited'`.

---

## 3. Arquitectura end-to-end (Modelo B — catálogo)

```
Desktop (Tauri/Rust)               Lumik Server SaaS (axum)               Nube
─────────────────────              ────────────────────────               ──────
0. login (email+pass) ─POST──►     photographer(id,email,pass_hash)       gdrive
                       ◄─JWT──      catalog.db (Postgres), TODO por photog: dropbox
1. sync_exported (local):           project(id,photographer_id,name,date,slug)   onedrive
   escanea _exported,               photo(id,project_id,filename,hash,     mega
   empareja → photo,                backup_status,backup_url,provider)    nextcloud
   set jpg_path/edited              remote(id,photographer_id,provider,token)
2. AL ABRIR proyecto:              (todos los endpoints scoped por JWT→photographer_id)
   ensure_catalog_synced ─check─►   GET /v1/projects/{id} → 200|404
     ├─ si 404: publish    ─push─►   (crea project + photos del fotógrafo)
     └─ si 200: reconcile  ◄─pull─   (trae backup_status al project.db)
3. backup_project:                  POST /v1/backup (multipart, JWT)
   sube _exported        ─POST──►   ├─ resuelve remotes del fotógrafo
                         ◄─recibo─   ├─ actualiza catalog + rclone copy ──►
                                    └─ responde results[] por proveedor
iPad / Web dashboard:
   login → JWT             ◄─────  (mismo sistema de cuentas)
   GET /v1/projects        ◄─────  (solo los proyectos del fotógrafo)
   sube contra photoId     ─POST─► (actualiza catalog.backup_status)
```

**Fuentes de verdad (importante):**
- **Existencia y metadata** de proyecto/foto (nombre, fecha, culled, tags…) → la **desktop** (disco externo `project.db`). La desktop *publica* al catálogo.
- **Estado de respaldo** (`backup_status`/`url`/`date`) → el **server** (porque el iPad puede subir sin la desktop presente). La desktop *reconcilia* desde el catálogo al reconectar.
- **El archivo respaldado** en sí → la **nube** (rclone). El catálogo es un índice; la verdad última de "existe el archivo" se puede re-verificar contra rclone.

**Identidad de foto compartida:** el `photoId` (UUID) lo genera la desktop al importar y viaja al catálogo. El iPad solo sube contra fotos que **ya están en el catálogo** (importadas y publicadas por la desktop). Fotos que nazcan solo en el iPad quedan fuera de alcance del MVP (ver §10).

**Identidad de usuario (multi-tenant):** todo en el catálogo (`project`, `photo`, `remote`) cuelga de un `photographer_id`. La desktop se loguea una vez → guarda el JWT → cada request lo lleva → el server deriva el `photographer_id` y **filtra todo por ese tenant**. Así "sabe qué credenciales usar": resuelve los `remote` DEL usuario autenticado. El `creator_id` local se asocia a la cuenta del server en el primer login (ver §4).

**Layout en la nube** — espeja la jerarquía por fecha del disco, solo `_exported/`:

`Lumik/{año}/{mes}/{día}_{slug}/_exported/IMG_0001.jpg`

Decisiones fijadas:
- **Carpeta raíz `Lumik/`** fija en todos los remotes (misma estructura en disco y nube → fácil verificar/restaurar).
- **Se conserva `_exported/`** en la ruta remota (deja claro que son finales editados y evita colisión si algún día se respalda RAW).
- **Se reusa `domain::project::ProjectFolder::path`** para construir el path remoto en el server (axum = Rust). Para compartir la regla sin duplicar: extraer `domain/project.rs` (o solo la función de ruta) a un crate del workspace consumible por `desktop` y `server`. Referencia local: `apps/desktop/src-tauri/src/domain/project.rs:40`.

---

## 4. Identidad y cuentas (SaaS multi-tenant)

Tres identidades distintas (NO confundir):

| # | Identidad | Dónde vive | Para qué |
|---|-----------|-----------|----------|
| 1 | **Fotógrafo (local)** | `photographer` + `creator_id` en la BD local del disco | Autoría de proyectos en la app |
| 2 | **Cuenta Lumik (server)** | tabla `photographer` en `catalog.db` (misma entidad + email/pass_hash) | Login SaaS; dueña del catálogo y de los remotes |
| 3 | **Credenciales de nube** | tabla `remote` (por `photographer_id`) en el server | Subir a gdrive/dropbox/… en nombre del fotógrafo |

> **Una sola entidad `photographer`**: en local tiene nombre/metadata; en el server gana `email` + `pass_hash`. No se introduce `user` para no partir el lenguaje del dominio.

**Cómo la desktop sabe qué credenciales usar:**
1. El usuario se loguea en la app (email+contraseña) → el server valida y devuelve un **JWT** (access + refresh).
2. La desktop guarda los tokens en `settings` y adjunta el `Authorization: Bearer <access>` en cada request.
3. El server verifica el JWT → obtiene `photographer_id` → **todos** los queries del catálogo y la resolución de remotes se filtran por ese `photographer_id`.
4. Al respaldar, el server usa los `remote` que pertenecen a *ese* usuario. Nunca hay ambigüedad de "cuáles credenciales".

**Puente identidad local ↔ cuenta (trivial):** como `creator_id` **ya es** el id del fotógrafo, se hace que `creator_id` (local) === `photographer_id` (server) — la misma identidad, **sin tabla de mapeo**. En el primer registro/login se reconcilia el id (el server adopta el `photographer.id` local, o la desktop adopta el que asigne el server). Al publicar el catálogo, los proyectos ya vienen atribuidos a esa identidad.

**Tokens:** access token corto (p.ej. 15 min) + refresh token largo con rotación. La desktop refresca en silencio; si el refresh caduca, pide login de nuevo.

### Auth: mecánica
- Contraseñas con **argon2id** (nunca en claro, nunca logueadas).
- JWT firmado (secreto del server); claim mínimo `sub = photographer_id`.
- Rate limiting en `/auth/login` y `/auth/register`.
- (Post-MVP) verificación de email, reset de contraseña, login social.

## 5. Conexión de proveedores (el "login" a la nube)

Cada proveedor conectado = **un remote de rclone**. Dos tipos:

> Multi-tenant: **cada usuario conecta SUS propias cuentas de nube**. El `client_id`/`client_secret` de la app OAuth es de Lumik (uno solo, en el server); el `refresh_token` resultante es **por usuario** y se guarda en `remote(photographer_id, provider, token)`. Los remotes de rclone se nombran/aíslan por usuario (p.ej. `u{photographer_id}_gdrive`) o se generan on-the-fly desde el token guardado (env `RCLONE_CONFIG_*` / connection strings) para no compartir un `rclone.conf` global.

- **OAuth (Google Drive, Dropbox, OneDrive)** — login con navegador:
  1. Registrar *una vez* una app OAuth de Lumik en cada proveedor (Google Cloud / Dropbox / Azure). El `client_id`/`client_secret` vive **solo en el server**.
  2. Dashboard (usuario logueado): "Conectar Google Drive" → server arma URL de consentimiento (con `state` ligado al `photographer_id`) → navegador → login/aceptar → callback al server con el `code` → server lo canjea por `refresh_token` → guarda `remote(photographer_id, 'gdrive', token)`.
  3. rclone refresca los tokens solo; no se vuelve a pedir login.
- **Sin OAuth (Mega, Nextcloud)** — pegar credenciales (guardadas cifradas por usuario):
  - **Mega**: email + contraseña.
  - **Nextcloud**: URL + usuario + *app-password* (WebDAV, vendor `nextcloud`).

**MVP de pruebas**: con una cuenta de prueba, conectar un proveedor por el flujo OAuth (o sembrar un `remote` a mano en la BD) para validar el pipeline antes de pulir la UI.

## 6. Contrato de API del servidor

> **Todos** los endpoints (salvo `/auth/*` y `/healthz`) exigen `Authorization: Bearer <access-JWT>` y se filtran por el `photographer_id` del token. Un usuario nunca ve/toca recursos de otro.

**Auth / cuentas**
- `POST /v1/auth/register` → `{ email, password }` → crea `photographer` (argon2id).
- `POST /v1/auth/login` → `{ email, password }` → `{ access, refresh, photographer }`.
- `POST /v1/auth/refresh` → `{ refresh }` → `{ access, refresh }` (rotación).
- `POST /v1/auth/logout` → invalida el refresh.
- `GET  /v1/auth/me` → datos del usuario autenticado.

**Catálogo / sync (desktop → server)**
- `POST /v1/sync/projects` → upsert de proyectos `[{ id, name, session_date, slug }]` (metadata: la desktop manda). El `photographer_id` lo pone el server desde el JWT (no se confía en el cliente).
- `POST /v1/sync/projects/{projectId}/photos` → upsert del catálogo de fotos `[{ id, filename, file_hash, jpg_path }]` (candidatas de `_exported`).
- Regla de merge: **metadata** (existencia, nombre, fecha) → gana la desktop; **backup_status/url/date** → gana el server (no se pisan con el push de la desktop).

**Listado / existencia**
- `GET /v1/projects` → proyectos del catálogo (para que el iPad los liste).
- `GET /v1/projects/{projectId}` → `200` si el proyecto ya está en el catálogo, `404` si no (lo usa la desktop al abrir para decidir si publicar).
- `GET /v1/projects/{projectId}/photos?backup=pending` → fotos y su estado.

**Backup**
- `POST /v1/projects/{projectId}/photos/{photoId}/backup`
  - `multipart/form-data`: `file` + campos `filename`, `file_hash`, `targets` (`["gdrive","dropbox"]` o `"all"`; default configurable). El `project_slug`/`session_date` para el path los toma el server del catálogo; los remotes se resuelven **del usuario del JWT**.
  - **Idempotencia**: si `file_hash` ya existe en un remote, no re-sube en ese remote.
  - Efecto: sube a la nube **y** actualiza `catalog.photo.backup_*`.
  - `200 → { results: [ { provider, backup_url, remote_path, status } ], backup_date }`
- `GET /v1/projects/{projectId}/backup/status` → `[ { photoId, backup_status, results:[…] } ]` (la desktop reconcilia desde aquí al abrir el proyecto).

**Proveedores (conexión)**
- `GET  /v1/providers` → lista de remotes conectados + estado
- `POST /v1/providers/{type}/connect` → inicia OAuth (devuelve `auth_url`) o recibe credenciales (Mega/Nextcloud)
- `GET  /v1/providers/oauth/callback` → callback OAuth; crea el remote
- `DELETE /v1/providers/{name}` → desconecta (borra el remote)

`GET /healthz`

**rclone en el server**: correr `rclone rcd` (daemon de control remoto) y usar su API HTTP: `operations/copyfile` para subir, `config/create|delete|dump` para gestionar remotes. `rcd` es preferible en un server de larga vida (reúso de conexión, jobs async, progreso, gestión de config programática). El binario se empaqueta igual que `exiftool` en `binaries/`.

---

## 7. Piezas nuevas en Desktop (clean arch)

- `application/ports.rs`:
  - `BackupClient` → `upload(photo, file_path, ctx, targets) -> BackupReceipt`, `check_status(project) -> ...`
- `infrastructure/backup/http_backup.rs`: implementa `BackupClient` con `reqwest` (POST multipart al server).
- `application/use_cases/sync_exported.rs`: escaneo + emparejamiento `_exported` ↔ `photo` (fija `jpg_path`, `workflow_status`). **Con tests de fakes.**
- `application/use_cases/backup_project.rs`: lista pendientes, itera subidas, actualiza `PhotoRepository`, reporta vía `ProgressReporter`. **Con tests de fakes.**
- `PhotoRepository` (puerto): añadir `update_backup(id, status, url, date, retries)` y `list_pending_backup(project_id)`.
- `CatalogClient` (puerto nuevo) + impl HTTP: `project_exists()`, `publish_projects()`, `publish_photos()`, `pull_backup_status()`.
- `AuthClient` (puerto nuevo) + impl HTTP: `login()`, `refresh()`, `me()`, `logout()`. Un middleware/interceptor en el cliente HTTP adjunta el access token y refresca de forma transparente en `401`.
- Almacenamiento de tokens: `access`/`refresh` en `settings` (o keychain del OS más adelante) + mapeo `creator_id ↔ photographer_id`.
- `application/use_cases/ensure_catalog_synced.rs` (**disparado al abrir el proyecto**): comprueba `project_exists`; si falta, publica proyecto + fotos; si existe, reconcilia `backup_status`. Idempotente y silencioso si no hay red o no hay sesión (degrada a solo-local).
- `application/use_cases/publish_catalog.rs` + `reconcile_backup_status.rs` (piezas que orquesta `ensure_catalog_synced`; también invocables a mano).
- `interface/commands/backup.rs`: `login/logout/current_photographer`, `start_backup(project_id, targets)`, `get_backup_status(project_id)`, `sync_exported(project_id)`, `publish_catalog(project_id)`, `list_providers()`.
- Frontend: pantalla de **login/registro** + estado de sesión; botón "Respaldar" + selector de proveedores destino (uno/varios/todos) + badge de estado por foto en el grid + settings (URL server, cerrar sesión). Claves i18n nuevas.

> Nota estado por-foto con multi-proveedor: `backup_status` a nivel foto = `uploaded` cuando **todos** los targets seleccionados subieron; `failed` si alguno falló. El detalle por proveedor (`results[]`) lo devuelve el server; si más adelante se quiere persistir por-proveedor, se añade una tabla `photo_backup(photo_id, provider, url, status)` (no en MVP).

---

## 8. Plan por fases

> **Orden:** de la cuenta hacia afuera (cuenta → web → proveedores → desktop → catálogo → backup).
> Prioriza tener un login web usable temprano y deja la subida real para el final, **pero de-riesga rclone en la Fase 0**.

> ⚠️ **Prerrequisito externo (arrancar YA, en paralelo con la Fase 0):** registrar las apps OAuth de Lumik en Google Cloud, Dropbox y Azure. La **verificación de Google (scopes de Drive) puede tardar semanas**; si se deja para la Fase 3, bloquea. No es código, pero tiene lead time.

### Fase 0 — Infra del server + rclone (de-risk) ✅
- [x] Scaffold `apps/server` (axum) con `/healthz` (verifica la BD).
- [x] `catalog.db` (**Postgres 17** vía docker-compose): tablas `photographer` (con email/pass_hash), `project(photographer_id)`, `photo`, `remote(photographer_id)` + campos `backup_*`. Migraciones con **sqlx** (`migrations/0001_init.sql`, aplicadas al arranque).
- [x] Instalar `rclone` (binario local, `scripts/install_rclone.sh`); **smoke test** vía `rclone rcd` → `operations/copyfile` (`scripts/rclone_smoke.sh`).

**Gate 0:** ✅ **alcanzado** — el server arranca, aplica migraciones (4 tablas), `/healthz` → `{"status":"ok","db":"up"}`, y el smoke test copia un archivo vía `rclone rcd`. Nube real (OAuth) → Fase 3.

### Fase 1 — Cuentas / Auth (backend) ✅
- [x] `POST /v1/auth/register|login|refresh|logout`, `GET /v1/auth/me`. **argon2id** + **JWT HS256** firmado; refresh tokens opacos (sha256) con **rotación + revocación** en BD (`migrations/0002_refresh_token.sql`).
- [x] Extractor `AuthPhotographer` (`FromRequestParts`): saca `photographer_id` del JWT y rechaza sin token/expirado.
- [x] Rate limiting por IP en `/auth/login|register` (fixed window). Tests: **11 verdes** (5 unitarios argon2/jwt incl. expiración + 6 integración con `#[sqlx::test]`: register/me, 401 sin token, email duplicado 409, login ok/mal, rotación de refresh, **aislamiento entre tenants**).

**Gate 1:** ✅ **alcanzado** — verificado en tests y en vivo: register/login → JWT; `/me` sin token → `401`, con token → `200` scoped al fotógrafo; login incorrecto → `401`.

### Fase 2 — Login en web ✅
- [x] `apps/web` (React 19 + Vite 6 + `@lumik/ui`): pantallas **registro/login/logout** que consumen `/v1/auth/*`; tokens en `localStorage` con **refresh transparente en 401** (`src/api/client.ts`).
- [x] `AuthProvider` (sesión) + `ProtectedRoute`; ruta protegida `/` (Dashboard) que hidrata con `GET /v1/auth/me`. i18n es/en reusando la instancia de `@lumik/ui`. Proxy dev `/v1 → :8787`.

**Gate 2:** ✅ **alcanzado** — build (tsc + vite) verde; e2e vía el proxy del navegador (`:5173`): register `200`, login `200`, `/me` con token → identidad correcta, sin token → `401`.

### Fase 3 — Configuración de servicios de backup (proveedores) ✅ (credenciales) · ⏳ (OAuth pendiente de apps)
- [x] Server: `GET /v1/providers`, `POST /v1/providers/{type}/connect`, `DELETE /v1/providers/{id}` — remote por `photographer_id`, **credenciales cifradas en reposo (AES-256-GCM)**, todo scoped por tenant.
- [x] **Credenciales (Nextcloud/WebDAV, Mega)**: conexión completa y probada (6 tests: connect/list/delete, campos requeridos, auth, **aislamiento entre tenants**).
- [~] **OAuth (gdrive/dropbox/onedrive)**: `connect`/`oauth/callback` scaffolded pero **gated** → `501` con mensaje claro hasta registrar las apps y definir `client_id/secret`. *(Prerequisito externo pendiente.)*
- [x] Web: panel "Conectar proveedor" (Nextcloud/Mega) + lista + desconectar.

**Gate 3:** ✅ **alcanzado** (credenciales) — verificado en vivo: conectar Nextcloud → guardado en `remote(photographer_id,…)` con el token **cifrado** (`plano=false` en BD); OAuth responde `501` gated. Cierre OAuth cuando lleguen las apps.

### Fase 4 — Login en app desktop ✅
- [x] Puerto `BackupAuthClient` + impl HTTP (`reqwest::blocking`, rustls) en `infrastructure/backup_auth.rs`; comandos `backup_login/register/logout/get_backup_session` + `get/set_backup_server_url`.
- [x] Sesión persistente en la tabla `settings` (BD global): access/refresh/photographer_id/email/name/server_url. `get_backup_session` valida con `/me`, refresca en 401, y **no desloguea ante error de red** (offline → mantiene sesión).
- [x] UI: pestaña "Respaldo" en `SettingsPage` (`BackupAccountSection`) con login/registro/logout + URL del server + i18n es/en.
- [~] Reconciliación de identidad: se guarda el `photographer_id` del server como puente (el `===` con `creator_id` se realiza en Fase 5 al publicar el catálogo).

**Gate 4:** ✅ **alcanzado** — verificado con test del cliente HTTP real contra el server (`register → me → login → refresh → login incorrecto`); 53 tests del desktop verdes. La sesión persiste en `settings` entre reinicios (restauración vía `get_backup_session`).

### Fase 5 — Reconciliación de proyectos (catálogo) ✅
- [x] Server: `POST /v1/sync/projects` y `.../projects/{id}/photos` (upsert; `photographer_id` del JWT; **backup_* NO se pisa** al re-sincronizar); `GET /v1/projects`, `GET /v1/projects/{id}` (200/404), `GET /v1/projects/{id}/photos[?backup=]`. 4 tests (incl. preservación de backup_status y aislamiento entre tenants).
- [x] Desktop: puerto `CatalogClient` + impl HTTP (`infrastructure/catalog.rs`) + use case `ensure_catalog_synced` con 4 tests de fakes (no-session/offline/exists/missing).
- [x] **`ensure_project_synced` enganchado a abrir proyecto** (efecto en `ProjectDetail`): resuelve sesión (refresca en 401), si `404` publica proyecto + fotos; degrada a solo-local sin red/sesión.

**Gate 5:** ✅ **alcanzado** — verificado con test del `HttpCatalog` real contra el server (exists=false → publish → exists=true) + tests del server. Al abrir un proyecto se crea en el catálogo si falta y aparece en `GET /v1/projects`.

**Identidad (cierre de la nota de Fase 4):** NO se fuerza `creator_id === photographer_id` (sería destructivo sobre BDs de disco). En su lugar, el catálogo **atribuye los proyectos al fotógrafo del JWT**; el `creator_id` local no se envía. La propiedad multi-tenant se deriva del token, no de igualar ids.

### Fase 6 — `sync_exported` local (_exported ↔ photo) ✅
- [x] `use_cases/sync_exported.rs`: matching por *stem* (case-insensitive) → fija `jpg_path` + `workflow_status='edited'` vía `PhotoRepository::set_exported` (puerto + impl + query nuevos); reporta no-emparejados. 3 tests de fakes.
- [x] Comando `sync_exported`: escanea `_exported/` (extensiones de imagen), construye rutas `jpg_path` relativas al mount (`path_to_slash`), corre el use case y **publica al catálogo** (`publish_catalog_upsert`, best-effort, refresca en 401). Registrado en `invoke_handler`.
- [x] Frontend: `syncExported` disparado al abrir el proyecto tras `ensureProjectSynced` (fire-and-forget).

**Gate 6:** ✅ **lógica verificada** — matching cubierto por tests; command compila y publica reusando el path del catálogo. *Pendiente verificación en la app con `_exported` real en disco (requiere la GUI).* 61 tests desktop verdes.

### Fase 7 — Funcionalidad de backup (3 piezas) ✅
- [x] **Server:** `POST /v1/projects/{id}/photos/{id}/backup` (multipart; verifica dueño; ruta en la nube desde el catálogo; resuelve remotes del fotógrafo con `targets` fan-out; sube con **rclone real** `copyto` vía config on-the-fly con `pass` ofuscado; actualiza catálogo `backup_*`; recibo `results[]`). Test e2e con remote `alias` local.
- [x] **rclone real:** `copy_to` + `obscure` + `config_for` (materializa credenciales cifradas). Test de `copy_to`.
- [x] **Desktop:** puerto `BackupClient` + impl HTTP multipart; `PhotoRepository.set_backup_uploaded`; comando `backup_project` (lista pendientes con `jpg_path` y no `uploaded`, sube c/u, refresh-on-401, actualiza local). Asegura catálogo antes de subir.
- [x] **UI desktop:** botón "Respaldar" en el header del proyecto + estado (subidas/total) + i18n; refetch de fotos al terminar.

**Gate 7:** ✅ **verificado en server** (e2e con alias local: multipart → rclone → archivo aterriza en la ruta espejo → catálogo `uploaded`). *Subida real a Mega/Nextcloud pendiente de probar en la app (requiere GUI + proveedor real).* Server 26 tests · desktop 61 tests.

Notas:
- **La unidad de respaldo es el archivo de `_exported/`, no la fila `photo`.** El
  emparejamiento por *stem* con el RAW es frágil (los editores renombran al exportar:
  `..._Clorotipias__.cr2` → `..._Clorotipias_001.jpg`), así que el backup ya **no**
  depende de él. Cada archivo de `_exported/` es la unidad, con **id determinista
  `uuid5(project_id + filename)`**. El server **upsertea** la fila `photo` al recibir
  el archivo (no exige que exista antes).
- **UI = modal**: el botón "Respaldar" abre un modal que lista los archivos de
  `_exported/` marcando cuáles ya están en el remote (vía catálogo), con botón
  "Subir N" y estado por-archivo (spinner → ✓/✗). Comandos `list_exported_backup`
  + `backup_exported_file` (uno por archivo, para feedback granular).
- **Límite de body**: `DefaultBodyLimit::max(512MB)` en la ruta de backup (el
  default de axum, 2MB, cortaba las fotos).
- **Idempotencia:** el modal solo sube los que el catálogo marca no-`uploaded`.

### Fase 8 — Subida desde iPad/web + reconciliación cross-device
- [ ] Web: dashboard que lista proyectos/fotos del catálogo y sube contra `photoId` real (drag & drop / POST).
- [ ] Verificar el ciclo completo: iPad sube → catálogo `uploaded` → desktop reconcilia al abrir.

**Gate 8:** desde el iPad se elige un proyecto del catálogo, se sube una foto, y al reconectar la desktop la marca `uploaded`.

### Fase 9 — Robustez
- [ ] Reintentos con backoff (`backup_retries`); cola offline-first en desktop (encolar mientras no hay red).
- [ ] Verificación de integridad (comparar hash tras subida) y re-check contra rclone (`lsf`).
- [ ] Resolución de conflictos de sync (proyecto renombrado/borrado en ambos lados).
- [ ] Refuerzo de cuentas: verificación de email, reset de contraseña, revocación de sesiones.

---

## 9. Seguridad
- **Decisión — tokens en cliente: Bearer + localStorage (web) / Bearer + keychain (desktop).**
  Se evaluó cookie `httpOnly` para el web (más resistente a XSS) y se **optó por
  mantener Bearer/localStorage** por simplicidad y por servir igual a los dos
  clientes. Riesgo aceptado: **una XSS puede robar el refresh token**. Controles
  compensatorios obligatorios: **CSP estricta** en el web, dependencias auditadas,
  **access token corto** (15 min) + **rotación de refresh** (ya implementada),
  y **TLS** en todo. Reconsiderar cookie `httpOnly` si aparece contenido de
  terceros / user-generated en el portal.
- **TLS obligatorio** entre clientes y server (es SaaS público).
- Contraseñas con **argon2id**; JWT firmado, access corto + refresh con rotación; rate limiting en `/auth/*`.
- **Aislamiento por tenant**: todo query filtra por `photographer_id` del JWT; test explícito de que A no accede a recursos de B.
- Tokens de nube (`remote.token`) **cifrados en reposo** en la BD; nunca se exponen al cliente ni se loguean.
- Tokens de sesión en el cliente en `settings` (evaluar keychain del OS más adelante); nunca loguear tokens.
- Proteger el host del server (ahí viven la BD con tokens de nube + los `client_secret` de las apps OAuth de Lumik).

## 10. Qué NO sobre-ingenierizar (MVP)
- Sin cola distribuida: reintentos simples + estado en la BD del proyecto.
- Reconciliación por nombre de archivo (stem); sin hashing perceptual ni matching difuso.
- Estado por-foto agregado (no por-proveedor) hasta que se necesite.
- **Catálogo = push desde la desktop** (no un sync bidireccional completo): el iPad solo sube contra fotos **ya publicadas** por la desktop. Fotos que nazcan solo en el iPad (sin `photoId` de desktop, sin fila en `project.db`) quedan **fuera de alcance** del MVP; entran cuando se defina cómo "adoptarlas" al reconectar el disco.
- Auth mínima viable: email+contraseña + JWT. Verificación de email, reset de contraseña, login social y revocación fina → Fase 9.
- Plan de cuotas/billing por usuario: fuera de alcance inicial (aunque el modelo multi-tenant lo habilita después).

## 11. Bitácora
- **2026-09-15 · Fase 7 · fix freeze de UI + compresión más rápida** — Los comandos de
  respaldo eran síncronos → corrían en el hilo principal de Tauri y **congelaban la UI**.
  Se hicieron `async` y su cuerpo se envuelve en `async_runtime::spawn_blocking` (con un
  clon de `AppState`, ahora `Clone`), porque `reqwest::blocking` dentro de un contexto
  async paniquea ("runtime within runtime"); spawn_blocking corre en hilo dedicado y no
  bloquea la UI. Compresión: el filtro de redimensionado pasó de **Lanczos3** (lento) a
  **Triangle** (bilineal), ~5–10× más rápido con buena calidad. Tradeoff inherente: la
  conversión es server-side (por decisión), así que el archivo **original** sube completo
  al server antes de comprimir; si se quisiera minimizar, habría que convertir en el
  desktop antes de subir o usar decode con escala DCT (mozjpeg). — (1) se saltan archivos ocultos
  al escanear `_exported/`; (2) el modal solo lista los **no** subidos; (3) toggles de
  **proveedores** conectados (`list_backup_providers` → `targets`); (4) **select de
  calidad** con recompresión **en el server** (`image` crate: original/high/medium/low
  → redimensiona + JPEG); (5) subida **paralela** (pool de concurrencia 3 en el
  frontend); (6) **guard de subidas en curso** en `AppState` (`uploading: HashSet`) +
  Drop guard, para no re-lanzar al reabrir el modal (evita duplicados); (7) **spinner**
  CSS. El server upsertea la foto y el path remoto es determinista, así que re-subir
  sobreescribe (no duplica) — el guard es defensa extra.
- **2026-09-15 · Fase 7 · rediseño a "backup por archivo de _exported"** — Con datos
  reales (proyecto Clorotipias) se vio que el editor renombra los exports, así que el
  emparejamiento por stem fallaba y solo subía 1 de 4. Se pivotó: la unidad de
  respaldo es el **archivo de `_exported/`** con id determinista `uuid5(project_id+
  filename)`; el endpoint de backup **upsertea** la fila `photo` (verifica dueño del
  proyecto, ya no exige photo previa). Comandos nuevos `list_exported_backup` +
  `backup_exported_file`; se retiró `backup_project` (y `set_backup_uploaded`, sin uso).
  UI: **modal** con lista + "Subir N" + estado por-archivo. Fix aparte:
  `DefaultBodyLimit::max(512MB)` en la ruta de backup (el default 2MB cortaba las fotos).
- **2026-09-15 · Fase 7 · completada (verificada en server)** — Subida real. Server:
  `rclone::{obscure, config_for, copy_to}` materializa cada remote en un config
  temporal (pass ofuscado) y sube con `copyto`; endpoint multipart `.../backup` con
  fan-out a los remotes del fotógrafo, `spawn_blocking` para no bloquear el runtime,
  y actualización de `backup_*` en el catálogo. Test e2e con remote `alias` local
  (sin credenciales de nube). Desktop: `BackupClient` (reqwest multipart, feature
  añadida), comando `backup_project` (refresh-on-401, actualiza local vía
  `set_backup_uploaded`), botón "Respaldar" en el header. **Pendiente:** subida real a
  Mega/Nextcloud (requiere GUI + cuenta) y badge por-foto en el grid. Server 26 tests,
  desktop 61.
- **2026-09-15 · Fase 6 · completada (lógica)** — `sync_exported`: use case de
  matching por stem (case-insensitive) + `PhotoRepository::set_exported` (puerto/impl/
  query). Comando escanea `_exported/`, arma `jpg_path` relativo al mount y publica al
  catálogo (`publish_catalog_upsert`, refactor que extrae `build_catalog_data` — ahora
  compartido con `ensure_project_synced`). Se añadió `set_exported` no-op a los 4 fakes
  de otros use cases (el trait no tiene default, para forzar impl en producción).
  Disparo en el `useEffect` de `ProjectDetail` tras `ensure`. Falta la verificación en
  la app con `_exported` real (GUI). 61 tests desktop verdes.
- **2026-09-15 · Fase 5 · completada** — Catálogo end-to-end. Server: módulo `catalog`
  con sync/list/exists scoped por tenant; **el upsert de fotos preserva `backup_*`**
  (solo actualiza metadata) — probado. Desktop: `CatalogClient` + `HttpCatalog` +
  use case `EnsureCatalogSynced` (tests de fakes) + comando `ensure_project_synced`
  disparado en el `useEffect` de `ProjectDetail`. Se refactorizó `active_session`
  (resolución/refresh de sesión reutilizable) fuera de `get_backup_session`.
  **Decisión de identidad:** se descarta forzar `creator_id === photographer_id`
  (destructivo); el catálogo deriva la propiedad del JWT y no envía `creator_id`.
  **Nota:** el fake del use case usa `AtomicUsize` (no `RefCell`) porque el trait
  `CatalogClient` exige `Send + Sync`. 57 tests desktop + 24 server, todos verdes.
- **2026-09-15 · Fase 4 · completada** — Login del desktop contra el server. Puerto
  `BackupAuthClient` (application/ports) + impl `HttpBackupAuth` (reqwest **blocking**,
  rustls — seguro en comandos Tauri síncronos que corren en thread pool, no en el
  runtime async). Sesión persistida en la tabla `settings` de la BD global (se hicieron
  públicos `get/set_setting` + `delete_setting`). `get_backup_session` distingue 401
  (refresca / desloguea) de offline (mantiene sesión) por el mensaje de la impl. UI:
  pestaña "Respaldo" en Settings. Verificación runtime headless: test de `HttpBackupAuth`
  real contra el server (gated por `LUMIK_TEST_SERVER`). **Nota tooling:** `pkill -f
  lumik-server` dentro del comando que lo arranca mata su propio shell (su línea contiene
  el patrón) → no usar pkill en el mismo comando de arranque. **Identidad:** por ahora se
  guarda `photographer_id` del server como puente; el `creator_id === photographer_id` se
  concreta en Fase 5.
- **2026-09-15 · Fase 3 · parcial (credenciales completas, OAuth gated)** — Conexión
  de proveedores con cifrado en reposo (AES-256-GCM, `crypto.rs`; clave por
  `REMOTE_ENC_KEY`). La config rclone de cada remote se guarda como JSON cifrado en
  `remote.token`. Endpoints scoped por `photographer_id`. **Nextcloud/WebDAV y Mega**
  (credenciales) end-to-end + 6 tests (incl. aislamiento de tenants). **OAuth**
  (gdrive/dropbox/onedrive) queda **gated con 501** hasta registrar las apps
  (prerequisito externo con lead time). UI web con panel de conexión. 20 tests
  verdes en total. Verificado en vivo que el token queda cifrado en BD.
  **Decisión:** la materialización del remote en rclone (crear el remote real vía
  `rcd config/create` o generarlo on-the-fly desde el JSON cifrado) se hace en Fase 7
  (backup), donde de verdad se sube; en Fase 3 el catálogo de `remote` es la fuente.
- **2026-09-15 · Fase 2 · completada** — `apps/web` scaffoldeado con el mismo stack
  que la desktop (React 19 + Vite 6 + `@lumik/ui` + i18next), añadido al workspace
  pnpm. Se reutilizan `Button`/`Input`/`Logo`/`Loader` de `@lumik/ui` (estilos por
  tokens) y la misma instancia i18n (`import '@lumik/ui/i18n'` + `addResourceBundle`).
  Cliente HTTP propio (`fetch`) con store de tokens en `localStorage` y refresh
  transparente en 401; login/register usan POST público (sin adjuntar token ni
  reintentar). Routing con react-router-dom v7. Verificación e2e a través del proxy
  `/v1` de vite (:5173 → :8787). **Nota de tooling:** arrancar/matar servidores de
  larga vida desde el Bash tool devuelve exit 143/144 (señal) — cosmético; los
  servicios se validaron con tareas en background + curl en una llamada aparte.
- **2026-09-15 · Fase 1 · completada** — Auth con argon2id + JWT HS256 (access) +
  refresh tokens opacos con rotación/revocación en BD. Se convirtió el crate a
  **lib + bin** (`src/lib.rs` expone `AppState` + `build_router`) para poder testear
  con `#[sqlx::test]` (BD efímera por test). Extractor `AuthPhotographer` vía
  `FromRequestParts` (requiere `#[axum::async_trait]` en axum 0.7). Rate limiter
  propio en memoria (fixed window por IP; toma la IP de `ConnectInfo`, con fallback
  "global" en tests). **Nota:** el aislamiento entre tenants en Fase 1 se prueba vía
  `/me` (cada token resuelve solo a su fotógrafo); el test de aislamiento de *datos*
  llega con los endpoints de catálogo (Fase 5). 11 tests verdes.
- **2026-09-15 · Fase 0 · completada** — `apps/server` creado como crate Rust
  independiente (no hay workspace Cargo raíz; la desktop tiene el suyo en
  `src-tauri`). Stack: axum 0.7 + sqlx 0.8 (Postgres, `runtime-tokio-rustls`) +
  tracing. Postgres 17 vía `docker-compose.yml`; migraciones sqlx aplicadas al
  arranque (idempotentes vía `_sqlx_migrations`). rclone v1.75.1 instalado como
  binario local en `bin/` (gitignored), análogo a exiftool. **Desvío intencional:**
  el smoke test de Gate 0 usa un remote `alias` local en vez de nube real —
  valida el plumbing (subproceso + `rclone rcd` + `operations/copyfile`), que es
  el riesgo técnico; la nube con OAuth se prueba en Fase 3 (sin credenciales aún).
  Health check verificado (`{"status":"ok","db":"up"}`), 4 tablas creadas.
- (registrar decisiones/desvíos al ejecutar cada fase)
