# Spec Delta

## MODIFIED Requirements

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
