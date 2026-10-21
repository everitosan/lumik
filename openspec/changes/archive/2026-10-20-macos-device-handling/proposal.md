# Proposal

## Why

En macOS, Lumik detecta los discos externos al escanear, pero no se entera cuando se conectan o desconectan: no hay listener nativo y depende del sondeo de respaldo de 60 s, así que un disco recién conectado parece "no reconocido". Expulsar tampoco funciona: la implementación de macOS no desmonta nada y el disco reaparece en el siguiente escaneo. Además, el mismo disco tiene un UUID distinto en macOS y en Linux (macOS sintetiza un UUID v3 a partir del serial del volumen), y cada foto guarda ese UUID en `photo.device_uuid`, lo que ata los datos del proyecto a un disco y a una plataforma concretos y rompe la portabilidad de mover un proyecto a otro disco.

## What Changes

- **Detección inmediata en macOS**: un listener nativo de conexión y desconexión de volúmenes avisa a la interfaz en cuanto el sistema monta o desmonta un disco. Se mantiene el sondeo de respaldo de 60 s, igual que en Linux y Windows.
- **Expulsión real en macOS**: expulsar desmonta todos los volúmenes del disco físico y lo deja listo para retirarse. Si el sistema lo rechaza (por ejemplo, porque otro proceso tiene archivos abiertos), se informa el error con el proceso que lo bloquea y el disco sigue listado.
- **Identidad del disco por máquina**: el UUID del dispositivo pasa a ser explícitamente un identificador local de cada máquina, que solo sirve para localizar y expulsar el disco. No se unifica entre plataformas: ni archivo marcador en el disco ni conversión de formatos de UUID.
- **Identidad portable del proyecto**: un proyecto se reconoce por su `project.id` en cualquier disco donde esté. Mover la carpeta de un proyecto a otro disco, dentro de `lumik/`, basta para que aparezca desde ahí.
- **`photo.device_uuid` obsoleto**: las fotos nuevas dejan de guardar el UUID del disco (la columna se escribe vacía) y ninguna lógica lo lee. La columna se conserva para no migrar los `project.db` existentes en los discos.
- **Limitación documentada**: si el mismo proyecto (mismo `project.id`) está en dos discos conectados a la vez, solo uno se lista. Se documenta en la spec sin cambiar el comportamiento.

### Fuera de alcance

- Unificar el UUID del mismo disco entre macOS, Linux, Windows y Android.
- Resolver o advertir los proyectos duplicados entre discos.
- Eliminar la columna `photo.device_uuid` del esquema o limpiar los valores existentes.
- Buscar `exiftool` fuera del `PATH` cuando la `.app` empaquetada se abre desde Finder.

## Capabilities

### New Capabilities

_Ninguna._

### Modified Capabilities

- `device-detection`: el refresco ante conexión y desconexión incluye macOS; la expulsión segura se implementa en macOS, con escenarios para disco con varias particiones y proceso que bloquea; la identidad por UUID se redefine como identificador local de cada máquina.
- `project-management`: nuevos requisitos de identidad portable del proyecto por `project.id`, sin importar el disco en el que esté (escenario de mover un proyecto a otro disco), y de la limitación de proyectos duplicados en dos discos conectados. El requisito de descubrimiento existente no cambia.

## Impact

- `apps/desktop/src-tauri/src/device_watch.rs`: nueva rama macOS con DiskArbitration (hilo con CFRunLoop que emite `devices-changed`).
- `apps/desktop/src-tauri/src/infrastructure/devices.rs`: `os_eject` en macOS desmonta el disco completo y lo expulsa con DiskArbitration, reportando el error del sistema.
- `apps/desktop/src-tauri/Cargo.toml`: dependencias solo para macOS para el binding de DiskArbitration y CoreFoundation.
- `apps/desktop/src-tauri/src/application/use_cases/import_photos.rs`, `db/queries.rs`, `db/project_schema.sql`: las fotos nuevas guardan `device_uuid` vacío, con un comentario de obsolescencia en el esquema.
- `docs/useCases/flujo-importar-imagenes.md` y `apps/desktop/CLAUDE.md`: reflejar el cambio de `device_uuid` y el soporte de macOS.
- Sin cambios en `apps/server`, `apps/web`, el catálogo ni `packages/ts-types`: ninguno usa el UUID del dispositivo.
