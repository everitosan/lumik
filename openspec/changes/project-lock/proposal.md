# Proposal

## Why

El proyecto vive en el disco externo y su `project.db` se abre directo desde ahí. Hoy nada impide que el disco se lleve a otra máquina a mitad de una sesión y que dos dispositivos terminen trabajando sobre copias distintas de la misma base. Con la app de tablet en camino (ver `tablet-app-spike`), en la que Android trabaja sobre una copia de la base y la devuelve al disco, esto deja de ser raro: hace falta saber qué dispositivo tiene el proyecto y si el disco está al día antes de tocarlo.

## What Changes

- Al empezar a trabajar en un proyecto, el dispositivo escribe un candado en la carpeta del proyecto con quién lo tiene, desde cuándo y si el disco está al día. Lo suelta al terminar, al expulsar el disco o al cerrar la app.
- Si el proyecto está tomado por otro dispositivo, se abre en solo lectura con un aviso que dice quién lo tiene y desde cuándo, y ofrece "Tomar el control". El aviso es suave si el disco está al día y fuerte si el otro dispositivo dejó cambios sin devolver.
- El dashboard marca los proyectos tomados por otro dispositivo.
- Cada instalación de Lumik recibe una identidad estable y un nombre legible para aparecer en el candado.

## Capabilities

### New Capabilities
- `project-lock`: uso exclusivo de un proyecto entre dispositivos: el candado, el modo de solo lectura, el aviso y tomar el control.

### Modified Capabilities
- `device-detection`: expulsar un dispositivo también suelta los candados de los proyectos que este dispositivo tiene en él.

## Impact

- Desktop (Rust): modelo del candado en `domain`, lectura y escritura atómica en la infraestructura de archivos, adquisición y liberación en los comandos que abren un proyecto o escriben en él, expulsión, identidad de la instalación en la base global.
- Frontend: aviso y diálogo de "Tomar el control", modo de solo lectura en el proyecto, marca en la tarjeta del dashboard, textos `es`/`en`.
- Disco: un archivo `.lumik-lock` en la carpeta de cada proyecto mientras se trabaja en él. Versiones anteriores de Lumik lo ignoran.
- Prerrequisito de la app de tablet: el modo espejado de Android usa el mismo candado y su marca de cambios pendientes.
