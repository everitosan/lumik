# Lumik Roadmap

## Estado actual — MVP (v0.1.0-beta)

Funcional en Linux, Windows y Android. Las features core del flujo de culling están implementadas.

| Feature | Estado |
|---------|--------|
| Import RAW/JPEG a disco externo | ✅ Linux · Windows · Android |
| Grid de fotos agrupadas por fecha | ✅ Linux · Windows · Android |
| Visor canvas (zoom / pan / rotate) | ✅ Linux · Windows · Android |
| Histograma RGB en tiempo real | ✅ Linux · Windows · Android |
| Rating: estrellas, etiquetas de color | ✅ Linux · Windows · Android |
| Tags personalizados | ✅ Linux · Windows · Android |
| Culling: mover a `_culled/` y revertir | ✅ Linux · Windows · Android |
| Videos organizados en `_video/` | ✅ Linux · Windows · Android |
| Detección de discos por UUID | ✅ Linux · Windows · Android |
| Metadatos XMP del fotógrafo en import | ✅ Linux · Windows (embed en DNG) · Android (sidecar .xmp) |
| Keybindings configurables | ✅ Linux · Windows · Android (teclado BT/USB-OTG) |
| Gestos touch (pinch, swipe) | ✅ Android |
| Thumbnails y previews de JPEG | ✅ Linux · Windows · Android |
| Metadata EXIF de JPEG en BD | ✅ Linux · Windows · Android |

---

## En progreso

### Android — pendientes antes de distribución

La app funciona en tablet física (Huawei MatePad DBR-W09) pero tiene issues conocidos:

- **Inconsistencia thumbnail ↔ preview para JPEGs con Orientation ≠ 1** — el thumbnail aplica rotación física al generarse; el preview sirve el JPEG original → posible doble rotación visual en el visor dependiendo del comportamiento del WebView.
- **Nombre real del dispositivo** — actualmente heurística por UUID (`XXXX-XXXX` → "SD Card"). Requiere plugin Kotlin para `StorageManager.getDescription()`.
- **Distribución en Play Store** — requiere generar keystore de firma, configurar cuenta de servicio en Google Play Console y un GitHub Action de release con AAB (`--target aarch64`).

---

## Próximos pasos

### Search by Tag
Permitir buscar y filtrar fotos por tag dentro de un proyecto. El objetivo es que el
fotógrafo pueda marcar fotos con tags semánticos durante el culling ("perfil", "grupal",
"detalle") y luego filtrar la sesión por ellos para entregarlos por separado o editarlos
en lote.

**Alcance:**
- Barra de búsqueda/filtro en la vista de proyecto
- Filtrado por uno o más tags simultáneamente
- Combinable con filtros existentes (culled, rating)

---

### Albums
Agrupaciones manuales de fotos que cruzan proyectos y sesiones. A diferencia de los
proyectos (que agrupan por sesión de captura), un álbum es una curaduría: "Las mejores
fotos del año", "Portafolio", "Entrega cliente X".

**Alcance:**
- Crear y nombrar álbumes desde el dashboard
- Agregar fotos a un álbum desde cualquier proyecto
- Vista de álbum con el mismo grid y visor que los proyectos
- Los álbumes viven en la BD global (`~/.local/share/lumik/lumik.db`), no en el disco externo

---

## Backlog (sin fecha)

- Batch operations desde el grid (rating/cull en lote)
- Servidor de respaldo con API offline-first
- Portal web de entrega al cliente
- App iPad para campo (requiere Mac para compilar)
- macOS version
