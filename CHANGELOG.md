# Changelog

Todos los cambios notables de Lumik se documentan en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/)
y el proyecto sigue [Versionado Semántico](https://semver.org/lang/es/).

## [0.1.3-beta] — 2026-09-20

### Añadido
- **Respaldo en la nube (beta)**: nueva capacidad para respaldar la carpeta
  `_exported` de cada proyecto a servicios de nube de terceros, a través de un
  server propio de Lumik (axum + Postgres + rclone) y un portal web.
  - **Cuentas de respaldo**: registro/login desde el desktop contra el server
    (contraseñas con argon2id, sesión con JWT + refresh con rotación/revocación,
    aislamiento por fotógrafo).
  - **Proveedores de nube**: Mega y Nextcloud por credenciales (cifradas en
    reposo). Google Drive, Dropbox y OneDrive por OAuth quedan preparados,
    pendientes de registrar las apps de cada proveedor.
  - **Respaldo desde el desktop**: al abrir un proyecto se sincroniza su catálogo
    en el server; el respaldo de `_exported` usa un **modal** (elegir proveedores,
    calidad de compresión, subida en paralelo), corre **en segundo plano** con
    progreso persistente y una **cola offline-first** que reanuda al arrancar.
  - **Integridad y organización**: verificación por hash (SHA-256 + tamaño) de lo
    que se sube; renombrar un proyecto **mueve** su carpeta en la nube para que no
    se parta.
  - **Portal web / iPad**: dashboard con los proyectos del catálogo (portadas,
    agrupación por año, estado de respaldo y enlace a la carpeta en la nube) y
    subida de fotos desde el navegador.
- **Indicadores de proveedor** (ProviderIcon) en las tarjetas de proyecto.

### Corregido
- El modal de respaldo ya no congela la interfaz durante la subida y comprime más
  rápido; usa el componente `Select` base de `@lumik/ui`.
- Se evitan carpetas `_exported` duplicadas en Mega y peticiones redundantes a
  `GET /auth/me` al abrir cada proyecto.

## [0.1.2-beta] — 2026-06-29

### Añadido
- **Internacionalización (i18n)**: soporte completo en la app de escritorio y
  componentes UI, con español como idioma por defecto e inglés como alternativa,
  y cambio de idioma dinámico.
- **Pestañas en la página de Settings** y nuevo componente reutilizable de Tabs.
- **Menú de opciones en ProjectCard** para acciones sobre cada proyecto.
- **Filtros en la vista de detalle de proyecto.**
- **Sugerencia de tags y atajo de teclado** para etiquetar más rápido.
- **Expulsión y montaje de dispositivos** con notificaciones del sistema operativo.
- **Soporte para imágenes TIFF (xsane).**
- **Reconciliación de fecha** en imágenes antiguas cuando la fecha es nula.
- Nuevo **componente Loader**.
- Ícono de la aplicación.

[0.1.3-beta]: https://github.com/everitosan/lumik/releases/tag/v0.1.3-beta
[0.1.2-beta]: https://github.com/everitosan/lumik/releases/tag/v0.1.2-beta
