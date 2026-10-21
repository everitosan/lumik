# Proposal

## Why

La 0.1.4-beta sale con un `.dmg` universal para macOS, pero la landing sigue diciendo que macOS "llega pronto". La pestaña de descargas de macOS está deshabilitada y una Mac que entra a `/downloads` cae en la pestaña de Linux. El roadmap tampoco refleja lo que ya hay: le faltan proyectos editables y archivables, la papelera y los ajustes por importación, y anuncia la versión de macOS como "próximamente".

## What Changes

- **Descargas de macOS**: la pestaña de macOS se habilita y lista los `.dmg` de los releases de GitHub, igual que Linux y Windows. Una Mac que entra a la página abre esa pestaña. Hay un solo `.dmg` universal, sin selector de arquitectura.
- **Primera apertura en macOS**: la pestaña de macOS muestra siempre un bloque con el requisito mínimo (macOS 12 Monterey) y cómo abrir la app sin notarizar: quitar la cuarentena con `xattr` (con botón para copiar) o "Abrir igualmente" en Privacidad y seguridad.
- **Primera apertura en Windows**: el instalador tampoco va firmado. La pestaña de Windows muestra siempre una nota breve con los pasos para continuar cuando SmartScreen avisa: "Más información" y luego "Ejecutar de todas formas".
- **Releases sin paquetes para un SO**: cada pestaña oculta los releases que no tienen paquetes de ese sistema y muestra el estado vacío solo si no queda ninguno. Así la pestaña de macOS no enseña la 0.1.2 y la 0.1.1 vacías, y la landing puede publicarse antes o después del tag `v0.1.4-beta`.
- **Plataformas**: macOS pasa de "Próximamente" a "Disponible", igual que Linux y Windows. Que Lumik está en beta es un dato del producto, no de una plataforma: el subtítulo nombra las tres plataformas y dice una sola vez que Lumik está en beta. Ninguna plataforma ni pestaña de descargas lleva un badge "beta" propio. Android y el iPad no aparecen como plataformas mientras no haya algo descargable.
- **Roadmap** (en y es):
  - *Disponible ahora*: importar RAW, TIFF y JPEG al disco externo; rating, etiquetas y culling; filtros dentro del proyecto; visor con histograma RGB; atajos configurables; XMP del fotógrafo al importar; videos en la carpeta del proyecto; montaje y expulsión de discos, incluidos los de solo lectura; editar y archivar proyectos; papelera; ajustes por importación; interfaz en español e inglés; Linux, Windows y macOS.
  - *Próximamente*: búsqueda global por etiqueta (entre proyectos), álbumes, macOS firmado y notarizado.
  - *Más adelante*: respaldo en la nube y portal web para clientes, app iPad para campo, operaciones en lote.
  - Android no se menciona.

### Fuera de alcance

- Cambios al release, a la firma o a la notarización de macOS (ya cubiertos por `desktop-distribution`).
- Probar la 0.1.4 en Windows: se hace con el release y, si hace falta, sale un parche.
- Android, la app iPad nativa y el despliegue del server de respaldo.

## Capabilities

### New Capabilities

- `landing-site`: el sitio público de Lumik: qué plataformas anuncia, cómo la página de descargas presenta los paquetes de los releases de GitHub para cada sistema y qué guía de instalación muestra.

### Modified Capabilities

Ninguna.

## Impact

- `apps/landing/src/components/DownloadsPage.astro`: pestaña y panel de macOS, guías de primera apertura de macOS y Windows, y un script que trata los tres sistemas por igual.
- `apps/landing/src/components/LandingPage.astro`: badge "Disponible" de macOS, subtítulo de plataformas y listas del roadmap.
- `apps/landing/src/i18n/ui.ts`: textos nuevos y cambiados en `en` y `es`; se quitan los `dl.coming.*` que dejan de usarse.
- Sin cambios en `apps/desktop`, en el workflow de release ni en otras apps.
