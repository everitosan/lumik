# Design

## Context

- El logo está en `packages/ui/src/components/Logo/Logo.tsx` (y como archivo en `apps/landing/public/icon.svg`). Su viewBox es de 635×635. Lo forman un cuerpo oscuro (`#1F252F`), que es un solo trazo con forma de L, y tres páginas: triángulos con degradado y opacidad de 0.85, 0.6 y 0.6. Las tres páginas comparten el vértice inferior izquierdo, cerca de `(57, 537)`, que coincide con la esquina del lomo.
- Medidas desde ese vértice: lomo ~541; páginas ~455, ~552 y ~553; base ~537. Acostadas, las páginas 2 y 3 sobresalen unas 15 unidades del largo de la base.
- Las dos apps son de tema oscuro (`--lumik-background: #131313`; la tablet usa `#131313`). El cuerpo del libro, `#1F252F`, tiene un contraste de ~1.2:1 contra ese fondo.
- Arranque actual:
  - **Desktop:** `main.tsx` monta `App` directamente. `index.html` no tiene fondo, y la ventana de Tauri no define color de fondo.
  - **Tablet:** `App.tsx` monta `AppScreen` sobre `#131313`. El splash del sistema lo genera `expo prebuild` con su imagen por defecto (`assets/splash-icon.png` es la de la plantilla de Expo), y la app no depende de `expo-splash-screen`.
- La tablet ya tiene `react-native-svg` (15.16) y `react-native-reanimated` (4.7). La tablet no depende de `@lumik/ui`, porque ese paquete es React DOM.

## Goals / Non-Goals

**Goals:**
- Una sola definición de la geometría del logo partido y de la línea de tiempo, usada por las dos apps, para que la animación se vea igual en las dos.
- Que el último cuadro de la animación sea exactamente el logo actual.
- Que el splash no retrase ni bloquee la app: la app se monta y carga debajo desde el inicio.

**Non-Goals:**
- Un formato de animación externo (Lottie, video).
- Que el splash espere a los datos.
- Animar el logo en otros lugares (barra lateral, Acerca de).

## Decisions

### D1. El logo partido, como datos compartidos

Se crea un módulo de TypeScript sin dependencias, `packages/ui/src/brand/logo.ts`, exportado como `@lumik/ui/brand`. Contiene:
- el viewBox;
- el trazo del cuerpo y la recta que lo parte en **tapa** (la barra vertical de la L) y **contratapa** (la base): el borde superior de la contratapa (D2);
- los tres trazos de página, con sus degradados y opacidades;
- el nombre "lumik" del logotipo como trazos (D2b);
- el eje de giro;
- la línea de tiempo (D3).

`Logo.tsx` pasa a dibujarse con estos datos. La tablet agrega `@lumik/ui` como dependencia de workspace, pero solo importa `@lumik/ui/brand`, que no toca React DOM. Metro resuelve los `exports` del paquete.

El corte no edita el trazo: la tapa y la contratapa son el mismo cuerpo recortado por un `clipPath` a cada lado de la recta, y la tapa se solapa 4 unidades bajo la recta para que no quede una línea entre las dos. En el ícono, las páginas van debajo del cuerpo, y `Logo` pasa a dibujarlas en ese orden. Se comprueba renderizando el logo hecho con estos datos y el `icon.svg` actual, y comparándolos píxel a píxel.

**Alternativas:**
- Copiar los trazos en cada app: se desincronizan.
- Lottie: agrega `lottie-web` y `lottie-react-native`, y un archivo que se edita con otra herramienta. No hace falta para cinco piezas que solo giran, escalan y se desvanecen.

### D2. Todo gira alrededor de un solo eje, en el plano

El libro se ve desde su borde inferior, así que la apertura es una rotación 2D, sin perspectiva:
- **La tapa** gira 90° alrededor del eje, de acostada (sobre las páginas, apuntando a la derecha) a de pie (la barra de la L).
- **Cada página** gira de acostada (0°) a su ángulo final en el logo.
- **La contratapa** no se mueve.

La barra de la L no tiene grosor uniforme: es una cuña de ~18 unidades en el lomo y ~185 arriba. Girada sin más, atravesaría la contratapa. Por eso **la tapa y las páginas solo se dibujan por encima del borde superior de la contratapa**, con un `clipPath` fijo, que no gira con ellas:
- **Cerrado** (`BOOK_CLOSED`): la tapa está girada 84.5° y de ella se ve una franja sobre la contratapa, así que el libro es un bloque. Cada página está girada hasta que su borde superior queda bajo la recta, así que está entera dentro del libro.
- **Al abrir:** las páginas salen de la contratapa mientras giran, y así se ve que brotan del lomo.

Esto sustituye la idea de escalar las páginas de 0.9 a 1.

El orden de dibujo, de atrás hacia adelante, es: resplandor, páginas, contratapa, tapa. Es el del ícono: las páginas debajo del cuerpo. Cuando las piezas llegan a su lugar, el splash dibuja el logo sin recortes, idéntico al ícono, hasta el desvanecimiento. Así no queda la línea de un píxel que el recorte deja en el borde.

**Alternativa descartada:** abrir la tapa con `rotateY` y perspectiva (verla desde arriba). En el desktop de Linux (WebKitGTK), el 3D dentro de un SVG no es confiable, y además no corresponde a la L del logo.

### D2b. El nombre del logotipo, como trazos

El nombre solo existe como imagen (`apps/landing/public/logotipo.png`): minúsculas monolínea con extremos redondos y el punto de la "i" dorado. Se reconstruye midiendo la imagen: cada letra son líneas y arcos de grosor uniforme (`l`; `u`; `m`; la `i` con su punto; la `k` con su asta y dos diagonales), dibujados con `stroke` y `strokeLinecap="round"`. Se escalan a las coordenadas del logo con la proporción del logotipo (el libro y el nombre, centrados, con la misma separación). Color: `#E5E2E1` (el texto claro de la app) sobre el fondo oscuro, y el punto en `#E9C349`, el dorado de la marca.

**Animación, en el estilo de las páginas:** las páginas brotan del borde de la contratapa y las letras brotan de su línea base. Cada letra se recorta por encima de esa línea y sube desde debajo, con el mismo rebote leve, de izquierda a derecha. El punto aparece al final con un pequeño salto de escala.

**Alternativa descartada:** vectorizar el PNG con un trazador. Daría contornos irregulares y no trazos limpios, y el PNG es de una versión anterior del logo.

### D3. Línea de tiempo (en `@lumik/ui/brand`)

| Pieza | Desde | Hasta | Movimiento |
|---|---|---|---|
| Resplandor | 0.00 s | 0.30 s | opacidad 0 → 1 |
| Libro cerrado | 0.15 s | 0.35 s | opacidad 0 → 1 |
| Tapa | 0.40 s | 0.85 s | 0° → 90°, salida suave |
| Página 1 | 0.50 s | 0.95 s | 0 → su ángulo, con un leve rebote |
| Página 2 | 0.60 s | 1.05 s | igual |
| Página 3 | 0.70 s | 1.15 s | igual |
| Letras l, u, m, i, k | 1.00 s (+0.06 s cada una) | +0.30 s | suben desde la línea base, con un leve rebote |
| Punto de la "i" | 1.45 s | 1.70 s | escala 0 → 1, con un leve rebote |
| Todo (logo, nombre y resplandor) | 1.75 s | 2.05 s | opacidad 1 → 0 |

Cada app interpola estos números con su propio motor; no se comparte una función. Las curvas (salida suave y rebote leve) se nombran en la tabla y cada plataforma usa su equivalente.

### D4. Desktop: componente de React encima de la app

`LogoSplash` vive en `packages/ui` junto a `Logo`. Es un SVG de pantalla completa con un `<g>` por pieza, y en cada cuadro (`requestAnimationFrame`) pone el atributo `transform="rotate(a cx cy) scale(...)"`. Se usa el atributo del SVG, no CSS `transform-origin` ni `transform-box`, porque su comportamiento en SVG cambia entre los webviews de Tauri (WebKitGTK, WKWebView, WebView2).

`main.tsx` monta `<LogoSplash />` junto a `<App />`. La app se dibuja y empieza a cargar debajo desde el primer momento. Al terminar el desvanecimiento, el splash se desmonta y deja de capturar clics.

Para que no haya destello blanco:
- `index.html` pone `background: #131313` en `html` y `body`;
- la ventana define `backgroundColor: "#131313"` en `tauri.conf.json`.

"Reducir movimiento" se detecta con `matchMedia('(prefers-reduced-motion: reduce)')`.

**Alternativa descartada:** el splash como SVG y CSS escrito en `index.html`, para que aparezca antes de cargar React. Duplica la geometría fuera de D1. En Tauri el bundle es local y React se monta en pocos cientos de milisegundos sobre el fondo ya oscuro, así que la ganancia no lo justifica.

### D5. Tablet: splash del sistema y luego la animación en React Native

- **Splash del sistema:** `expo-splash-screen` como plugin en `app.config.ts`, con `backgroundColor: '#131313'` y una imagen del libro cerrado (`assets/splash-book.png`, generada desde la geometría de D1) con un ancho fijo en dp. Reemplaza `splash-icon.png`.
- **Animación:** `LogoSplash` de la tablet (`src/LogoSplash.tsx`) dibuja las piezas con `react-native-svg` y las anima con Reanimated: valores compartidos con `withDelay`, `withTiming` y `withSpring`, y `useAnimatedProps` en cada `G`.
- **Paso sin salto:** `App.tsx` llama a `SplashScreen.preventAutoHideAsync()` al cargar el módulo. Cuando `LogoSplash` ya está dibujado (`onLayout`), llama a `SplashScreen.hideAsync()` y arranca la línea de tiempo. El libro cerrado de la animación se dibuja centrado y con el mismo tamaño en dp que la imagen del sistema.
- **"Reducir movimiento":** `useReducedMotion()` de Reanimated.

Se monta una sola vez en la raíz, encima de `AppScreen`. Volver desde segundo plano no remonta la app, así que no se repite (spec "Splash al arrancar").

## Risks / Trade-offs

- **[El icono del splash del sistema tiene medidas y máscara impuestas]** Desde Android 12, el sistema muestra la imagen dentro de un área de ~288 dp con una zona visible circular de ~192 dp, y el tamaño puede variar entre fabricantes. → El libro cerrado se diseña para caber en ese círculo. El tamaño se ajusta en la DBR-W09 y en el emulador. El resplandor, que aparece en la animación, disimula diferencias de pocos píxeles.
- **[El corte del cuerpo puede notarse]** Con antialiasing, dos trazos que comparten un borde pueden dejar una línea fina. → Las dos piezas se solapan unas unidades bajo la tapa, y se compara píxel a píxel el logo de piezas contra el actual.
- **["Reducir movimiento" en Linux]** WebKitGTK no siempre refleja la preferencia del sistema en `prefers-reduced-motion`. → Se acepta: en ese caso se ve la animación completa, que es corta.
- **[~2 s en cada arranque]** Un fotógrafo abre la app muchas veces. → La duración es fija y corta, y la app carga debajo mientras tanto. Si resulta molesto, se puede acortar la línea de tiempo sin cambiar la spec más allá del número aproximado.
- **[La tablet depende de `@lumik/ui`]** Si alguien importa la raíz `@lumik/ui` en la tablet, arrastra React DOM. → La tablet solo importa `@lumik/ui/brand`. Se documenta en `apps/tablet/CLAUDE.md`.

## Migration Plan

No hay datos ni ajustes que migrar. Es visual y se revierte quitando el componente del arranque de cada app.
