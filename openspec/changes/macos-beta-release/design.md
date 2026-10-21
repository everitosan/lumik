# Design

## Context

- exiftool se lanza en tres puntos: la sesión persistente de `exiftool.rs` (`silent_command("exiftool")`) y dos extracciones binarias `-b` en `infrastructure/imaging.rs` (líneas 155 y 241). Los tres usan el nombre a secas, así que dependen del PATH.
- En Windows funciona porque `tauri.windows.conf.json` empaqueta `binaries/exiftool-x86_64-pc-windows-msvc.exe` (un lanzador) y `exiftool_files/` (Strawberry Perl, `exiftool.pl` y `lib/`, exiftool 13.59, todo versionado en git), y Windows busca primero en la carpeta del ejecutable. En macOS, `Command::new` solo mira el PATH, y una app abierta desde Finder recibe `/usr/bin:/bin:/usr/sbin:/sbin`.
- `exiftool_files/exiftool.pl` busca sus módulos en `<carpeta del script>/lib` (`unshift @INC, "$1/lib"`). El código de exiftool está en `lib/Image/` y `lib/File/RandomAccess.pm`; el resto de `lib/` es la librería estándar de Strawberry Perl.
- Atajos: `matchesKey` (`src/lib/hooks.ts`) solo entiende `Ctrl+` con `ctrlKey && !metaKey`. La captura de `SettingsPage.tsx` guarda solo `e.key`, así que nunca graba un modificador. El único atajo por defecto con modificador es `project.show_culled` (`Ctrl+c`).
- `release.yml` tiene `create-release`, `build-linux` y `build-windows`. La versión se toma del tag con `sed -i` (GNU) sobre `Cargo.toml`.
- `binaries/dnglab-*` (tres binarios, uno solo arm64 para Mac) sigue en el `externalBin` de Windows, pero ningún código lo invoca.

## Goals / Non-Goals

**Goals:**
- Un `Lumik.app` autosuficiente para el beta, generado por CI en cada tag.
- Reutilizar lo que ya está versionado para exiftool, sin una segunda copia.

**Non-Goals:**
- Empaquetar exiftool en Linux. El `.deb` y el `.AppImage` siguen dependiendo del exiftool del sistema; no lo pide este change.
- Cambiar el formato en que se guardan los atajos en la BD global.

## Decisions

### D1. exiftool en macOS: el mismo `exiftool.pl` y `lib/Image` de Windows, con `/usr/bin/perl`

`tauri.macos.conf.json` mapea recursos desde `exiftool_files/`:

```
exiftool_files/exiftool.pl              -> Resources/exiftool/exiftool
exiftool_files/lib/Image/**             -> Resources/exiftool/lib/Image/
exiftool_files/lib/File/RandomAccess.pm -> Resources/exiftool/lib/File/RandomAccess.pm
```

Se ejecuta como `/usr/bin/perl <Resources>/exiftool/exiftool …`. Lo demás de `lib/` lo pone el Perl del sistema. El script no tiene arquitectura, así que sirve igual en el binario universal.

- *Alternativa: vendorizar la distribución Unix (`Image-ExifTool-13.59.tar.gz`) en `exiftool_unix/`.* Duplica unos 20 MB en git y obliga a actualizar dos copias. Queda como respaldo si la verificación de 1.1 muestra que el `exiftool.pl` de Windows depende de algo propio del lanzador.
- *Alternativa: `externalBin` con el script.* Tauri pide un binario por triple (`exiftool-universal-apple-darwin`), lo copia a `Contents/MacOS/` y su `lib/` tendría que quedar junto a él, fuera de `Resources/`. Es más frágil y no aporta nada.
- *Alternativa: pedir a los testers `brew install exiftool`.* Contradice la spec "Herramientas incluidas en el paquete".

### D2. Un único resolvedor de exiftool

`exiftool.rs` expone una función que devuelve el `Command` listo (programa y argumentos iniciales), y los tres puntos de lanzamiento la usan. Orden de resolución, calculado una vez y guardado en un `OnceLock`:

```
macOS:   Resources/exiftool/exiftool existe y /usr/bin/perl existe
           -> /usr/bin/perl <script>
         si no -> primer "exiftool" ejecutable en PATH, /opt/homebrew/bin, /usr/local/bin
otros:   "exiftool" (sin cambios: Windows lo resuelve junto al .exe, Linux por PATH)
```

`Resources` se deduce de `std::env::current_exe()` (`Contents/MacOS/lumik` → `../Resources`) para no pasar el `AppHandle` hasta `imaging.rs`. Con `pnpm tauri dev` no hay bundle, así que se cae al PATH, como hoy. Se registra en el log qué ruta se eligió.

- *Alternativa: `app.path().resource_dir()`.* Es lo canónico en Tauri, pero obliga a inicializar el resolvedor en `setup` y a pasarlo a módulos que hoy no reciben estado. `current_exe` da el mismo resultado dentro del bundle.

### D3. El modificador `Ctrl+` como "modificador de la plataforma"

El valor guardado sigue siendo `Ctrl+<tecla>`, sin migración. `matchesKey` toma como modificador `metaKey` en macOS y `ctrlKey` en las demás plataformas, y exige que el otro no esté pulsado: en Mac, Ctrl+clic es clic secundario. La plataforma se detecta de forma síncrona con `navigator.platform`/`userAgent` (`Mac`), porque `matchesKey` es una función pura que se llama en los handlers y `usePlatform()` es asíncrono. En el WebView de Tauri en Mac, `navigator.platform` es `MacIntel` también en Apple Silicon.

`formatKey` muestra `⌘C` en macOS y `Ctrl+C` en las demás. La captura guarda `Ctrl+<e.key>` cuando está pulsado el modificador de la plataforma, rechaza combinaciones con Alt y con el modificador ajeno, y sigue ignorando las teclas modificadoras solas.

- *Alternativa: un prefijo nuevo `Mod+`.* Es más explícito, pero exige migrar la BD global y el seed de `queries.rs`, y no cambia el comportamiento.

### D4. Bundle de macOS: `tauri.macos.conf.json`

Tauri v2 fusiona sola la configuración `tauri.<plataforma>.conf.json`. Contiene:
- `bundle.targets: ["app", "dmg"]`
- `bundle.macOS.minimumSystemVersion: "12.0"` (era `10.15`; se subió tras la prueba en Catalina, ver Risks)
- `bundle.macOS.signingIdentity: "-"` (firma ad hoc; Apple Silicon no ejecuta código sin firma)
- `bundle.resources` con el mapeo de D1
- `bundle.macOS.infoPlist` (o `Info.plist` junto a `tauri.conf.json`, que Tauri fusiona) con `NSRemovableVolumesUsageDescription`, en español e inglés si el formato lo permite, si no en español

### D5. Job `build-macos`

```
build-macos  (runs-on: macos-14, needs: create-release)
  rustup target add aarch64-apple-darwin x86_64-apple-darwin
  pnpm install
  version del tag en Cargo.toml        (sed -i '' …, BSD)
  pnpm --filter @lumik/desktop tauri build --target universal-apple-darwin
  gh release upload  target/universal-apple-darwin/release/bundle/dmg/*.dmg
```

`macos-14` es arm64. Rust compila la porción `x86_64` de forma cruzada sin SDK extra, y `lipo` (Xcode) une las dos. La compilación cruzada falla si una dependencia nativa necesita `cc` para otro target; las C de `rusqlite` (bundled) y `sysinfo` compilan bien con el clang de Xcode para ambos targets.

### D6. dnglab fuera

Se quita `binaries/dnglab` del `externalBin` de Windows y se borran los tres binarios de `binaries/`. Antes se confirma con `grep` que nada en `src-tauri/src` ni en el frontend lo invoca.

## Risks / Trade-offs

- [Apple retira `/usr/bin/perl` en una versión futura de macOS] → El resolvedor cae al PATH y a Homebrew. Si pasa, se empaqueta un Perl relocalizable; Catalina, Monterey y Sequoia todavía lo traen.
- [`exiftool.pl` de Windows con ajustes para el lanzador .exe] → La tarea 1.1 lo ejecuta con `/usr/bin/perl` y el `lib/` mínimo antes de construir nada encima; si falla, D1 pasa a la distribución Unix.
- [La WebView de Catalina (Safari 13–15) no soporta CSS o JS que la UI usa (`:has()`, `aspect-ratio`, `gap` en flex o `structuredClone`)] → Prueba manual en Catalina en 5.2. Lo que falle se arregla en este change si es puntual; si la lista es larga, se sube `minimumSystemVersion` y se anota. *(Resultado de 5.2: en Catalina la app funciona pero la UI se ve mal; se subió el mínimo a macOS 12 Monterey, cuya WebView es la que se probó en 5.1.)*
- [Permiso de volúmenes extraíbles negado] → El escaneo (`statfs`, `diskutil`) no necesita el permiso, pero leer `project.db` sí. Se verifica en 5.1 que negar no rompe la lista de discos ni la app.
- [Firma ad hoc: Gatekeeper bloquea la primera apertura] → Es aceptado para el beta; las notas del release llevan las instrucciones.
- [El `.dmg` crece unos 20 MB por exiftool] → Es aceptable; Windows ya carga 34 MB.

## Migration Plan

Sin migración de datos. El rollback es revertir el commit: el job de macOS es independiente y no afecta a los de Linux ni Windows.

## Open Questions

- Si `NSRemovableVolumesUsageDescription` se puede localizar con `InfoPlist.strings` desde Tauri o si queda solo en español para el beta. No cambia las tareas.
