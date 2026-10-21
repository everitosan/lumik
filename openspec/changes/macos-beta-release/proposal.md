# Proposal

## Why

Lumik ya detecta y expulsa discos en macOS, y lista los de solo lectura, pero solo se ha usado con `pnpm tauri dev`, que hereda el PATH de la terminal. Un `Lumik.app` abierto desde Finder no encuentra `exiftool` (`Command::new("exiftool")` solo busca en el PATH mínimo de launchd), así que se queda sin miniaturas ni previews de RAW y sin firma XMP al importar. Además, el release no genera ningún paquete de macOS. Para dar el beta a algunos testers hace falta un `.dmg` que funcione por sí solo en Intel y Apple Silicon. La firma con Developer ID y la notarización se dejan para la salida al mercado.

## What Changes

- **exiftool dentro del `.app`**: la distribución Unix de exiftool (script Perl más `lib/`) va como recurso del bundle de macOS y se ejecuta con el Perl del sistema. Si falta, se busca en el PATH y en las rutas habituales de Homebrew. Lo usan el visor y las miniaturas (preview embebido de RAW) y la firma al importar.
- **Atajos con Cmd en macOS**: un atajo guardado como `Ctrl+<tecla>` se dispara con Cmd en macOS y con Ctrl en el resto de plataformas. Ajustes lo muestra como `⌘<tecla>` en macOS. La captura de atajos en Ajustes guarda el modificador, que hoy se pierde: se guarda solo `e.key`.
- **Permiso de volúmenes extraíbles**: el `Info.plist` lleva `NSRemovableVolumesUsageDescription` con un texto que explica para qué accede Lumik a los discos externos.
- **Release de macOS**: nuevo job en `release.yml` que compila un binario universal (`aarch64` + `x86_64`) para macOS 10.15 Catalina o posterior, firmado ad hoc, y sube un `.dmg` al release del tag.
- **Docs**: el ROADMAP saca macOS del backlog y corrige "embed en DNG". El CHANGELOG 0.1.4 explica cómo quitar la cuarentena del `.app` sin notarizar.
- **Limpieza**: se quita `dnglab` (descartado porque la conversión a DNG era lenta; la metadata vive en la BD) del `externalBin` de Windows y de `binaries/`.

### Fuera de alcance

- Firma con Developer ID, notarización y staple, que tocan al salir al mercado.
- Mac App Store y sandbox.
- Android, aplazado; su verificación pendiente está en `docs/ROADMAP.md`.
- Atajos con otros modificadores (Alt, Shift).

## Capabilities

### New Capabilities

- `desktop-distribution`: cómo se entrega e instala la app de escritorio en cada plataforma: paquetes del release, versiones mínimas, firma y herramientas incluidas en el paquete para que la app instalada funcione sin dependencias externas.

### Modified Capabilities

- `app-settings`: "Atajos de teclado configurables": el modificador es Cmd en macOS y Ctrl en el resto, se muestra con su símbolo y se guarda al reasignar.
- `photo-import`: "Metadatos del fotógrafo": en macOS también se escribe dentro del archivo, como en Linux y Windows.

## Impact

- `apps/desktop/src-tauri/src/exiftool.rs` e `infrastructure/imaging.rs`: resolver el programa de exiftool en lugar del literal `"exiftool"`.
- `apps/desktop/src-tauri/tauri.macos.conf.json` (nuevo): recursos de exiftool, `minimumSystemVersion`, `signingIdentity: "-"` y el `Info.plist` con el texto del permiso.
- `apps/desktop/src-tauri/exiftool_unix/` (nuevo, vendorizado) o descarga en CI, según `design.md`.
- `apps/desktop/src/lib/hooks.ts` (`matchesKey`) y `src/pages/SettingsPage.tsx` (`formatKey`, captura).
- `.github/workflows/release.yml`: job `build-macos`.
- `apps/desktop/src-tauri/tauri.windows.conf.json` y `binaries/dnglab-*`: se quita dnglab.
- `docs/ROADMAP.md`, `CHANGELOG.md` y `apps/desktop/CLAUDE.md`.
- Sin cambios en `apps/server`, `apps/web`, el formato de `project.db` ni la BD global.
