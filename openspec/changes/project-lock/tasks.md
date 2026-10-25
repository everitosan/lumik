# Tasks

## 1. Núcleo

- [x] 1.1 Modelo `ProjectLock` en `domain` (instalación, nombre, inicio, generación, pendiente) con su lectura y escritura en JSON, y decisión de estado (libre, propio, ajeno al día, ajeno con pendientes). Verificar con tests de dominio de cada estado.
- [x] 1.2 Identidad de la instalación en la base global (UUID y nombre editable, por defecto el hostname). Verificar con un test de base global y en Ajustes.
- [x] 1.3 Escritura atómica del candado (temporal y renombrado) en la infraestructura de archivos, y columna `generation` en `project.db` que sube con cada escritura confirmada. Verificar con tests de infraestructura y de migración.
- [x] 1.4 Sesión de trabajo por proyecto: tomar el candado al abrir o al empezar una operación de escritura, soltarlo al terminar la última, y error `ProjectLocked` en los comandos que escriben. Verificar con tests de casos de uso: importación en curso mantiene el candado, aprobar con candado ajeno falla sin mover archivos.
- [x] 1.5 Tomar el control y detectar un candado perdido antes de escribir. Verificar con tests: tras tomar el control en A, B no escribe.
- [x] 1.6 Expulsar suelta los candados propios del disco antes de cerrar las bases. Verificar con un test de la expulsión y el escenario de la spec.

## 2. Interfaz

- [ ] 2.1 *(Código hecho: `open_project_session`/`close_project_session`/`take_project_control`, `ProjectLockBanner` en la vista, solo lectura en grid y visor (sin papelera, importar, respaldar, rotar, calificar ni portada) y ajustes de vista solo en memoria; `api.ts` avisa con `lumik:project-locked` si una escritura choca con el candado y la vista se vuelve a revisar; `tsc` y build limpios; falta la comprobación en `dev`.)* Aviso de proyecto tomado (suave o fuerte según `pending`) con "Tomar el control", y modo de solo lectura en el grid y el visor. Verificar en `dev` con un candado ajeno creado a mano.
- [ ] 2.2 *(Código hecho: `ProjectCard.lockedBy` con historia en el Storybook, `locked_by` en el dashboard y `TakeControlDialog` antes de editar, archivar o eliminar; falta la comprobación en `dev`.)* Marca "En {dispositivo}" en la tarjeta del dashboard y confirmación de tomar el control en editar, archivar y eliminar. Verificar en `dev`.
- [ ] 2.3 *(Código hecho: "Nombre de este equipo" en Ajustes → Apariencia con `get_install_name`/`set_install_name`, textos `es`/`en`; falta la comprobación en `dev`.)* Nombre de la instalación en Ajustes y textos `es`/`en`. Verificar en `dev` en los dos idiomas.

## 3. Documentación y verificación

- [x] 3.1 Diagrama del candado en `docs/useCases/` y comandos en `apps/desktop/CLAUDE.md`. Verificar que el Mermaid parsea.
- [ ] 3.2 Prueba manual con dos equipos (Linux y Windows): abrir en uno, desconectar el disco sin cerrar, abrir en el otro (aviso suave), tomar el control, volver al primero (lo ve tomado). Correr `cargo test`, `pnpm --filter @lumik/desktop build` y `openspec validate project-lock --strict`.
