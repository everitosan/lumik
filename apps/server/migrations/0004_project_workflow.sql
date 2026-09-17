-- Estado del proyecto (imported/edited/delivered), fijado por la desktop tras
-- respaldar. Alimenta el badge de estado en el dashboard web.
ALTER TABLE project ADD COLUMN IF NOT EXISTS workflow_status TEXT;
