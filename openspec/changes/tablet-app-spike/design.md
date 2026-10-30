# Design

## Context

- El núcleo de Lumik (`domain`, `application`, `import`, unas 18k líneas de Rust) no depende de Tauri y habla con el mundo por puertos (`FileStore`, `MetadataTool`, `ImageProcessor`, `ProgressReporter`, `DeviceScanner`, `BackupClient`…). Quedan unas 50 llamadas directas a `std::fs` dentro del núcleo, casi todas en el pipeline de import.
- `project.db` vive en la carpeta del proyecto en el disco externo, en modo WAL.
- Android hoy corre la UI web en Tauri con `MANAGE_EXTERNAL_STORAGE`, que Google Play no aprueba para esta app. exiftool no existe en móvil: Android ya usa rawler y un `.xmp` aparte.
- El UI kit `@lumik/ui` es DOM y no se reutiliza en React Native; sí sus tokens de color y tipografía, los tipos de `packages/ts-types`, los textos de i18n y el cliente del servidor de respaldo.

## Goals / Non-Goals

**Goals:**
- Validar con prototipos los cinco riesgos que deciden la arquitectura.
- Fijar los principios táctiles y los flujos por sistema antes de diseñar pantallas finales.

**Non-Goals:**
- Construir la app ni publicarla.
- Cambiar el desktop, salvo extraer el núcleo a un crate si el spike lo necesita.
- Teléfonos: el objetivo es tablet; en teléfono la app podrá abrir, pero no se optimiza.

## Decisions

### D1. React Native (Expo) con el núcleo Rust compartido

```
 apps/desktop (Tauri, sin cambios)        apps/tablet (Expo / React Native)
 +---------------------------+            +--------------------------------+
 | React DOM + @lumik/ui     |            | RN + Expo UI (SwiftUI/Compose) |
 +-------------+-------------+            | variantes .ios / .android      |
               | IPC Tauri                +---------------+----------------+
               |                                          | uniffi-bindgen-react-native
       +-------v------------------------------------------v-------+
       |  crates/lumik-core  (domain + application + import + db)  |
       +-------+-----------------------------+--------------------+
               |                             |
     infra desktop (exiftool,       infra movil (rawler, .xmp,
     mount watchers, std::fs)       bookmarks iOS / SAF Android)
```

El núcleo incluye `db` (rusqlite con SQLite embebido): las dos apps usan el mismo `project.db`, y es lo que hace portátil el disco. La firma del import por plataforma (pool de exiftool en desktop, rawler + XMP en Android) queda en la infraestructura de cada app, y el núcleo no lleva `#[cfg(target_os)]`.

Alternativas: Tauri también en tablet (una sola UI, pero web y con poca sensación nativa); SwiftUI y Compose por separado (la mejor sensación, pero dos UIs completas del flujo entero para una sola persona). React Native da una UI con piezas nativas reales donde se notan: Expo UI envuelve SwiftUI y Jetpack Compose, `react-native-screens` da la navegación nativa, y las variantes por sistema viven en archivos `.ios.tsx` / `.android.tsx` solo donde hace falta.

### D2. Almacenamiento por plataforma: puerto `ProjectStore`

```
                 desktop            iPad                    Android (Play)
 acceso          std::fs directo    std::fs bajo bookmark   SAF + fd para leer/escribir
 project.db      directo (WAL)      directo, si WAL va en   copia de trabajo en la sandbox
                                    exFAT por USB-C         + devolucion al disco
 .thumbs/        en el disco        en el disco             en la sandbox (regenerables)
 candado         project-lock       project-lock            project-lock (usa `pending`)
```

En Android, la base se copia a la sandbox al abrir y se devuelve al disco (`VACUUM INTO` a un temporal y renombrado por SAF) justo después de cada operación que mueve archivos y, con debounce, tras calificar o etiquetar; también al pasar a segundo plano y al cerrar. Al abrir se reconcilia: cada ruta de la base se verifica en el disco y, si falta, se busca en las otras subcarpetas del proyecto.

### D3. Principios táctiles (los dos sistemas)

- **Objetivos grandes:** 44 pt en iPad y 48 dp en Android como mínimo; estrellas y botones de aprobar de 56 o más.
- **Pulgares en horizontal:** las acciones de culling van en los bordes laterales, al alcance de los pulgares con la tablet sujeta con las dos manos, no arriba al centro.
- **Gestos con alternativa visible:** deslizar arriba aprueba, abajo quita la aprobación, izquierda y derecha cambian de foto; doble toque hace zoom al 100 %, pellizco hace zoom libre. Cada gesto tiene su botón.
- **Toque largo:** abre el menú contextual del sistema (con vista previa en iPad) y entra a la selección múltiple; en iPad también arrastre con dos dedos para seleccionar.
- **Deshacer siempre:** aprobar, mandar a la papelera o calificar muestra "Deshacer" (snackbar en Android, banner en iPad) durante unos segundos. Así no hacen falta confirmaciones.
- **Háptica:** un toque al aprobar, al calificar y al llegar al final de la lista.
- **Nada depende de hover.** Si hay teclado o ratón en iPad, los atajos del desktop (`s`, números para estrellas, flechas) funcionan igual, y el puntero se resalta.
- **Grid y visor juntos en horizontal:** grid a la izquierda y foto grande a la derecha; en vertical, el visor a pantalla completa.
- **Procesos largos visibles:** importar y respaldar muestran un progreso que no se esconde, y en iPad un aviso de "no cierres Lumik" mientras se copia la tarjeta.

### D4. Flujo en iPad (iPadOS: barra lateral, hojas, menús de UIKit)

```
 1. Inicio                                       2. Importar (hoja sobre la vista)
+----------+------------------------------------+   +-----------------------------------+
| Lumik    |  Proyectos              [+] [...] |   |  Cancelar   Importar    Siguiente |
|          |  2026                              |   |-----------------------------------|
| Proyectos|  +--------+ +--------+ +--------+  |   |  Origen                           |
| Discos   |  | Boda   | | XV Ana | | Retrato|  |   |  [ Elegir tarjeta en Archivos ]   |
|  SSD-T7  |  | 312 *  | | 1.2k   | | 80     |  |   |  SD  "FUJI X-T5"   7.2 GB         |
|  SD Fuji |  +--------+ +--------+ +--------+  |   |                                   |
| Respaldo |  (toque largo: menu del sistema)   |   |  300 pares  3 RAW  1 JPG          |
| Ajustes  |                                    |   |  12 JPG se uniran a fotos         |
+----------+------------------------------------+   +-----------------------------------+

 3. Culling en horizontal (grid + visor)
+----------+---------------------------+------------------------------------------+
| < Boda   |  [Todas] [Aprob.] [*3+]   |                       RAW+JPG  [s]       |
|          |  +--+ +--+ +--+ +--+      |                                          |
| Filtros  |  |  | |##| |  | |  |      |              foto grande                 |
|  Todas   |  +--+ +--+ +--+ +--+      |   (pellizco = zoom, doble toque = 100%)  |
|  Aprob.  |  +--+ +--+ +--+ +--+      |                                          |
|  * * *   |  |  | |  | |  | |  |      |  [x]                              [ok]   |
|          |  +--+ +--+ +--+ +--+      |  quitar   * * * * *   (56pt)    aprobar  |
+----------+---------------------------+------------------------------------------+
                   ^ pulgar izquierdo                       pulgar derecho ^
 desliza arriba = aprobar | abajo = quitar | lados = foto anterior / siguiente
 banner: "Aprobada  [Deshacer]"   +  toque haptico
```

### D5. Flujo en Android (Material 3: navigation rail, panel lista-detalle, FAB, snackbar)

```
 1. Inicio                                       2. Importar (pantalla completa, paso a paso)
+---+-------------------------------------------+   +-----------------------------------+
| = |  Proyectos                       [buscar] |   | <-  Importar           1 de 3     |
|   |  2026                                     |   |-----------------------------------|
|[P]|  +--------+ +--------+ +--------+         |   |  Origen                           |
|[D]|  | Boda   | | XV Ana | | Retrato|         |   |  ( Elegir carpeta de la tarjeta ) |
|[R]|  | 312    | | 1.2k   | | 80     |         |   |  SD Card   7.2 GB                 |
|[A]|  +--------+ +--------+ +--------+         |   |                                   |
|   |                              +----------+ |   |  300 pares  3 RAW  1 JPG          |
|   |                              | Importar | |   |              [ Siguiente ]        |
+---+------------------------------+----------+-+   +-----------------------------------+
 rail: Proyectos / Discos / Respaldo / Ajustes      selector SAF la primera vez (permiso persistente)

 3. Culling en horizontal (lista-detalle adaptable)
+---+---------------------------+-------------------------------------------+
|   | <- Boda       [filtros v] |                         RAW+JPG   [s]     |
|[P]|  +--+ +--+ +--+ +--+      |                                           |
|[D]|  |  | |##| |  | |  |      |               foto grande                 |
|[R]|  +--+ +--+ +--+ +--+      |                                           |
|[A]|  +--+ +--+ +--+ +--+      |  (x)                               (ok)   |
|   |  |  | |  | |  | |  |      |  quitar   * * * * *   (56dp)    aprobar   |
+---+---------------------------+-------------------------------------------+
 snackbar abajo: "Aprobada              DESHACER"    + vibracion corta
 atras predictivo: deslizar desde el borde cierra el visor y vuelve al grid
```

### D6. Lo que cambia entre sistemas y lo que no

| | iPad | Android |
|---|---|---|
| Navegación | barra lateral (split view) | navigation rail |
| Importar | hoja modal sobre la vista | pantalla completa paso a paso |
| Acción principal | botón `+` en la barra superior | FAB "Importar" |
| Menú contextual | UIContextMenu con vista previa | menú de Material anclado |
| Deshacer | banner arriba | snackbar abajo |
| Volver | deslizar desde el borde izquierdo | atrás predictivo |
| Iconos | SF Symbols | Material Symbols |
| Igual en los dos | grid + visor en horizontal, gestos de culling, estrellas de 56, háptica, atajos de teclado | |

## Risks / Trade-offs

- [uniffi-bindgen-react-native es joven] → El experimento 1 lo prueba con el import real y su progreso. Si falla, la alternativa es un módulo nativo de Expo por sistema que llame al núcleo por C ABI.
- [iOS suspende la app en segundo plano] → Aviso de "no cierres Lumik" mientras se copia, e import reanudable con el reintento existente (`import-retry-failed`).
- [SAF es lento para muchas operaciones chicas] → Las miniaturas y vistas previas van a la sandbox en Android; en el disco solo se copian los originales y se escribe la base.
- [Dos UIs de producto (desktop DOM y tablet RN)] → Se comparten núcleo, tipos, textos y tokens de diseño. Las pantallas se diseñan juntas para que el flujo sea el mismo.
- [WAL sobre exFAT por USB-C en iPadOS puede no funcionar] → El experimento 3 lo decide; si falla, el iPad usa también el modo espejado.

## Decisión (4.1, 2026-10-30)

**Seguir con la arquitectura en Android y evolucionar el prototipo a la app real** (`apps/tablet-spike` → `apps/tablet`). El iPad queda pendiente de la cuenta de Apple Developer: sus experimentos (2.2, 2.3 y la mitad iPad de 1.2, 2.1 y 2.4) se harán en el change que construya la app, antes de publicar en iPad.

### Resultados (Android, Huawei DBR-W09, Android 12, build de release)

| Riesgo | Resultado | Veredicto |
|---|---|---|
| 2.1 Puente RN ↔ Rust (uniffi) | 1 000 eventos en 1.5 s con la UI a 52–60 fps; 2.3 GB copiados con el JS a 58 fps como mínimo. En debug, cada callback frena la copia | D1 se confirma. La app limita los eventos de progreso (uno cada ~100 ms) y las cifras solo valen en release |
| 2.4 Grid y visor | FlashList con 2 000 miniaturas a 57 fps de promedio; visor con pellizco, doble toque y deslizar, que sigue al dedo | Se confirma. Falta la validación del fotógrafo en una sesión real |
| 2.5 SAF y modo espejado | El permiso sobrevive a reinicios y a reinstalar; devolver `project.db` tarda 0.2–0.3 s con `integrity_check` ok; la copia SD → disco va a ~27 MB/s, limitada por la escritura de Android en el disco, ~2× el desktop | D2 se confirma para Android |
| 3.1 Prototipo D5 | Inicio, crear, editar, archivar y eliminar proyectos, import con el núcleo, culling, papelera, filtros, candado, discos con nombre y expulsar: todo con los casos de uso del núcleo sobre SAF | Se confirma; el flujo sigue las reglas del desktop |
| 2.2, 2.3 iPad | Sin probar | Pendiente |

### Ajustes a las decisiones

- **D3 "cada gesto tiene su botón" se ajusta.** A pedido del fotógrafo, el visor no tiene botones de aprobar y quitar. Los gestos se enseñan con una guía de dedos animados y todo se puede deshacer. Las estrellas y las etiquetas de color siguen siendo botones grandes.
- **D5 se ajusta a las reglas del desktop.** No hay FAB de importar en el inicio: se importa desde dentro de un proyecto. El disco es obligatorio al crear un proyecto. Los discos van en el rail. "‹" y el botón atrás regresan un nivel. El visor usa todo el ancho con una tira de miniaturas abajo, en lugar del grid de lista y detalle.
- **D2, reglas de SAF aprendidas:**
  - Ningún archivo del disco se sobrescribe en su lugar: en esta tablet quedaba vacío al desmontar. Se escribe un temporal, se borra el anterior y se renombra (`safWrite.ts`).
  - Los archivos se crean como binarios, para que Android no les agregue una extensión.
  - Android no da la etiqueta del disco: se usa `lumik/.disk.json` (change `disk-naming`).

### Lo que falta antes de publicar en Android

- La sesión real de culling de 200 fotos y una tanda grande con el disco real comparada con el desktop (3.1).
- Quitar del prototipo la pestaña de experimentos y lo que solo era medición.
- Las specs de la app (import en la tablet, culling táctil, modo espejado, discos), en el change que la construya.
- Retirar `MANAGE_EXTERNAL_STORAGE` del build de Android de Tauri cuando la app de tablet lo reemplace.
