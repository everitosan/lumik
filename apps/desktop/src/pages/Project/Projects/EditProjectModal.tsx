import { useEffect, useRef, useState, type ChangeEvent, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal, Input, Button, Checkbox } from '@lumik/ui';
import { DateField } from '../../../components/DateField';
import { editProject, getProject } from '../../../lib/api';
import type { EditProjectProgress, EditProjectResult, ProjectDashboard } from '../../../lib/types';

interface EditProjectModalProps {
  /** Proyecto a editar; `null` cierra el modal. */
  project: ProjectDashboard | null;
  onClose: () => void;
  /** Se llama tras guardar (con o sin fallos de metadatos). */
  onSaved: (result: EditProjectResult) => void;
}

const formStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '20px',
};

const actionsStyles: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'flex-end',
  gap: '12px',
};

const errorStyles: React.CSSProperties = {
  padding: '12px 16px',
  backgroundColor: 'var(--lumik-error-container, #93000a)',
  color: 'var(--lumik-on-error-container, #ffdad6)',
  borderRadius: '8px',
  fontSize: '14px',
};

const optionsStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
  paddingTop: '4px',
  borderTop: '1px solid var(--lumik-outline-variant, #424654)',
};

const warningStyles: React.CSSProperties = {
  margin: '0 0 0 32px',
  fontSize: '12px',
  color: 'var(--lumik-tertiary, #ffb690)',
};

const summaryStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
  padding: '12px 16px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: '8px',
  fontSize: '13px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const failedListStyles: React.CSSProperties = {
  margin: 0,
  paddingLeft: '18px',
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '12px',
  color: 'var(--lumik-error, #ffb4ab)',
};

/** Misma regla que `sanitize_file_name` en Rust (domain/project.rs). */
const sanitizeProjectName = (name: string) => name.replace(/[^\p{L}\p{N}_-]/gu, '_');

/** Fecha de la carpeta: la de sesión o, si no hay, la de creación. */
const initialDate = (p: ProjectDashboard) => p.session_date ?? p.created_at.slice(0, 10);

export function EditProjectModal({ project, onClose, onSaved }: EditProjectModalProps) {
  const { t } = useTranslation();
  const open = project !== null;

  const [name, setName] = useState('');
  const [date, setDate] = useState('');
  const [description, setDescription] = useState('');
  const [originalDescription, setOriginalDescription] = useState('');
  const [renameFiles, setRenameFiles] = useState(false);
  const [rewriteMetadata, setRewriteMetadata] = useState(false);
  const [fieldErrors, setFieldErrors] = useState<{ name?: string; date?: string }>({});
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [progress, setProgress] = useState<EditProjectProgress | null>(null);
  const [result, setResult] = useState<EditProjectResult | null>(null);
  const prevOpen = useRef(false);

  // Precarga los campos cada vez que se abre el modal.
  useEffect(() => {
    if (open && !prevOpen.current && project) {
      setName(project.name);
      setDate(initialDate(project));
      setDescription('');
      setOriginalDescription('');
      setRenameFiles(false);
      setRewriteMetadata(false);
      setFieldErrors({});
      setError(null);
      setProgress(null);
      setResult(null);
      getProject(project.id)
        .then((full) => {
          const desc = full?.description ?? '';
          setDescription(desc);
          setOriginalDescription(desc);
        })
        .catch((err) => console.error('getProject failed:', err));
    }
    prevOpen.current = open;
  }, [open, project]);

  if (!project) return null;

  const hasPhotos = project.photo_count > 0;
  const nameChanged = name.trim() !== project.name;
  const descriptionChanged = description.trim() !== originalDescription.trim();
  const yearChanged = date.slice(0, 4) !== initialDate(project).slice(0, 4);
  const dateChanged = date !== initialDate(project);
  const showRenameFiles = hasPhotos && nameChanged && name.trim() !== '';
  const showRewrite = hasPhotos && (descriptionChanged || yearChanged);
  const metadataValue = description.trim() ? `${description.trim()}@${date.slice(0, 4)}` : null;

  const handleSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (saving) return;

    const errors: { name?: string; date?: string } = {};
    if (!name.trim()) errors.name = t('project.edit.validation.nameRequired');
    if (!date) errors.date = t('project.edit.validation.dateRequired');
    setFieldErrors(errors);
    if (Object.keys(errors).length > 0) return;

    const wantsRename = showRenameFiles && renameFiles;
    const wantsRewrite = showRewrite && rewriteMetadata;
    if (!nameChanged && !dateChanged && !descriptionChanged && !wantsRewrite) {
      onClose();
      return;
    }

    setSaving(true);
    setError(null);
    setProgress(null);
    try {
      const res = await editProject(
        {
          id: project.id,
          name: name.trim(),
          session_date: date,
          description: description.trim() || undefined,
          rename_files: wantsRename,
          rewrite_metadata: wantsRewrite,
        },
        setProgress,
      );
      onSaved(res);
      // Con fallos se queda abierto mostrando el resumen.
      if (res.metadata_failed.length > 0) setResult(res);
      else onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleClose = () => {
    if (saving) return;
    onClose();
  };

  const saveLabel = saving
    ? progress
      ? t('project.edit.progress', { done: progress.done, total: progress.total })
      : t('common.loading')
    : t('common.save');

  return (
    <Modal title={t('project.edit.title')} open={open} onClose={handleClose} closable style={{ width: '520px' }}>
      {result ? (
        <div style={formStyles}>
          <div style={summaryStyles}>
            {result.renamed_files > 0 && <span>{t('project.edit.result.renamed', { count: result.renamed_files })}</span>}
            <span>{t('project.edit.result.metadataUpdated', { count: result.metadata_updated })}</span>
            <span>{t('project.edit.result.metadataFailed', { count: result.metadata_failed.length })}</span>
            <ul style={failedListStyles}>
              {result.metadata_failed.slice(0, 5).map((f) => (
                <li key={f.path}>{f.name}</li>
              ))}
            </ul>
            <span>{t('project.edit.result.retryHint')}</span>
          </div>
          <div style={actionsStyles}>
            <Button variant="primary" type="button" onClick={onClose}>
              {t('project.edit.result.close')}
            </Button>
          </div>
        </div>
      ) : (
        <form onSubmit={handleSubmit} style={formStyles}>
          {error && <div style={errorStyles}>{error}</div>}

          <Input
            label={t('project.edit.name')}
            placeholder={t('project.create.namePlaceholder')}
            value={name}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
            error={fieldErrors.name}
            fullWidth
            autoFocus
            disabled={saving}
          />

          <DateField
            label={t('project.edit.date')}
            value={date}
            onChange={setDate}
            disabled={saving}
            error={fieldErrors.date}
          />

          <Input
            label={t('project.edit.description')}
            variant="textarea"
            placeholder={t('project.create.descriptionPlaceholder')}
            value={description}
            onChange={(e: ChangeEvent<HTMLTextAreaElement>) => setDescription(e.target.value)}
            fullWidth
            disabled={saving}
          />

          {(showRenameFiles || showRewrite) && (
            <div style={optionsStyles}>
              {showRenameFiles && (
                <>
                  <Checkbox
                    label={t('project.edit.renameFiles')}
                    description={t('project.edit.renameFilesHint', { name: sanitizeProjectName(name.trim()) })}
                    checked={renameFiles}
                    onChange={setRenameFiles}
                    disabled={saving}
                  />
                  {renameFiles && <p style={warningStyles}>{t('project.edit.externalWarning')}</p>}
                </>
              )}
              {showRewrite && (
                <Checkbox
                  label={t('project.edit.rewriteMetadata')}
                  description={
                    metadataValue
                      ? t('project.edit.rewriteMetadataHint', { value: metadataValue, count: project.photo_count })
                      : t('project.edit.rewriteMetadataClearHint', { count: project.photo_count })
                  }
                  checked={rewriteMetadata}
                  onChange={setRewriteMetadata}
                  disabled={saving}
                />
              )}
            </div>
          )}

          <div style={actionsStyles}>
            <Button variant="ghost" type="button" onClick={handleClose} disabled={saving}>
              {t('project.create.buttons.cancel')}
            </Button>
            <Button variant="primary" type="submit" disabled={saving || !name.trim() || !date}>
              {saveLabel}
            </Button>
          </div>
        </form>
      )}
    </Modal>
  );
}
