-- Carpeta real del backup en la nube (ancla). Se fija en el primer respaldo y se
-- reutiliza aunque el proyecto se renombre; al renombrar, la desktop pide mover la
-- carpeta y se actualiza aquí. Evita carpetas partidas por rename/cambio de fecha.
ALTER TABLE project ADD COLUMN IF NOT EXISTS backup_dir TEXT;
