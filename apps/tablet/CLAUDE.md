# Lumik para tablet (Android)

App de tablet de Lumik: proyectos, import de la tarjeta y culling táctil sobre el
mismo disco que usa el desktop. Expo / React Native con el núcleo de Rust
(`crates/lumik-core`) por uniffi (`crates/lumik-mobile`). El iPad se sumará cuando
exista la cuenta de Apple Developer.

Comportamiento esperado: `openspec/specs/tablet-*` (almacenamiento, import,
culling, ajustes y distribución) en la raíz del monorepo. Reglas de Expo,
navegación, build y SAF: `AGENTS.md`.

## Estructura

```
App.tsx                 # raíz: monta AppScreen y la barra de estado (modo limpio)
src/
  AppScreen.tsx         # navegación por estado: inicio → proyecto → foto, import
  HomeScreen.tsx        # rail (Proyectos, Discos, Ajustes), "Dar acceso", nuevo proyecto
  ProjectsDashboard.tsx # tarjetas por año, orden, archivados, menú del proyecto
  ProjectScreen.tsx     # grid del proyecto con filtros y papelera
  CullingScreen.tsx     # visor: tira, estrellas, colores, deshacer, guía
  Viewer.tsx            # gestos del visor (Gesture Handler + Reanimated)
  GestureGuide.tsx      # guía de gestos con dedos animados
  PhotoInfoPanel.tsx    # histograma y datos de la toma
  Icon.tsx              # íconos Lucide con los mismos nombres que `Icon` de `@lumik/ui`
  ImportScreen.tsx      # import en tres pasos (archivos, ajustes, importar) desde el proyecto
  SettingsScreen.tsx    # fotógrafo, firmar y renombrar al importar, nombre de la tablet
  importer.ts           # import: tarjeta → sandbox + staging del disco → `_media/`
  session.ts            # proyecto abierto: copia de trabajo, devolución, candado
  projectDb.ts          # traer y devolver `project.db` (VACUUM INTO + renombrado)
  projects.ts           # buscar proyectos en los discos (como el desktop)
  projectActions.ts     # editar, archivar, eliminar
  createProject.ts      # crear proyecto (`lumik/{año}/{mes}/{día}_{nombre}/`)
  covers.ts             # datos y portadas de las tarjetas, en segundo plano
  disk.ts               # `DiskFiles` del núcleo sobre SAF
  diskName.ts           # nombre del disco (`lumik/.disk.json`) y descripción
  lock.ts               # candado `.lumik-lock` (identidad de la tablet)
  safWrite.ts           # escribir sin sobrescribir (temporal + renombrado)
  prefs.ts, paths.ts    # ajustes en la sandbox y su carpeta
modules/
  saf/                  # módulo Expo (Kotlin): SAF, volúmenes, capacidad, filesDir
  core-native/          # turbo module generado por ubrn desde crates/lumik-mobile
```

## Reglas

- **Modo espejado:** `project.db` se usa desde una copia en la sandbox y vuelve al
  disco unos 3 s después del último cambio (`GIVE_BACK_DEBOUNCE_MS` en
  `session.ts`), al pasar a segundo plano, al salir y al expulsar.
- **Mismas reglas que el desktop:** se importa desde un proyecto, el disco es
  obligatorio al crear, los discos van en el rail y "‹" / atrás regresan un nivel.
- **Ningún archivo del disco se sobrescribe en su lugar** (`replaceFile`).
- **Errores del import con su etapa** (tarjeta, sandbox o disco), también en logcat.
- Probar en la tablet con hub: compilar e instalar con cable, `adb logcat -c` y
  `adb logcat -G 16M`, desconectar, probar y leer el log al reconectar.

## Versión y publicación (Google Play)

- `app.config.ts` toma la versión de `LUMIK_VERSION` (el tag sin la `v`) y calcula
  `versionCode` = `major*1 000 000 + minor*10 000 + patch*100 + beta` (estable = 99):
  `0.2.0-beta.3` → 20003, `0.2.0` → 20099.
- `plugins/withReleaseSigning.js` firma el release con la clave de subida si están
  las variables `LUMIK_UPLOAD_STORE_FILE`, `LUMIK_UPLOAD_STORE_PASSWORD`,
  `LUMIK_UPLOAD_KEY_ALIAS` y `LUMIK_UPLOAD_KEY_PASSWORD`; sin ellas usa la de debug
  (builds para probar en la tablet).
- `android.blockedPermissions` quita los permisos de almacenamiento amplio y
  `SYSTEM_ALERT_WINDOW` que pudiera agregar una dependencia.
- El job `build-tablet-android` de `.github/workflows/release.yml` construye el AAB en
  cada tag `v*`, lo sube al release y, si existe la cuenta de servicio, a la pista
  interna con `scripts/play_upload.py`.

Secretos del repositorio que usa el workflow:

| Secreto | Qué es |
|---|---|
| `LUMIK_UPLOAD_KEYSTORE_BASE64` | keystore de la clave de subida, en base64 (`base64 -w0 upload.jks`) |
| `LUMIK_UPLOAD_STORE_PASSWORD` | contraseña del keystore |
| `LUMIK_UPLOAD_KEY_ALIAS` | alias de la clave |
| `LUMIK_UPLOAD_KEY_PASSWORD` | contraseña de la clave |
| `PLAY_SERVICE_ACCOUNT_JSON` | JSON de la cuenta de servicio con permiso de publicar la app (opcional) |

La primera subida de una app nueva a Play Console es manual (Play no acepta la API
hasta entonces); desde la segunda, el workflow.
