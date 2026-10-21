# Tasks

## 1. exiftool dentro del `.app`

- [x] 1.1 Comprobar que el `exiftool.pl` de Windows funciona con el Perl del sistema y el `lib/` mínimo de D1: copiar a una carpeta del scratchpad `exiftool.pl` como `exiftool`, `lib/Image/` y `lib/File/RandomAccess.pm`, y correr `/usr/bin/perl <carpeta>/exiftool -ver` y `-b -PreviewImage` sobre un RAW de prueba de `src-tauri/fixtures`. Verificar que imprime `13.59` y que el preview es un JPEG válido. Si falla, cambiar D1 a la distribución Unix 13.59 en `exiftool_unix/` antes de seguir.
- [x] 1.2 En `exiftool.rs`, agregar el resolvedor de D2 (bundle → PATH → `/opt/homebrew/bin` → `/usr/local/bin` en macOS; `"exiftool"` en las demás plataformas), guardado en un `OnceLock`, con un log de la ruta elegida y una función que devuelva el `Command` listo. Usarla en la sesión persistente y en las dos extracciones `-b` de `infrastructure/imaging.rs`. Verificar con tests unitarios de la elección de ruta (función pura sobre el directorio del ejecutable y una lista de candidatos existentes), con `cargo test` y con `cargo check`.
- [x] 1.3 Crear `src-tauri/tauri.macos.conf.json` con `targets` `app` y `dmg`, `minimumSystemVersion` `10.15`, `signingIdentity` `-`, los recursos de D1 y `NSRemovableVolumesUsageDescription` (D4). Verificar con `pnpm tauri build --bundles app` en esta Mac que `Lumik.app/Contents/Resources/exiftool/` tiene el script y `lib/`, que `codesign -dv` reporta `Signature=adhoc` y que `plutil -p Info.plist` muestra `LSMinimumSystemVersion` `10.15` y el texto del permiso.
- [ ] 1.4 Abrir el `Lumik.app` de 1.3 desde Finder, no desde la terminal, con `~/.local/bin/exiftool` renombrado temporalmente: abrir un proyecto de KINGSTON con RAW, regenerar sus miniaturas y abrir una foto en el visor. Verificar que las miniaturas y el preview aparecen y que el log dice que se usó el exiftool del bundle. Restaurar `~/.local/bin/exiftool` al terminar.
- [x] 1.5 Documentar en `apps/desktop/CLAUDE.md` cómo se resuelve exiftool por plataforma y la configuración de macOS. Verificar que coincide con el código de 1.2 y 1.3.

## 2. Atajos con Cmd en macOS

- [x] 2.1 En `src/lib/hooks.ts`, hacer que `matchesKey` use el modificador de la plataforma (D3): `metaKey` en macOS y `ctrlKey` en las demás, exigiendo que el ajeno y Alt no estén pulsados, con la detección síncrona de Mac. Verificar con `pnpm exec tsc --noEmit` y con tests del matcher si el paquete tiene runner. Si no lo tiene, comprobarlo a mano en 2.3.
- [x] 2.2 En `SettingsPage.tsx`, hacer que `formatKey` muestre `⌘<tecla>` en macOS y `Ctrl+<tecla>` en las demás, y que la captura guarde `Ctrl+<e.key>` con el modificador de la plataforma, rechazando Alt y el modificador ajeno. Verificar con `pnpm exec tsc --noEmit`.
- [ ] 2.3 Verificar a mano con `pnpm tauri dev` en macOS: Cmd+C alterna "solo aprobadas" y Ctrl+C no; Ajustes muestra `⌘C`; reasignar "importar" a Cmd+I funciona e `i` sola deja de importar; restaurar el valor original. En Linux (Xorg), comprobar que Ctrl+C sigue funcionando y que Ajustes muestra `Ctrl+C`.

## 3. Release de macOS

- [x] 3.1 Agregar a `.github/workflows/release.yml` el job `build-macos` de D5 (`macos-14`, los dos targets de Rust, versión del tag con `sed -i ''`, `tauri build --target universal-apple-darwin` y subida del `.dmg`). Verificar con `actionlint` si está disponible, o revisando que el YAML se parsee, y comprobar localmente que el comando de `sed` cambia la versión de una copia de `Cargo.toml` en el scratchpad.
- [ ] 3.2 Probar el job sin publicar un release real: lanzarlo en un tag de prueba (`v0.1.4-beta.mac1`) en un fork o rama, o con `workflow_dispatch` temporal, según prefiera el usuario. Verificar que el `.dmg` sale y que `lipo -archs` sobre `Contents/MacOS/lumik` imprime `x86_64 arm64`. Borrar el tag o release de prueba al terminar.

## 4. Limpieza y documentación

- [x] 4.1 Quitar `binaries/dnglab` del `externalBin` de `tauri.windows.conf.json` y borrar `binaries/dnglab-*` (D6). Verificar antes con `grep -rn dnglab apps/desktop` que solo aparece en la config y los binarios, y después con `cargo check`.
- [x] 4.2 Actualizar `docs/ROADMAP.md`: macOS sale del backlog, la tabla de estado incluye macOS donde aplica, la fila de metadatos XMP pasa de "embed en DNG" a "embebido en el archivo", y se anota que la firma con Developer ID y la notarización quedan para la salida al mercado. Agregar a `CHANGELOG.md` 0.1.4 la entrada de macOS con la descarga universal, el mínimo 10.15 y cómo abrirla la primera vez (`xattr -dr com.apple.quarantine /Applications/Lumik.app`, o "Abrir igualmente" en Ajustes del Sistema → Privacidad y seguridad), y la de los atajos con Cmd. Verificar que el `awk` de `create-release` extrae la sección 0.1.4 con esas instrucciones.

## 5. Verificación integral

- [ ] 5.1 Con el `.dmg` de 3.2 en esta Mac (Monterey): instalar en Aplicaciones, quitar la cuarentena con las instrucciones del CHANGELOG y abrir. Verificar: aparece el aviso de volúmenes extraíbles con el texto de Lumik; al permitir, KINGSTON muestra sus proyectos; importar un RAW con firma deja el autor en el archivo (`exiftool -Artist`); Cmd+Q cierra; copiar y pegar con Cmd en el buscador funciona. Revocar el permiso en Privacidad y seguridad y comprobar que la app no se rompe y que el disco se sigue listando.
- [ ] 5.2 Repetir lo básico de 5.1 en la Mac con Catalina (abrir, listar discos, abrir un proyecto con RAW, visor, importar), revisando que la UI se vea bien. Si algo de CSS o JS no funciona en esa WebView, arreglarlo aquí si es puntual; si no, decidir con el usuario si se sube `minimumSystemVersion` y actualizar la spec y el CHANGELOG.
- [ ] 5.3 Correr `openspec validate macos-beta-release --strict`.

## Workflow follow-up

- Archivar con `/opsx:archive` cuando 5.1 y 5.2 estén hechas.
- Para el primer release de beta con testers, publicar el tag `v0.1.4-beta` (u otro) y compartir con ellos las instrucciones de cuarentena.
