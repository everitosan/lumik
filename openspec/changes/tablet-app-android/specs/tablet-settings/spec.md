# Spec Delta

## Purpose

Los ajustes de la app de tablet Android: quién es el fotógrafo para la firma de las fotos, cómo se importa por defecto y con qué nombre se identifica la tablet en el candado de los proyectos.

## ADDED Requirements

### Requirement: Datos del fotógrafo
En Ajustes, el fotógrafo SHALL poder editar el autor, el copyright y la URL de contacto que se escriben en las fotos al importar. Los datos SHALL guardarse en la tablet y conservarse al actualizar la app.

#### Scenario: Guardar el autor
- **WHEN** el fotógrafo escribe "Ana López" como autor y vuelve a abrir la app
- **THEN** Ajustes sigue mostrando "Ana López" y los imports siguientes lo usan

### Requirement: Ajustes del import
Ajustes SHALL ofrecer "Firmar las fotos al importar" y "Renombrar por fecha de captura al importar", los dos activos por defecto, y el import SHALL usarlos.

#### Scenario: Sin renombrar
- **WHEN** el fotógrafo desactiva el renombrado e importa `DSCF0001.RAF`
- **THEN** la foto queda en `_media/` como `DSCF0001.RAF`

### Requirement: Entrega en proyectos nuevos
Ajustes SHALL ofrecer "Entrega al aprobar en proyectos nuevos" con las tres opciones del desktop (nada, el JPG de los pares RAW+JPG, todo JPG de cámara), "nada" por defecto. Un proyecto creado en la tablet SHALL tomar ese valor al crearse y después SHALL guardar el suyo: cambiar el ajuste no cambia los proyectos que ya existen.

#### Scenario: Proyecto nuevo con pares
- **WHEN** el fotógrafo elige "El JPG de los pares RAW+JPG" en Ajustes y crea un proyecto
- **THEN** el import de ese proyecto muestra esa entrega, y al aprobar un par su JPG va a `_exported/`

#### Scenario: Los proyectos existentes no cambian
- **WHEN** el fotógrafo cambia el ajuste a "Todo JPG de cámara"
- **THEN** los proyectos que ya existían conservan su entrega

### Requirement: Nombre de la tablet
Ajustes SHALL mostrar y permitir cambiar el nombre con que la tablet aparece en el candado de los proyectos. Por defecto SHALL ser "Tablet {modelo}".

#### Scenario: Cambiar el nombre
- **WHEN** el fotógrafo cambia el nombre a "Tablet de Ana" y abre un proyecto
- **THEN** en el desktop ese proyecto aparece como abierto en "Tablet de Ana"
