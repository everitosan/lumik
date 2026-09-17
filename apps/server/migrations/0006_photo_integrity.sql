-- Integridad del backup: hash y tamaño de los bytes efectivamente subidos a la
-- nube (tras recompresión si aplica). rclone verifica el hash origen↔destino al
-- copiar (copyto), así que un backup 'uploaded' con estos valores es un registro
-- auditable de que lo que llegó al remoto coincide con lo que calculamos.
ALTER TABLE photo ADD COLUMN IF NOT EXISTS backup_sha256 TEXT;
ALTER TABLE photo ADD COLUMN IF NOT EXISTS backup_size   BIGINT;
