# Design

## Context

`DownloadsPage.astro` es una página estática de Astro. Un script en el cliente, sin framework, hace un solo `fetch` a `api.github.com/repos/everitosan/lumik/releases` y pinta las tarjetas. Hoy el script asume dos sistemas:

- IDs paralelos: `releases-list` y `releases-list-win`, `state-loading` y `state-loading-win`.
- `rendered = { linux, windows }`.
- `assetFilter` solo distingue entre Windows y "el resto".
- `activateTab` nunca abre macOS, aunque `detectOS()` lo detecte.

`assetIcon`, `assetDesc` y la clave `dl.desc.dmg` ya conocen el `.dmg`.

El job `build-macos` de `release.yml` sube un único `*.dmg` del target `universal-apple-darwin`. Los releases publicados (0.1.0 a 0.1.2) no tienen `.dmg`.

Los textos viven en `src/i18n/ui.ts`, en `en` y `es`. `LandingPage.astro` pinta el roadmap con claves numeradas (`roadmap.now.i1` … `i10`) escritas una por una.

## Goals / Non-Goals

**Goals:**
- Que los tres sistemas pasen por el mismo código, para que agregar uno sea sumar una entrada de configuración.
- Que la guía de Gatekeeper no dependa de que cargue la API de GitHub.

**Non-Goals:**
- Llevar el roadmap de la landing a un archivo de datos compartido con `docs/ROADMAP.md`. Los dos tienen público y nivel de detalle distintos.
- Cachear la respuesta de GitHub o generar la página en el build.

## Decisions

### Configuración por SO en lugar de IDs paralelos
El script define un objeto con una entrada por sistema: extensiones, texto vacío y texto de hint. El HTML usa IDs con el sufijo del SO (`panel-macos`, `releases-macos`, `loading-macos`, `error-macos`), generados en el frontmatter de Astro a partir de una lista `['linux', 'windows', 'macos']`. `render(os)` es una sola función.
*Alternativa*: agregar un tercer juego de IDs `-mac`. Se descartó porque triplica el código que ya está duplicado.

### Filtrar releases por SO antes de pintar
En cada pestaña se calcula `releases.filter(r => !r.draft && r.assets.some(matches(os)))`. La marca "latest" va al primero de esa lista, no a `releases[0]`. El estado vacío por release (`no-assets`) desaparece: con el filtro ya no puede darse.
*Alternativa*: mostrar todos los releases con "sin paquetes". Se descartó porque en macOS serían tres tarjetas vacías antes de la primera útil.
*Consecuencia*: en la pestaña de macOS, el "latest" puede ser un release más viejo que el último de GitHub si un tag sale sin `.dmg`. Es lo correcto: es el último que se puede descargar en ese sistema.

### Guía de primera apertura fuera del área que se pinta con JS
El bloque de macOS (requisito mínimo, pasos, comando y botón de copiar) es HTML estático dentro de `panel-macos`, encima de la lista. Así se ve aunque falle el `fetch` o JavaScript. El botón usa `navigator.clipboard.writeText` y cambia su texto unos segundos a "Copiado". Si la API del portapapeles no está disponible (HTTP o un navegador viejo), el comando sigue en un `<code>` seleccionable.

### Detección de Mac
Se conserva `detectOS()` (`ua.includes('mac')`) y se abre la pestaña que devuelve, sin forzar Linux. Safari en iPad se anuncia como "Macintosh", así que un iPad caerá en macOS. Es aceptable: no hay paquete para iPad y la pestaña dice explícitamente "macOS 12 o posterior".

### Roadmap con listas en lugar de claves numeradas
Las claves pasan a nombres descriptivos (`roadmap.now.trash`, `roadmap.next.globalTagSearch`, …). `LandingPage.astro` itera un arreglo de claves por grupo, en vez de escribir cada `<li>`. Así agregar o quitar un elemento toca una línea por idioma y una del arreglo, y no se renumera nada.
*Alternativa*: mantener `i1…iN` y renumerar. Se descartó porque el roadmap cambia en casi cada release.

### "Beta" es del producto, no de una plataforma
Todos los releases son `-beta`, así que un badge "Beta" solo en macOS daba a entender que Linux y Windows son estables. Las tres plataformas usan el estado "Disponible", y `platforms.sub` dice una vez que Lumik está en beta. La pestaña de macOS no lleva badge. Lo que de verdad distingue a cada sistema, la fricción de la primera apertura, va en su guía dentro de la página de descargas.
*Alternativa*: poner "Beta" en las tres. Se descartó porque repite tres veces el mismo dato.
"Soon" se queda en el CSS por si vuelve a hacer falta, pero ninguna plataforma lo usa.

### Guías de instalación con un mismo bloque
La nota de SmartScreen de Windows y la guía de Gatekeeper de macOS comparten la clase `.install-guide`: HTML estático al inicio del panel de su sistema, fuera del área que pinta el JS. La de Windows es más corta porque solo son dos clics y no hace falta comando.

## Risks / Trade-offs

- [Límite de la API de GitHub sin token, 60 peticiones por hora por IP] → Ya existe y no cambia; el aviso de error enlaza a los releases.
- [Se publica la landing antes del tag `v0.1.4-beta`] → La pestaña de macOS muestra "aún no hay paquetes para macOS" con la guía visible. Al salir el release aparece sola, sin volver a desplegar.
- [El nombre del `.dmg` cambia, por ejemplo con `_aarch64`] → El filtro va por extensión, no por nombre.

## Migration Plan

Página estática: se compila con `pnpm --filter @lumik/landing build` y se despliega como siempre. Para revertir, basta con volver a desplegar el build anterior. Conviene publicarla después del release 0.1.4 para que la pestaña de macOS ya tenga paquete, aunque no es obligatorio.
