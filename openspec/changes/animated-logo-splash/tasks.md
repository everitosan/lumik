# Tasks

## 1. Logo partido (datos compartidos)

- [x] 1.1 Crear `packages/ui/src/brand/logo.ts` (sin dependencias), exportado como `@lumik/ui/brand` en `package.json`. Debe tener el viewBox, el eje de giro, los trazos de tapa y contratapa (el cuerpo actual partido, con solape bajo la tapa), los tres trazos de página con sus degradados y opacidades, y la línea de tiempo de D3. Verificar renderizando las piezas juntas y el `apps/landing/public/icon.svg` actual a 1024 px con `inkscape`, y comparándolos con `magick compare -metric AE`: diferencia de 0 píxeles, o solo antialiasing de borde.
- [x] 1.2 Dibujar `Logo.tsx` con los datos de `@lumik/ui/brand`. Verificar con `pnpm --filter @lumik/desktop build` y viendo el logo en la barra lateral y en Acerca de sin cambios.
- [x] 1.3 Definir la geometría del libro cerrado: el ángulo de partida de la tapa y de cada página, y el recorte por el borde de la contratapa (D2), para que cerrado se vea un bloque sin páginas asomadas. Verificar con un render del cuadro inicial (libro cerrado) y del final (logo), revisados contra la spec "Secuencia del libro que se abre".
- [x] 1.4 Reconstruir el nombre "lumik" de `logotipo.png` como trazos en `@lumik/ui/brand` (D2b): las letras, el punto, los colores, su lugar bajo el libro y su parte de la línea de tiempo (D3). Verificar renderizando el nombre encima de `logotipo.png`, a su escala, y comparando a ojo la forma de cada letra.

## 2. Desktop

- [x] 2.1 `LogoSplash` en `packages/ui` (D4): un SVG a pantalla completa sobre `#131313`, con resplandor, contratapa, páginas y tapa animados en cada cuadro con el atributo `transform`, siguiendo la línea de tiempo. Con `prefers-reduced-motion: reduce`, el logo quieto con el mismo desvanecimiento. Al terminar se desmonta y no captura clics. Agregar una historia en `apps/uikit`. Verificar en Storybook la animación completa y la versión con movimiento reducido (emulada en las DevTools).
- [x] 2.2 Montar `LogoSplash` junto a `App` en `apps/desktop/src/main.tsx`, poner fondo `#131313` en `index.html` y `backgroundColor: "#131313"` en la ventana de `tauri.conf.json`. Verificar con `pnpm --filter @lumik/desktop build` y abriendo la app compilada en Linux: sin destello blanco, el libro se abre y sale el nombre en ~2 s, la app queda usable debajo y se puede hacer clic al terminar.
- [x] 2.3 Documentar el splash en `apps/desktop/CLAUDE.md`: estructura y arranque. Verificar leyendo la sección.
- [x] 2.4 Agregar el nombre al `LogoSplash` del desktop: cada letra sale de su línea base y el punto aparece al final (D2b, D3); con movimiento reducido, quieto. Verificar en Storybook con capturas, como en 2.1.

## 3. Tablet

- [x] 3.1 Agregar `@lumik/ui` (workspace) y `expo-splash-screen` (`pnpm exec expo install`) a `apps/tablet`. Generar `assets/splash-book.png` (el libro cerrado sobre transparente, desde la geometría de 1.3) y configurar el plugin en `app.config.ts` con `backgroundColor: '#131313'` y el ancho de la imagen. Quitar `splash-icon.png`. Verificar con `expo prebuild` y un APK en el emulador: el splash del sistema muestra el libro cerrado sobre `#131313`, sin el ícono de Expo.
- [x] 3.2 `src/LogoSplash.tsx` (D5): las piezas con `react-native-svg` y la animación con Reanimated, según la línea de tiempo de `@lumik/ui/brand`. Incluye el nombre (1.4). Con `useReducedMotion()`, el logo y el nombre quietos con el desvanecimiento. En `App.tsx`, `preventAutoHideAsync()` al cargar el módulo y `hideAsync()` en el `onLayout` del splash, encima de `AppScreen`. Verificar con `pnpm --filter @lumik/tablet typecheck`, y en el emulador y en la DBR-W09 con un APK de release: el paso del splash del sistema a la animación no salta, el libro se abre y deja ver la app, y volver desde segundo plano no repite el splash.
- [x] 3.3 Probar con "Quitar animaciones" activado en Android: se ven el logo y el nombre quietos y luego la app. Verificar en la DBR-W09.
- [x] 3.4 Documentar en `apps/tablet/CLAUDE.md`: el splash en la estructura, y que la tablet solo importa `@lumik/ui/brand`, nunca la raíz de `@lumik/ui`. Verificar leyendo el archivo.

## 4. Integración

- [x] 4.1 Comparar el desktop y la tablet lado a lado (video o capturas del cuadro inicial, del medio y del final): la misma secuencia, los mismos tiempos y el logo final idéntico. Correr `pnpm --filter @lumik/desktop build`, `pnpm --filter @lumik/tablet typecheck` y `openspec validate animated-logo-splash --strict`.

## Workflow follow-up

- Archivar el change cuando las pruebas en la tablet y en el desktop estén hechas.
