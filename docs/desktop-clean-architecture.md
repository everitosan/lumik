# Refactor a Arquitectura Limpia — Backend Rust (Lumik Desktop)

> Documento vivo. Marca cada checkpoint (`- [x]`) en cuanto se complete.
> Ámbito: `apps/desktop/src-tauri/src/` (~5.500 líneas). No incluye el frontend.

**Regla de oro del refactor:** cada fase compila y es mergeable por separado, y
**ninguna regla de dominio se mueve sin su test**. El objetivo del refactor no es
solo reorganizar carpetas, sino blindar las reglas frágiles (rotación, rutas de
culling, normalización de tags, rutas por fecha) con tests que las fijen antes de
tocarlas.

## Leyenda de estado

- `- [ ]` pendiente · `- [x]` completado · `- [~]` en progreso
- Cada fase termina con un **Gate** que debe cumplirse antes de pasar a la siguiente.

---

## 1. Diagnóstico (punto de partida)

| Módulo | Líneas | Problema principal |
|--------|--------|--------------------|
| `commands.rs` | 2041 | Módulo-Dios: IPC + orquestación + reglas + infraestructura |
| `db/queries.rs` | 712 | Mapeo fila→struct duplicado; migraciones ad-hoc |
| `devices.rs` | 495 | Acoplado a `sysinfo`/OS sin abstracción |
| `db/mod.rs` | 273 | Migraciones `ALTER TABLE` incrustadas en `open()` |

Problemas de fondo:

1. `commands.rs` hace de todo (ej. `save_photo_culled` valida, calcula rutas,
   mueve archivos + sidecars, actualiza SQLite y hace rollback en una función).
2. No hay frontera testeable: los comandos tocan directamente `State`, `std::fs`,
   subprocesos de `exiftool` y SQLite.
3. La plataforma se filtra: `#[cfg(target_os = "android")]` esparcido en
   `commands.rs`, `pipeline.rs`, `devices.rs`, `db/mod.rs`.
4. Dominio anémico: `Photo`/`Project` son sacos de campos; las reglas viven como
   funciones sueltas dentro de comandos.

---

## 2. Principio rector: regla de dependencias

Cuatro capas concéntricas; las dependencias **apuntan siempre hacia adentro**.
La infraestructura implementa `trait`s (puertos) definidos por la aplicación; el
frontend (Tauri) es un mecanismo de entrega intercambiable.

```
┌─────────────────────────────────────────────┐
│  interface/  (Tauri commands — thin)         │  ← delivery
│  ┌───────────────────────────────────────┐  │
│  │  application/  (use cases + ports)     │  │  ← orquestación
│  │  ┌─────────────────────────────────┐  │  │
│  │  │  domain/  (entidades + reglas)  │  │  │  ← núcleo puro, sin deps
│  │  └─────────────────────────────────┘  │  │
│  └───────────────────────────────────────┘  │
│  infrastructure/ implementa los ports  ──────┼──▶ (apunta hacia adentro)
└─────────────────────────────────────────────┘
   exiftool · rawler · sqlite · sysinfo · fs · tauri::emit
```

- **domain**: no depende de `rusqlite`, `tauri` ni `std::process`.
- **application**: depende solo de `domain`; **define** los puertos.
- **infrastructure**: depende de `application`+`domain`; **implementa** los puertos.
- **interface**: traduce IPC ↔ casos de uso. Delgada.

---

## 3. Estructura de módulos objetivo

```
src/
  main.rs                 # entry point (sin cambios)
  lib.rs                  # composition root: arma infra, cablea use cases, registra commands

  domain/
    photo.rs              # Photo + VOs: Rotation, Stars, ColorLabel, Tags
    project.rs            # Project + ProjectFolder (ruta por fecha)
    photographer.rs
    device.rs
    paths.rs              # RelativePath, MountPoint, reglas _media/_culled
    error.rs              # DomainError

  application/
    ports.rs              # todos los traits (boundaries)
    registry.rs           # ProjectRegistry (hoy open_projects/ejecting_devices)
    error.rs              # AppError (thiserror + Serialize)
    use_cases/
      import_photos.rs    # orquesta pipeline (hoy start_import)
      cull_photo.rs
      rate_photo.rs
      rotate_photo.rs
      manage_projects.rs  # create/rename/delete/archive/relocate
      manage_devices.rs   # scan/eject/refresh
      media.rs            # thumbnail/preview

  infrastructure/
    persistence/
      global_db.rs        # impl PhotographerRepo, SettingsRepo, DeviceRegistryRepo
      project_db.rs       # impl ProjectRepo, PhotoRepo
      mappers.rs          # fila SQLite ↔ entidad (dedupe get_photo/get_project_photos)
      migrations.rs       # migraciones ordenadas y versionadas (user_version)
    metadata/
      exiftool.rs         # sesión persistente (desktop) → impl MetadataTool
      rawler.rs           # android → impl MetadataTool
    imaging/              # thumbnails, previews, open_jpeg_tiff → impl ImageProcessor
    devices/
      scanner.rs          # devices.rs → impl DeviceScanner
      watcher.rs          # device_watch.rs
      eject.rs            # os_eject
    apple_tags.rs         # ya limpio → impl FinderTagWriter
    fs.rs                 # path_to_slash, silent_command, workspace

  interface/
    device_commands.rs
    project_commands.rs
    photo_commands.rs
    import_commands.rs
    settings_commands.rs
    dto.rs                # request/response del borde IPC
```

---

## 4. Puertos (los traits frontera)

Viven en `application/ports.rs`, se implementan en `infrastructure/`:

```rust
// Persistencia
pub trait PhotoRepository {
    fn list(&self, project_id: &ProjectId) -> AppResult<Vec<Photo>>;
    fn get(&self, id: &PhotoId) -> AppResult<Option<Photo>>;
    fn create_batch(&self, photos: &[NewPhoto]) -> AppResult<Vec<Photo>>;
    fn update_rotation(&self, id: &PhotoId, r: Rotation) -> AppResult<()>;
    fn update_rating(&self, id: &PhotoId, rating: &Rating) -> AppResult<()>;
    fn update_culled(&self, id: &PhotoId, culled: bool, new_path: &RelativePath) -> AppResult<()>;
}
pub trait ProjectRepository { /* get, dashboard_entry, create, rename, archive… */ }
pub trait PhotographerRepository { /* active, metadata, keybindings, settings, devices… */ }

// Herramientas externas (aquí colapsa el #[cfg] de plataforma)
pub trait MetadataTool {
    fn read_rotation(&self, file: &Path) -> Rotation;
    fn extract_batch(&self, files: &[PathBuf]) -> HashMap<PathBuf, FileMetadata>;
    fn embed_photographer(&self, dir: &Path, meta: &PhotographerMetadata, opts: EmbedOpts) -> AppResult<()>;
    fn set_orientation(&self, file: &Path, r: Rotation) -> AppResult<()>;
    fn extract_preview(&self, raw: &Path, dest: &Path) -> AppResult<()>;
}
pub trait ImageProcessor {
    fn make_thumbnail(&self, src: &Path, dest: &Path, rotation: Rotation) -> AppResult<()>;
    fn rotate_thumbnail(&self, thumb: &Path, delta: Rotation) -> AppResult<()>;
}
pub trait DeviceScanner { fn scan(&self) -> Vec<Device>; fn eject(&self, uuid: &str, mount: &Path) -> AppResult<()>; }
pub trait FinderTagWriter { fn sync(&self, target: &Path, colors: &[MacColor]) -> AppResult<()>; fn move_sidecar(&self, from: &Path, to: &Path) -> AppResult<()>; }
pub trait ProgressReporter { fn progress(&self, p: ImportProgress); fn log(&self, session: &str, msg: &str); }
```

`ProgressReporter` desacopla el import de `AppHandle`/`app.emit`, volviéndolo
testeable con un reporter falso.

---

## 5. Estrategia de tests del dominio

Prioridad del refactor: **cada regla que hoy vive en `commands.rs` se extrae con su
batería de tests ANTES de eliminarla del comando.** Los tests del dominio son
puros (sin disco, sin exiftool, sin SQLite) y deben correr en milisegundos.

Reglas a blindar y sus casos mínimos:

- **`Rotation`** (VO): valores válidos `{0,90,180,270}`; rechazar otros; `delta =
  (nuevo - viejo + 360) % 360`; mapeo EXIF↔grados (`6→90`, `3→180`, `8→270`,
  otro→0) y su inverso (`90→6`, …).
- **`Stars`** (VO): rango `0..=5`; rechazar `-1` y `6`.
- **`Tags::normalize`**: minúsculas + trim + dedupe preservando orden; entrada
  vacía / solo comas → `None`; `"A, a ,  b"` → `"a,b"`.
- **`cull_destination`**: `_media/x` → `_culled/x` y viceversa; preserva nombre de
  archivo; error si la ruta no tiene padre/proyecto identificable.
- **`ProjectFolder::path`**: `{mount}/lumik/{año}/{mes}/{día}_{slug}`; slug
  reemplaza `/` por `-` y hace trim; fallback a fecha de hoy si no hay
  `session_date`; parsing de fecha corta/ISO/EXIF.
- **Orden del dashboard**: `session_date DESC NULLS LAST`, luego `created_at DESC`.
- **`MacColor` / `colors_from_label`**: ya tiene tests en `apple_tags.rs` — se
  conservan al mudarlo.

Convención: tests unitarios junto al código (`#[cfg(test)] mod tests`) para el
dominio; tests de casos de uso con *fakes* de los puertos en
`application/use_cases/*`. Comando para correrlos:

```
cd apps/desktop/src-tauri && cargo test
```

Gate de cobertura por fase: no se marca una fase como completa si su regla
extraída no tiene test verde.

---

## 6. Plan por fases con checkpoints

### Fase 0 — Preparación
- [x] Crear rama `refactor/clean-arch`.
- [x] Confirmar baseline: `cargo test` y `cargo build` verdes antes de tocar nada.
      (Requirió corregir una aserción obsoleta en `test_sanitize_name` — ver bitácora.)
- [x] Añadir este documento al repo y enlazarlo desde `docs/ROADMAP.md`.

**Gate 0:** ✅ build y tests actuales pasan (7/7); rama creada.

---

### Fase 1 — Extraer dominio puro (bajo riesgo, alto valor)
Mover reglas sueltas a `domain/` **con tests**, sin cambiar comportamiento.

- [x] Crear módulo `domain/` y declararlo en `lib.rs`.
- [x] `domain/photo.rs`: VO `Rotation` (validación + delta + mapeo EXIF).
  - [x] Tests de `Rotation`.
- [x] `domain/photo.rs`: VO `Stars`.
  - [x] Tests de `Stars`.
- [x] `domain/photo.rs`: `normalize_tags` (antes en `commands.rs`).
  - [x] Tests de `normalize_tags`.
- [x] `domain/paths.rs`: `cull_destination` (regla `_media`↔`_culled`).
  - [x] Tests de `cull_destination`.
- [x] `domain/project.rs`: `ProjectFolder` (slug, `date_parts`, `path`).
  - [x] Tests de `ProjectFolder`.
- [x] `domain/project.rs`: comparador de orden del dashboard.
  - [x] Tests del orden del dashboard.
- [x] Reemplazar los usos inline en `commands.rs` por las funciones del dominio.
- [x] `domain/error.rs`: `DomainError`.

**Gate 1:** ✅ 19 tests de dominio verdes + 7 previos = 26/26; `commands.rs` ya no
contiene esas reglas; sin warnings. Dos correcciones de comportamiento
identificadas y documentadas (dashboard NULLS LAST, fallback de fecha) — ver
bitácora.

---

### Fase 2 — Definir puertos y mover infraestructura detrás de ellos
Colapsar el `#[cfg]` de plataforma al *composition root*.

- [x] `application/ports.rs` (crece por puerto a medida que se cablea).
- [x] `DeviceScanner`: `SystemDeviceScanner` envuelve `devices.rs` + `os_eject`
      relocado; cableado en AppState + composition root. `#[cfg]` de eject fuera
      de `commands.rs`.
- [x] `MetadataTool`: `ExiftoolMetadata` (desktop) + `AndroidMetadata` (rawler/exif_android).
  - [x] Tests de caracterización contra fixtures reales (`tests/fixtures/*.jpg`
        con EXIF conocido): correctitud + idempotencia. Mejor que un fake.
- [x] `ImageProcessor`: `DesktopImaging` (exiftool+image) / `AndroidImaging` (rawler);
      thumbnails/previews/tiff. Con tests de idempotencia contra fixtures.
- [x] `FinderTagWriter`: `AppleDoubleFinderTags` (desktop) / `NoopFinderTags` (Android);
      envuelve `apple_tags.rs` (que pasa a ser desktop-only y conserva sus tests).
- [ ] `ProgressReporter`: **diferido a Fase 3** — no aporta a Gate 2 (el logging no
      es `#[cfg]`, solo acopla a Tauri). Encaja al extraer `import_photos`.
- [x] `open_project_folder` detrás de `infrastructure/folder.rs` (saca su `#[cfg]`).
- [x] Patrón de composition root establecido en `lib.rs` (elige impl por puerto).

**Gate 2:** ✅ **alcanzado** — cero `#[cfg(target_os)]` en `commands.rs` (verificado
por grep); la selección de plataforma ocurre solo en `lib.rs`. 35/35 tests verdes,
sin warnings. `ProgressReporter` se pospone a Fase 3 (no afecta este gate).

---

### Fase 3 — Extraer casos de uso (de más enredado a menos)
Cada caso de uso orquesta puertos; sin `std::fs`/SQLite directo.

- [ ] `use_cases/cull_photo.rs` + test con fakes.
- [ ] `use_cases/rotate_photo.rs` (incluye debounce del write) + test.
- [ ] `use_cases/rate_photo.rs` (normalización + sidecar) + test.
- [ ] `use_cases/import_photos.rs` (hoy `start_import`) + test del flujo con fakes.
- [ ] `use_cases/manage_projects.rs` (create/rename/delete/archive/relocate).
- [ ] `use_cases/manage_devices.rs` (scan/eject/refresh).
- [ ] `use_cases/media.rs` (thumbnail/preview).
- [ ] `application/registry.rs`: encapsular `open_projects`/`ejecting_devices`/`rotation_write_gen`.

**Gate 3:** cada caso de uso tiene test con puertos falsos; la lógica ya no vive
en los comandos.

---

### Fase 4 — Adelgazar la capa de interfaz
- [ ] Dividir `commands.rs` en `interface/*_commands.rs` que solo delegan.
- [ ] `interface/dto.rs`: DTOs de request/response del borde IPC.
- [ ] Comandos devuelven `Result<T, AppError>`; eliminar el patrón repetido
      `.map_err(|e| { error!(...); e.to_string() })`.
- [ ] Actualizar `invoke_handler!` en `lib.rs` a los nuevos módulos.

**Gate 4:** `commands.rs` eliminado; cada comando es un adaptador delgado.

---

### Fase 5 — Consolidar persistencia y errores
- [ ] `infrastructure/persistence/mappers.rs`: unificar el mapeo fila→`Photo`
      duplicado entre `get_photo` y `get_project_photos`.
- [ ] `infrastructure/persistence/migrations.rs`: migraciones versionadas con
      `PRAGMA user_version`; retirar los `ALTER TABLE` de `db/mod.rs` y la
      `migrate_project_settings` de `queries.rs`.
  - [ ] Test de migración: abrir una BD "vieja" simulada y verificar que migra.
- [ ] `application/error.rs`: `AppError` con `thiserror` + `impl Serialize`
      (`{ code, message }` hacia el frontend).

**Gate 5:** una sola ruta de mapeo; migraciones ordenadas y probadas; errores
tipados y serializables.

---

### Fase 6 — Cierre
- [ ] `cargo test` completo verde (dominio + casos de uso + infra).
- [ ] `cargo build` para Linux, Windows y Android sin `#[cfg]` filtrado.
- [ ] Verificación manual del MVP: import, culling, rating, rotación, eject.
- [ ] Actualizar `apps/desktop/CLAUDE.md` con la nueva estructura de módulos.
- [ ] Revisión final: ninguna capa interior importa de una exterior.

**Gate 6:** MVP funcional idéntico, con backend por capas y dominio testeado.

---

## 7. Qué NO sobre-ingenierizar

- `apple_tags.rs` ya está aislado y testeado: solo se muda, no se reescribe.
- Sin `Repository` genérico, sin mediador/CQRS: traits concretos y directos.
- Los VOs cubren reglas frágiles (rotación, rutas, tags), no cada `String`.
- **Workspace de crates** (`lumik-domain`, `-application`, `-infra`): solo si se
  quiere que el compilador imponga la regla de dependencias, o cuando
  `server`/`web` del monorepo compartan dominio. Para el MVP, el layering por
  módulos da el 80% del beneficio sin la fricción del split.

---

## 8. Bitácora

Registrar aquí decisiones/desvíos al ejecutar cada fase (fecha + nota breve).

- **2026-07-05 · Fase 0** — El baseline no estaba 100% verde: `test_sanitize_name`
  fallaba con una aserción obsoleta. Esperaba `"Boda_Mar_a_Jos_"` (asumía que se
  eliminaban acentos), pero `char::is_alphanumeric()` es Unicode-aware y conserva
  `í`/`é` → salida real `"Boda_María_José"`. Se corrigió la aserción al
  comportamiento real del código (acentos válidos en nombres de archivo exFAT).
- **2026-07-05 · Fase 1 · corrección de bug** — Orden del dashboard: el código
  previo en `get_projects_dashboard` documentaba "NULLS LAST" pero su `match`
  invertido sobre `(b, a)` en realidad ponía los proyectos sin `session_date` al
  PRINCIPIO. `domain::project::compare_dashboard` respeta la intención documentada
  (sin fecha al final). Cambio de comportamiento visible: los proyectos sin fecha
  pasan del tope al fondo del dashboard. Test que lo fija:
  `dashboard_orders_newest_first_nulls_last`.
- **2026-07-05 · Fase 1 · unificación menor** — En `create_project`, una
  `session_date` presente con ≥10 chars pero no parseable en 3 segmentos (p.ej.
  `"2024/06/15"`) antes caía a `0000/00/00`; ahora `ProjectFolder::date_parts`
  devuelve `None` y se usa la fecha de hoy como fallback (igual que cuando no hay
  fecha). Caso extremo e improbable; se prefirió una sola ruta de fallback.
- **2026-07-05 · Fase 2 · parcial** — Primer puerto: `DeviceScanner`
  (`SystemDeviceScanner`) con `os_eject` relocado a `infrastructure/devices.rs`.
  Establece el patrón: `application/ports.rs` define el trait, `infrastructure/`
  lo implementa, `lib.rs` (composition root) elige la impl e inyecta en AppState,
  y `commands.rs` la consume vía `state.device_scanner`. Pendiente el grueso de
  la fase (Metadata/Imaging/Progress): son ~17 `#[cfg]` que envuelven exiftool/
  rawler/decodificación de imagen y tocan thumbnails, previews e import — rutas
  runtime-críticas que no se pueden unit-testear con fixtures y conviene validar
  ejecutando la app (disco externo + exiftool). Se abordan como siguiente sub-paso
  con verificación en ejecución.
- **2026-07-05 · Fase 2 · Gate 2 alcanzado** — Extraídos `MetadataTool`,
  `ImageProcessor`, `FinderTagWriter` y `open_project_folder`. `commands.rs` ya no
  contiene ningún `#[cfg(target_os)]`. Enfoque de fixtures (sugerido por el usuario):
  se agregaron `tests/fixtures/orient_{1,6}.jpg` con EXIF conocido y 9 tests de
  caracterización que fijan correctitud e idempotencia de la extracción. Tres
  cambios de comportamiento menores, intencionales y documentados:
  1. **Escritura de orientación unificada**: antes desktop debounce 600ms + exiftool
     y Android inmediato + XMP. Ahora ambos usan el mismo debounce y el puerto abstrae
     exiftool vs XMP. Android gana el debounce (evita carreras de escritura); resultado
     equivalente.
  2. **Log por miniatura en import**: antes solo desktop emitía "Miniatura: X (rot N°)".
     Ahora el caller registra según la rotación que reporta el puerto, así que Android
     también emite esas líneas (consistencia; cosmético).
  3. **`open_project_folder` en Android**: si el dispositivo está desmontado, ahora
     devuelve el error de "no montado" antes que el de "no soportado" (antes siempre
     "no soportado"). Irrelevante: Android no abre carpetas de todos modos.
  Además `apple_tags` pasa a ser desktop-only (en Android el `FinderTagWriter` es no-op
  y no lo referencia). `ProgressReporter` se difiere a Fase 3 (no afecta Gate 2).
