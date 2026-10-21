# Tasks

## 1. Descargas por sistema

- [x] 1.1 En `DownloadsPage.astro`, generar las pestañas y los paneles de `linux`, `windows` y `macos` con IDs por sufijo de SO. Habilitar la pestaña de macOS (sin `disabled` ni "soon") y quitar el panel "coming soon". Verificar en `pnpm --filter @lumik/landing dev` que las tres pestañas cambian de panel.
- [x] 1.2 Reemplazar el script por una configuración por SO (extensiones, texto vacío, hint) y una sola función `render(os)` con un único `fetch`. Verificar que Linux y Windows muestran los mismos paquetes que antes de 0.1.2 y 0.1.1.
- [x] 1.3 Filtrar en cada pestaña los releases sin paquetes de ese SO, poner "latest" al primero de la lista filtrada y mostrar el aviso vacío si no queda ninguno. Quitar el `no-assets` por release. Verificar con los releases actuales que la pestaña de macOS muestra el aviso vacío y la de Windows muestra 0.1.2 como "latest".
- [x] 1.4 Abrir la pestaña del SO detectado, incluida macOS. Verificar con DevTools (user agent de Safari en macOS) que se abre macOS y que el subtítulo dice que se detectó macOS.
- [x] 1.5 Agregar la guía de primera apertura en HTML estático dentro del panel de macOS: requisito de macOS 12, pasos, comando `xattr` en `<code>` y botón de copiar con confirmación temporal. Verificar que el botón copia el comando exacto y que la guía sigue visible si se bloquea `api.github.com` en DevTools.
- [x] 1.6 En `ui.ts` (`en` y `es`), actualizar `dl.hint.macos`, agregar `dl.no.assets.macos` y las claves de la guía, y quitar `dl.coming.*` y las que dejen de usarse. Verificar con `grep` que cada clave `dl.*` usada existe en los dos idiomas y que no quedan claves sin uso.

## 2. Plataformas y roadmap

- [x] 2.1 En `LandingPage.astro`, macOS pasa a badge "Beta" con su variante visual. Agregar `platforms.beta` y actualizar `platforms.sub` en `en` y `es` para nombrar Linux, Windows y macOS. Verificar en `/` y `/es/` que Linux y Windows dicen Disponible, macOS dice Beta y no aparecen Android ni el iPad.
- [x] 2.2 Renombrar las claves del roadmap a nombres descriptivos y pintar cada grupo iterando un arreglo de claves. Contenido según el proposal: Ahora (13 elementos, incluidos editar y archivar proyectos, papelera, ajustes por importación y Linux · Windows · macOS beta); Próximamente (búsqueda global por etiqueta entre proyectos, álbumes, macOS firmado y notarizado); Más adelante (respaldo en la nube y portal web, app iPad para campo, operaciones en lote). Verificar en `/#roadmap` y `/es/#roadmap` que se ven los tres grupos completos y que no se menciona Android.

## 3. Beta como estado del producto y guía de Windows

- [x] 3.1 Quitar el badge "beta" de macOS en la sección de plataformas y en su pestaña de descargas: las tres plataformas dicen "Disponible", `platforms.sub` indica una vez que Lumik está en beta y el roadmap dice "Linux · Windows · macOS". Quitar `platforms.beta` y el CSS de la variante beta. Verificar en `/` y `/es/` que las tres plataformas dicen Disponible y que el subtítulo menciona la beta.
- [x] 3.2 Agregar al panel de Windows una nota estática de SmartScreen ("Más información" → "Ejecutar de todas formas") en `en` y `es`, con la clase `.install-guide` compartida con la guía de macOS. Verificar que aparece solo en la pestaña de Windows y que sigue visible si falla la API de GitHub.

## 4. Verificación integral

- [x] 4.1 Correr `pnpm --filter @lumik/landing build` sin errores y revisar con `pnpm --filter @lumik/landing preview` las cuatro páginas (`/`, `/es/`, `/downloads`, `/es/downloads`) en escritorio y a 375 px de ancho: pestañas, guía de macOS y roadmap sin scroll horizontal.

## Workflow follow-up

- Publicar la landing después del release `v0.1.4-beta` y confirmar que la pestaña de macOS muestra el `.dmg` universal como "latest".
- Archivar el change.
