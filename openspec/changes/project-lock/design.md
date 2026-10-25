# Design

## Context

Cada proyecto guarda su `project.db` (modo WAL) en su carpeta del disco y el desktop lo abre directo. El registro de proyectos abre las bases de los discos montados para listarlas en el dashboard, así que "abrir la base" no equivale a "trabajar en el proyecto". No hay identidad de instalación: la base global solo registra discos. Ver proposal.md para la motivación.

## Goals / Non-Goals

**Goals:**
- Que dos dispositivos no modifiquen el mismo proyecto sin saberlo.
- Que el caso común, un candado huérfano porque el disco se desconectó sin cerrar, se resuelva con un clic.
- Dejar listo el formato del candado que usará el modo espejado de Android.

**Non-Goals:**
- Fusionar cambios de dos dispositivos.
- Bloqueo en tiempo real entre equipos en red: un disco USB está conectado a un solo equipo a la vez.
- El modo espejado en sí (vive en la app de tablet).

## Decisions

- **Archivo `.lumik-lock` en la carpeta del proyecto, JSON.** Viaja con el disco, que es lo que se mueve entre dispositivos, y cualquier plataforma lo lee. Campos: `install_id`, `install_name`, `since` (UTC), `generation` y `pending`. Se escribe en un temporal y se renombra, para que nadie lea un candado a medias. Se descartó guardarlo dentro de `project.db`: en Android la base del disco no se abre directo, así que el candado tiene que leerse sin SQLite.
- **El candado se toma por sesión de trabajo, no al listar.** Se toma al entrar al proyecto o al empezar una operación que escribe en él (importar, respaldar, cambiar la entrega, editar, archivar, eliminar) y se suelta cuando no queda ninguna. Lo lleva un contador por proyecto en el estado de la app. Listar en el dashboard solo lee el candado.
- **`generation` y `pending`.** `generation` es un contador guardado también en la base del proyecto, que sube con cada escritura confirmada. En modo directo (desktop) `pending` siempre es `false`, porque escribe en el disco. El modo espejado lo pone en `true` mientras tiene cambios sin devolver. Así el aviso del otro dispositivo distingue "disco al día" de "cambios sin devolver" sin abrir la base.
- **Solo lectura reforzada en el backend.** Los comandos que escriben comprueban el candado antes de tocar disco o base y devuelven un error tipado `ProjectLocked`. La UI deshabilita las acciones, pero la garantía está en el núcleo.
- **Perder el candado.** Si un dispositivo descubre que su candado fue reemplazado (al volver a mirar o antes de escribir), pasa a solo lectura y no escribe en la base del disco.
- **Identidad de la instalación.** UUID generado al primer arranque y guardado en la base global, con el nombre del equipo (hostname en desktop, nombre del dispositivo en tablet) editable en Ajustes.

## Risks / Trade-offs

- [El candado se queda huérfano si la app muere o el disco se arranca] → El candado propio se retoma sin aviso, y uno ajeno se toma con un clic. En desktop `pending=false`, así que el aviso es suave.
- [Versiones anteriores de Lumik ignoran el candado] → Hoy solo hay un usuario activo; el CHANGELOG pide actualizar todas las instalaciones.
- [Un disco en solo lectura no admite candado] → Esos discos ya no se abren (spec `device-detection`), así que no cambia nada.
- [Reloj distinto entre dispositivos] → `since` es solo informativo; la decisión usa `install_id` y `pending`, no la hora.

## Migration Plan

Sin migración de datos: los proyectos sin `.lumik-lock` se consideran libres. La columna `generation` se agrega con `DEFAULT 0`, de forma idempotente, como las migraciones anteriores.
