# Design

## Context

- El prototipo vive en `apps/tablet-spike` (Expo SDK 58, RN 0.88, Fabric). La pestaña "3.1 Prototipo" (`src/proto/`) es la app. Las pestañas "2.1 Puente", "2.4 Grid y visor" y "2.5 Disco (SAF)" (`BridgeScreen`, `SafScreen` y el modo benchmark de `CullingScreen`) son experimentos. La navegación es por estado (`useState`), como en el desktop.
- El núcleo llega por `crates/lumik-mobile` (uniffi 0.31) y el turbo module `modules/core-native`. SAF va por el módulo Expo local `modules/saf`. `lumik-mobile` todavía exporta funciones de los experimentos (`start_test_import`, `start_progress_burst`, `start_fd_copy`, `start_fd_read`, `start_fd_write`, `start_copy_to_dir`).
- El import de la tablet llama a `ImportPhotos` con `metadata: None` y `rename_on_import: true` fijos: no firma ni lee ajustes. El culling usa `CullPhoto`, que ya recibe la opción de entrega del proyecto.
- `android/` se genera con CNG y no se versiona. Hoy el release se firma con la clave de debug y `versionCode` es 1.
- El desktop se compila para Android con Tauri. Lo específico de Android está en `#[cfg(target_os = "android")]` en 12 archivos de `src-tauri/src`, en `exif_android.rs`, en `gen/android`, en una sección de dependencias de `Cargo.toml` y en ramas de la UI (`Sidebar`, `Layout`, `Projects`, `ProjectDetail`, `PhotoDetail`, `PhotoSidebar`, `ImportPage`, `hooks.ts` y `api.ts`). El workflow `release.yml` no construye Android.

## Goals / Non-Goals

**Goals:**
- Una app de tablet Android publicable en Google Play, hecha del prototipo y sin código de experimentos.
- Que el import firme y respete los ajustes como en el desktop.
- Un solo camino a Android: la app de tablet. El build de Tauri para Android se retira.

**Non-Goals:**
- iPad: espera la cuenta de Apple. El código queda listo para sumarlo (Expo).
- Respaldo, búsqueda, análisis por contenido, álbumes y rotación en la tablet.
- Cambiar la opción de entrega del proyecto desde la tablet: se respeta la que tiene, y se cambia en el desktop.
- Idioma inglés en la tablet: la v1 sale en español (ver Riesgos).
- Anunciar Android en la landing.

## Decisions

### D1. Renombrar, no reescribir

`git mv apps/tablet-spike apps/tablet`. El paquete pasa a `@lumik/tablet`, el nombre visible a "Lumik" y el id a `com.lumik.tablet`. `src/proto/` sube a `src/` y `PrototypeScreen` pasa a ser la raíz de `App.tsx`, sin pestañas. Se borran `BridgeScreen`, `SafScreen`, el benchmark de `CullingScreen` y las funciones de experimentos de `lumik-mobile` que ya nadie llama.

Cambiar el id hace que la app nueva no herede los permisos de SAF ni los ajustes del prototipo. Se acepta: el fotógrafo vuelve a dar acceso a los discos una vez.

*Alternativa:* conservar `com.lumik.tabletspike`. Se descarta porque el id de Play no se puede cambiar después de publicar.

### D2. Navegación por estado

Se mantiene la navegación por estado de tres niveles (inicio → proyecto → foto, más import y ajustes), con `BackHandler` para el botón atrás, como en el desktop. Se actualiza el `AGENTS.md` de la app, que pide Expo Router.

*Alternativa:* Expo Router. Se descarta por ahora: aportaría enlaces profundos que la app no necesita y obligaría a reescribir la navegación que ya está probada.

### D3. Ajustes de la tablet

Una pantalla "Ajustes" en el rail (hoy desactivado) con:
- fotógrafo: autor, copyright y URL;
- "Firmar las fotos al importar" y "Renombrar por fecha de captura al importar";
- nombre de la tablet.

Se guardan en la sandbox, junto a los ajustes actuales del prototipo (`prefs.ts`). El nombre de la tablet se guarda en el archivo de identidad de `install_identity`, que gana una función `set_install_name` en `lumik-mobile`.

`start_card_import` recibe un registro `ImportSettings { sign, rename, author, copyright, url }` y lo pasa a `ImportParams` (`metadata` y `rename_on_import`). La firma la escribe `adapters::AndroidMetadata` en un sidecar `.xmp`, como ya hacía la app Tauri en Android, y el `.xmp` se sube al disco con los demás archivos.

### D4. Espacio libre antes de importar

El paso de importar suma el tamaño de los archivos elegidos (ya se conoce al elegirlos) y lo compara con `freeBytes` del volumen (`StatFs`, de `disk-naming`).

### D5. Retirar Tauri Android

- Se borran `gen/android`, `exif_android.rs` y las ramas `#[cfg(target_os = "android")]` de `src-tauri`, junto con su sección de dependencias.
- Donde una función existía solo por Android (`scan_mounted_devices_android`, `friendly_volume_name`, `AndroidMetadata` y `AndroidImaging` del desktop), se borra.
- `lumik-mobile` conserva sus propias copias (`exif.rs`, `adapters.rs`), así que la tablet no pierde nada.
- En la UI del desktop se quitan las ramas `platform === 'android'`.

*Alternativa:* dejar el código sin compilar. Se descarta porque se pudre sin que nadie lo note.

### D6. Google Play

```
tag vX.Y.Z-beta.N
   |
   v
release.yml: job build-tablet-android (ubuntu)
   - Rust + NDK r26 + cargo-ndk -> ubrn build android --release (arm64-v8a)
   - expo prebuild (CNG) con app.config.ts: version y versionCode del tag
   - gradle bundleRelease firmado con la clave de subida (secretos)
   - AAB al release de GitHub
   - AAB a la pista interna de Play (cuenta de servicio, si está el secreto)
```

`app.config.ts` reemplaza a `app.json` para leer la versión del entorno. El código de versión sale del tag: `major*1 000 000 + minor*10 000 + patch*100 + beta`, y una estable usa `99` en lugar del número de beta. Así `0.2.0-beta.3` → 20003 y `0.2.0` → 20099, y siempre crece. La firma de release se configura con un *config plugin* que lee el keystore y las contraseñas de variables de entorno, sin versionar nada secreto.

*Alternativa:* EAS Build. Se descarta por ahora: el núcleo de Rust necesita NDK y `cargo-ndk` en la máquina de build, y prepararlo en EAS cuesta más que en GitHub Actions, donde ya se construye el desktop.

### D7. Rama

Se integra `spike/tablet-app` en `release/0.2.0` (avance rápido: `release/0.2.0` no tiene commits propios después del inicio del spike). El trabajo sigue en `release/0.2.0` y la tablet sale con los tags `v0.2.0-beta.N`.

## Risks / Trade-offs

- [Revisión de Play por el acceso a carpetas] → SAF con selector de carpetas no necesita declaración especial. Se evita cualquier permiso de almacenamiento amplio (spec `tablet-distribution`).
- [Fotógrafos que usaban Lumik en Android con Tauri] → El APK de Tauri nunca se publicó en el workflow de release. Se avisa en las notas de la beta que Android pasa a la app de tablet.
- [Solo español] → `app-settings` exige español e inglés para el desktop. La tablet no lo hereda en esta versión. Queda anotado para un change posterior.
- [Rendimiento solo medido en una Huawei] → La pista interna de Play permite probar en otras tablets antes de abrir la beta.
- [El escaneo de un disco grande en el inicio es lento por SAF] → Ya se lee en paralelo por disco y las portadas llegan después. Si en campo resulta lento, se guardará en caché la lista de proyectos por disco.

## Migration Plan

1. Integrar `spike/tablet-app` en `release/0.2.0`.
2. Renombrar y limpiar la app, y añadir ajustes, firma y espacio libre.
3. Retirar Tauri Android y adaptar las specs.
4. Crear la app en Play Console y configurar los secretos.
5. Subir la primera beta a la pista interna.

Para volver atrás, los commits de retirar Tauri Android van aparte y se pueden revertir solos.

## Open Questions

- Nombre visible en Play ("Lumik" o "Lumik Tablet") y textos de la ficha: se deciden al crear la app en Play Console, sin cambiar el código.
