# Proposal

## Why

Al abrir Lumik, el desktop entra directo a la app y la tablet muestra el splash por defecto de Expo (la imagen de plantilla, no el logo). Ninguna de las dos presenta la marca. El logo de Lumik es un libro con páginas de color, y un splash animado puede contar en un par de segundos qué es Lumik: un álbum que se abre para ver tus fotos.

## What Changes

- **Splash animado al arrancar el desktop y la tablet Android:**
  - Sobre el fondo de la app (`#131313`) aparece un resplandor y, recortado contra él, el libro cerrado visto desde su borde inferior: un rectángulo.
  - La tapa gira 90° sobre el lomo hasta quedar de pie, y las páginas de color se levantan sobre el mismo eje, una tras otra, hasta formar el logo.
  - Debajo, el nombre "lumik" del logotipo (minúsculas de trazo redondeado, con el punto dorado de la "i") sale de su línea base letra por letra, con el mismo rebote que las páginas.
  - El logo completo se desvanece con transparencia y deja ver la app.
- **Duración fija** de ~2 s, independiente de lo que tarde en cargar la app. Solo en el arranque en frío; volver a la app desde segundo plano no lo muestra.
- **Con "reducir movimiento"** activado en el sistema, se muestran el logo y el nombre quietos con el resplandor y el mismo desvanecimiento.
- **En la tablet**, el splash del sistema (obligatorio desde Android 12) muestra el libro cerrado sobre `#131313`, en el mismo lugar y tamaño donde arranca la animación, para que el paso entre los dos no se note. Sustituye la imagen de Expo.
- **En el desktop**, la ventana es `#131313` desde el primer instante: sin destello blanco antes de que cargue la interfaz.
- **El cuerpo del logo se parte en tapa y contratapa** para poder animar la tapa. Al terminar, las piezas vuelven a formar exactamente el logo actual.
- **El nombre del logotipo pasa a trazos vectoriales.** Hoy solo existe como imagen (`apps/landing/public/logotipo.png`); se reconstruye con líneas y arcos para animarlo igual en las dos apps.
- **Fuera de alcance:** la landing y el portal web; el iPad; un splash que espere a que carguen los proyectos.

## Capabilities

### New Capabilities

- `app-splash`: la pantalla de arranque de las apps de Lumik: qué muestra, la secuencia de la animación del logo, cuánto dura, cuándo aparece y cómo responde a "reducir movimiento". Cubre el desktop y la tablet Android.

### Modified Capabilities

(ninguna)

## Impact

- **`packages/ui`:**
  - la geometría del logo partida en piezas animables (tapa, contratapa y tres páginas) y los tiempos de la animación, como datos que comparten el desktop y la tablet;
  - el componente del splash para el desktop (SVG).
- **`apps/desktop`:**
  - el splash montado encima de la app al arrancar;
  - el fondo oscuro en `index.html` y en la ventana (`tauri.conf.json`).
- **`apps/tablet`:**
  - splash animado en React Native con `react-native-svg` y `react-native-reanimated`, ya instalados;
  - nueva dependencia `expo-splash-screen` para configurar el splash del sistema y ocultarlo cuando la animación ya está en pantalla;
  - una imagen nueva del libro cerrado en `assets/`.
- **Sin cambios** en el núcleo de Rust, en las bases de datos ni en el comportamiento de la app después del splash.
