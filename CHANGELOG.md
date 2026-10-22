# Changelog

Todos los cambios notables de Lumik se documentan en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/)
y el proyecto sigue [Versionado Semántico](https://semver.org/lang/es/).

## [0.1.5-beta] — pendiente (en pruebas)

### Añadido
- **Seguimiento de la importación archivo por archivo**: el paso 3 muestra la
  fase (videos, copia y firma, mover, registrar), el archivo en curso con su
  número ("42 de 400"), los bytes de ese archivo y de toda la importación, y una
  barra que avanza según los bytes. Los videos también se siguen: uno grande ya
  no parece colgado. El registro de actividad muestra una línea por archivo y se
  conserva al terminar.
- **Resultado siempre visible**: al terminar, el paso 3 indica si la importación
  fue completa, parcial o falló, con cuántas fotos y videos entraron, qué
  archivos fallaron y por qué (con "Ver todos"), y qué archivos se ignoraron por
  formato.

### Cambiado
- **Importación más rápida**: una sesión de 304 RAW (7.2 GB) con 3 videos,
  firmada y renombrada desde un disco mecánico, pasa de unos 5 minutos a 2 min
  32 s. El origen se lee de corrido, de un archivo a la vez, mientras la firma
  se aplica en paralelo, y las miniaturas se generan con la rotación que ya se
  leyó.
- Las fotos se preparan en una carpeta oculta dentro del proyecto
  (`.import-staging/`), en el mismo disco de destino, en lugar de la carpeta
  temporal del sistema. El espacio que pide el paso 3 es el único que la
  importación necesita, y una carpeta temporal pequeña (en Linux suele estar en
  RAM) ya no limita el tamaño de la sesión.

### Corregido
- Si algo fallaba durante la importación, el asistente volvía al botón
  "Importar" sin decir qué había pasado. Ahora se muestra el motivo y lo que sí
  quedó importado.
- Un solo archivo con problema cancelaba toda la importación. Ahora se informa
  como fallido y se sigue con los demás.
- Importar una foto o un video con el nombre de uno que ya estaba en el
  proyecto lo sobrescribía sin avisar. Ahora el nuevo se guarda con un sufijo
  `-001`, `-002`…; lo mismo con dos archivos del mismo nombre que vienen de
  carpetas distintas.
- Las importaciones interrumpidas dejaban copias completas de las fotos en la
  carpeta temporal del sistema. Ahora no se usa, y lo que quede en el proyecto
  se limpia al iniciar la siguiente importación.
- Lumik dejaba un proceso de exiftool abierto cada vez que se cerraba.

## [0.1.4-beta] — 2026-10-21

### Añadido
- **macOS (beta)**: Lumik se publica para macOS como `.dmg` universal, que corre
  nativo en Mac Intel y Apple Silicon con macOS 12 Monterey o posterior. Trae
  exiftool incluido, así que no hace falta instalar nada más. Detecta los discos
  externos en cuanto se conectan, los expulsa desde la barra lateral y lista los
  de solo lectura (como NTFS) con un aviso. Los atajos con modificador usan Cmd
  (Ajustes los muestra como `⌘C`).

  **Primera apertura**: esta versión no está notarizada por Apple, así que macOS
  la bloquea la primera vez. Copia Lumik a Aplicaciones y, en Terminal, ejecuta:

  ```
  xattr -dr com.apple.quarantine /Applications/Lumik.app
  ```

  O bien ábrela una vez, cierra el aviso y ve a Ajustes del Sistema →
  Privacidad y seguridad → "Abrir igualmente" (en macOS 12:
  Preferencias del Sistema → Seguridad y privacidad → General → "Abrir de todos
  modos"). Las siguientes aperturas no
  muestran el aviso. Hazlo con la copia de Aplicaciones: el permiso dado a la
  app abierta desde el `.dmg` no pasa a la copia. La primera vez que Lumik lea un disco externo, macOS pedirá
  permiso para acceder a volúmenes extraíbles: hay que permitirlo para ver sus
  proyectos.
- **Ajustes de cada importación**: el wizard de importación tiene un paso nuevo,
  entre Origen y Confirmar, para revisar el renombrado y la firma (autor,
  copyright, URL) antes de importar. Parten de la configuración global y lo que
  se cambie ahí solo aplica a esa importación. El resumen de confirmación
  muestra lo elegido.
- **Orden del dashboard configurable**: el botón Ordenar del dashboard permite
  ver los años y los proyectos de cada año de más recientes a más antiguos o al
  revés. La elección se guarda en la configuración de Lumik y se conserva entre
  reinicios.
- **Editar proyecto**: además del nombre se pueden cambiar la fecha de sesión y
  la descripción. Dos opciones extienden el cambio a las fotos ya importadas:
  renombrar los archivos con el nombre nuevo y actualizar la descripción
  embebida (`{descripción}@{año}`). Los archivos de `_exported` no cambian.
- **Archivar proyectos**: el menú de la tarjeta permite archivar y desarchivar un
  proyecto. Los archivados se ocultan del dashboard y el botón "Archivados (N)"
  los muestra atenuados en su año; la elección se conserva al reabrir la app.

### Cambiado
- **Instalador de Windows más ligero**: ya no incluye dnglab, que no se usaba
  desde que la metadata se guarda en la base de datos en lugar de convertir a DNG.
- **Badge de estado del proyecto**: las tarjetas del dashboard (desktop y portal
  web) muestran el estado de flujo con un icono además del texto, y con colores
  que se leen como semáforo de trabajo: importado en amarillo, editado en azul
  y entregado en gris. La etiqueta "Editando" pasa a "Editado" ("Edited" en
  inglés).
- **Iconos sobre Lucide**: los iconos de la interfaz pasan de SVGs copiados de
  Feather a la biblioteca Lucide (su fork mantenido), con el mismo estilo y
  trazos uniformes. Los componentes y nombres de iconos no cambian.
- **Barra de filtros del dashboard**: "Archivados" y "Ordenar" pasan de la
  cabecera a una barra inferior alineada a la derecha, con el mismo diseño que
  el pie del detalle de proyecto. La cabecera queda con el título y los
  contadores, el buscador y "Nuevo proyecto".

### Corregido
- Al reasignar un atajo en Ajustes pulsando Ctrl con una tecla, se guardaba solo
  la tecla y se perdía el modificador.
- No se podía impedir una importación sin espacio suficiente en el disco del
  proyecto: ahora el botón para iniciarla se desactiva y se avisa del espacio
  que falta.
- El icono de importar mostraba una flecha de *subir*; ahora muestra una flecha
  que entra en una bandeja (botones de importar y badge "Importado").

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

[0.1.4-beta]: https://github.com/everitosan/lumik/releases/tag/v0.1.4-beta
[0.1.3-beta]: https://github.com/everitosan/lumik/releases/tag/v0.1.3-beta
[0.1.2-beta]: https://github.com/everitosan/lumik/releases/tag/v0.1.2-beta
