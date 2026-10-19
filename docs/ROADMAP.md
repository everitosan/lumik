# Lumik Roadmap

## Estado actual — v0.1.3-beta

Funcional en Linux, Windows y Android. Las features core del flujo de culling están
implementadas y el respaldo en la nube está en beta (server + portal web).

La especificación de referencia de cada capability vive en [`openspec/specs/`](../openspec/specs/).

| Feature | Estado |
|---------|--------|
| Import RAW/JPEG a disco externo | ✅ Linux · Windows · Android |
| Grid de fotos agrupadas por fecha | ✅ Linux · Windows · Android |
| Visor canvas (zoom / pan / rotate) | ✅ Linux · Windows · Android |
| Histograma RGB en tiempo real | ✅ Linux · Windows · Android |
| Rating: estrellas, etiquetas de color | ✅ Linux · Windows · Android |
| Tags personalizados | ✅ Linux · Windows · Android |
| Filtros por aprobadas, estrellas, tags y color | ✅ Linux · Windows · Android |
| Culling: aprobar (mover a `_culled/`) y quitar aprobación | ✅ Linux · Windows · Android |
| Proyectos: renombrar, eliminar, portada | ✅ Linux · Windows · Android |
| Abrir la carpeta del proyecto en el explorador | ✅ Linux · Windows |
| Videos organizados en `_video/` | ✅ Linux · Windows · Android |
| Detección de discos por UUID | ✅ Linux · Windows · Android |
| Metadatos XMP del fotógrafo en import | ✅ Linux · Windows (embed en DNG) · Android (sidecar .xmp) |
| Keybindings configurables | ✅ Linux · Windows · Android (teclado BT/USB-OTG) |
| Gestos touch (pinch, swipe) | ✅ Android |
| Thumbnails y previews de JPEG | ✅ Linux · Windows · Android |
| Metadata EXIF de JPEG en BD | ✅ Linux · Windows · Android |
| Internacionalización (es / en) | ✅ Linux · Windows · Android |
| Respaldo de `_exported` a la nube (Mega, Nextcloud) | ✅ beta — desktop + server + portal web |
| Portal web: dashboard de proyectos y subida desde iPad | ✅ beta |

---

## En progreso

### Android — pendientes antes de distribución

La app funciona en tablet física (Huawei MatePad DBR-W09) pero tiene issues conocidos:

- **Inconsistencia thumbnail ↔ preview para JPEGs con Orientation ≠ 1** — el thumbnail aplica rotación física al generarse; el preview sirve el JPEG original → posible doble rotación visual en el visor dependiendo del comportamiento del WebView.
- **Nombre real del dispositivo** — actualmente heurística por UUID (`XXXX-XXXX` → "SD Card"). Requiere plugin Kotlin para `StorageManager.getDescription()`.
- **Distribución en Play Store** — requiere generar keystore de firma, configurar cuenta de servicio en Google Play Console y un GitHub Action de release con AAB (`--target aarch64`).

---

## Próximos pasos

### Proveedores OAuth (Google Drive, Dropbox, OneDrive)
El server ya los reconoce pero responde 501 hasta registrar las apps OAuth de Lumik en
cada proveedor. Detalle en [backup-plan.md](backup-plan.md) (Fase 3).

### Cuentas de respaldo — endurecimiento
Verificación de email, recuperación de contraseña y revocación de sesiones desde la UI.

### Proyectos archivados
El backend ya tiene `archive_project`, pero no hay UI. Idea inicial: una sección aparte de
"archivadas" para que no aparezcan en el dashboard principal pero sigan consultables.
Por definir.

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

## Arquitectura / Deuda técnica

- ✅ **Refactor del backend Rust a Arquitectura Limpia** — completado (PR #1). Plan y
  checkpoints en [desktop-clean-architecture.md](desktop-clean-architecture.md); quedan
  diferidos `ProgressReporter`, los casos de uso de proyectos/dispositivos y los DTOs del
  borde IPC.
- URL del server de respaldo por defecto en el desktop (`http://localhost:8787`): cambiarla
  antes de distribuir.

---

## Backlog (sin fecha)

- Batch operations desde el grid (rating/aprobación en lote)
- App iPad nativa para campo (requiere Mac para compilar; hoy el iPad usa el portal web)
- macOS version
