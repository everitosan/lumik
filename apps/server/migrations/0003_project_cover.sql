-- Thumbnail de portada del proyecto (JPEG), para el dashboard web.
-- La desktop lo sube al registrar el proyecto o al cambiar la portada.
ALTER TABLE project ADD COLUMN IF NOT EXISTS cover BYTEA;
