# Tasks

## 1. Componentes en @lumik/ui

- [x] 1.1 Crear `FilterBar` (con `start`, `end` y `FilterBarDivider`) en `packages/ui/src/components/FilterBar/` con los estilos del pie del detalle más `flexWrap: 'wrap'`, exportarlo desde `packages/ui/src/index.ts` y añadir su story en `apps/uikit`; verificar con `pnpm --filter @lumik/ui build` y el build del uikit
- [x] 1.2 Crear `FilterToggle` (`label: ReactNode`, `checked`, `onChange`; `button role="switch" aria-checked`; círculo de 16px, borde de 2px y color terciario cuando está activo) en `packages/ui/src/components/FilterToggle/`, exportarlo y añadir su story (apagado, encendido, con contador); verificar con los builds de `@lumik/ui` y del uikit

## 2. Detalle de proyecto

- [x] 2.1 Tomar una captura de referencia del pie del detalle de proyecto en la app (apagado y con "Mostrar solo aprobadas" activo) — omitida por decisión del usuario
- [x] 2.2 Migrar `ProjectDetailFooter.tsx` a `FilterBar`, `FilterBarDivider` y `FilterToggle` sin cambiar textos ni filtros; verificar con `pnpm exec tsc --noEmit` en `apps/desktop` y comparando en la app con la captura de 2.1

## 3. Dashboard

- [x] 3.1 Reestructurar `Projects.tsx`: contenedor exterior sin padding, zona con padding para la cabecera (título con contadores, buscador, "Nuevo proyecto") y la cuadrícula, y `FilterBar` al final con el botón "Archivados (N)" (solo si hay archivados), un `FilterBarDivider` y `SortMenu` en `end`; verificar con `jq empty` y `tsc`
- [x] 3.2 Hacer que el panel de `SortMenu` abra hacia arriba (`bottom: '100%'`); verificar con `tsc` y en la app

## 4. Verificación

- [x] 4.1 Verificar en la app el dashboard en ventana ancha y estrecha (contadores en la cabecera; "Archivados (N)" y "Ordenar" a la derecha de la barra sin apretarse), el botón de archivados (aparece solo si hay archivados, persiste, filtra y actualiza los contadores) y "Ordenar" abriendo hacia arriba; y que el pie del detalle se ve como antes
- [x] 4.2 Añadir la entrada en `[Sin publicar]` de `CHANGELOG.md`; verificar que sigue el formato de las entradas existentes
