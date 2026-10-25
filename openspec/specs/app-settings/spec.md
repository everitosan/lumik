# app-settings Specification

## Purpose

Reúne la configuración propia de cada instalación de Lumik: quién es el fotógrafo y con qué datos firma sus fotos, cómo se comporta el import, qué atajos usa y en qué idioma ve la app.

## Requirements

### Requirement: Fotógrafo por defecto
Al arrancar por primera vez, la app SHALL crear automáticamente un fotógrafo activo a partir del usuario del sistema operativo: alias con los primeros 10 caracteres del nombre de usuario y email `{usuario}@local`. Si ya existe, SHALL reutilizarlo. Cada proyecto creado SHALL atribuirse a ese fotógrafo.

#### Scenario: Primer arranque
- **WHEN** el usuario `everardo` abre Lumik por primera vez
- **THEN** existe un fotógrafo activo con alias `everardo` y email `everardo@local`

#### Scenario: Arranques siguientes
- **WHEN** la app vuelve a arrancar
- **THEN** se usa el mismo fotógrafo y no se crea otro

### Requirement: Metadatos del fotógrafo
El usuario SHALL poder editar los datos que se escriben en las fotos al importar: autor, copyright y URL de contacto. Estos datos SHALL guardarse en la base global de la instalación, no en los discos, y SHALL ser los valores iniciales de cada importación, que puede cambiarlos solo para esa importación sin modificarlos.

#### Scenario: Editar copyright
- **WHEN** el usuario cambia el copyright a "© 2026 Estudio Lumik"
- **THEN** los imports siguientes muestran y escriben ese copyright en las fotos, salvo que se cambie en el paso de ajustes de una importación

#### Scenario: Cambio en una importación
- **WHEN** el usuario cambia el autor en el paso de ajustes de una importación
- **THEN** la página de Ajustes sigue mostrando el autor anterior

### Requirement: Ajustes de importación y sidecars
La app SHALL ofrecer tres ajustes, activos por defecto: embeber los metadatos del fotógrafo al importar, renombrar los archivos por fecha de captura al importar (con un ejemplo del nombre resultante) y generar sidecars de etiquetas para iPad y Mac. Además SHALL ofrecer el valor de fábrica de "Al aprobar, entregar directamente" para proyectos nuevos, que en una instalación nueva es *nada*. Los ajustes SHALL persistir entre reinicios.

#### Scenario: Instalación nueva
- **WHEN** se consultan los ajustes en una instalación nueva
- **THEN** los tres ajustes están activos y la entrega al aprobar de proyectos nuevos es *nada*

#### Scenario: Desactivar renombrado
- **WHEN** el usuario desactiva el renombrado y reinicia la app
- **THEN** el ajuste sigue desactivado y los imports conservan los nombres originales

#### Scenario: Cambiar el valor de fábrica de la entrega
- **WHEN** el usuario elige *todo JPG de cámara* como valor de fábrica y reinicia la app
- **THEN** el ajuste conserva *todo JPG de cámara*, los proyectos que cree desde entonces lo toman, y los existentes conservan su propio valor

### Requirement: Atajos de teclado configurables
La app SHALL tener atajos para el visor (cerrar, anterior, siguiente, zoom, ajustar, rotar, aprobar, estrellas 0–5, añadir tag) y para las vistas de proyectos (nuevo proyecto, buscar, solo aprobadas, volver, importar). El usuario SHALL poder reasignar cada uno pulsando la nueva tecla, opcionalmente con el modificador de la plataforma: `Cmd` en macOS y `Ctrl` en las demás. Un atajo con modificador SHALL valer en todas las plataformas y mostrarse en Ajustes con el símbolo de la plataforma (`⌘` en macOS, `Ctrl+` en las demás). Las teclas modificadoras sueltas y una tecla vacía SHALL rechazarse.

#### Scenario: Reasignar aprobar
- **WHEN** el usuario reasigna "aprobar" a la tecla `p`
- **THEN** en el visor `p` alterna la aprobación y la barra espaciadora deja de hacerlo

#### Scenario: Tecla modificadora sola
- **WHEN** al reasignar el usuario solo pulsa `Shift`
- **THEN** no se guarda ningún cambio

#### Scenario: Atajo con modificador inesperado
- **WHEN** un atajo es la tecla `f` y el usuario pulsa `Alt+f`
- **THEN** el atajo no se dispara

#### Scenario: Modificador en macOS
- **WHEN** en macOS el atajo "solo aprobadas" tiene su valor por defecto y el usuario pulsa `Cmd+c` en la vista de proyecto
- **THEN** la vista alterna a solo aprobadas, `Ctrl+c` no la alterna, y Ajustes muestra el atajo como `⌘C`

#### Scenario: Reasignar con modificador
- **WHEN** en Linux el usuario reasigna "importar" pulsando `Ctrl+i`
- **THEN** se guarda con el modificador, `i` sola deja de importar y `Ctrl+i` importa

### Requirement: Idioma de la interfaz
La interfaz SHALL estar disponible en español (por defecto) e inglés. Cambiar de idioma SHALL aplicarse al momento, sin reiniciar, y SHALL recordarse en esa instalación.

#### Scenario: Cambiar a inglés
- **WHEN** el usuario elige inglés en Ajustes
- **THEN** los textos cambian al momento y la app sigue en inglés en el siguiente arranque

### Requirement: Organización de Ajustes
La página de Ajustes SHALL organizarse en las pestañas Importación, Atajos, Apariencia y Respaldo.

#### Scenario: Abrir Ajustes
- **WHEN** el usuario abre Ajustes
- **THEN** ve las cuatro pestañas, con Importación seleccionada
