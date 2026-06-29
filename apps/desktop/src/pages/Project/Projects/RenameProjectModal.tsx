import { useEffect, useRef, useState, type ChangeEvent, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal, Input, Button } from '@lumik/ui';

interface RenameProjectModalProps {
  open: boolean;
  currentName: string;
  onClose: () => void;
  onSubmit: (newName: string) => void;
  loading?: boolean;
  error?: string | null;
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

export function RenameProjectModal({
  open,
  currentName,
  onClose,
  onSubmit,
  loading,
  error,
}: RenameProjectModalProps) {
  const { t } = useTranslation();
  const [name, setName] = useState(currentName);
  const [fieldError, setFieldError] = useState<string | null>(null);
  const prevOpen = useRef(open);

  // Seed the field with the current name each time the modal opens.
  useEffect(() => {
    if (open && !prevOpen.current) {
      setName(currentName);
      setFieldError(null);
    }
    prevOpen.current = open;
  }, [open, currentName]);

  const handleSubmit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (loading) return;
    const trimmed = name.trim();
    if (!trimmed) {
      setFieldError(t('project.rename.validation.nameRequired'));
      return;
    }
    if (trimmed === currentName) {
      onClose();
      return;
    }
    onSubmit(trimmed);
  };

  const handleClose = () => {
    if (loading) return;
    onClose();
  };

  return (
    <Modal
      title={t('project.rename.title')}
      open={open}
      onClose={handleClose}
      closable
      style={{ width: '460px' }}
    >
      <form onSubmit={handleSubmit} style={formStyles}>
        {error && <div style={errorStyles}>{error}</div>}

        <Input
          label={t('project.rename.name')}
          placeholder={t('project.create.namePlaceholder')}
          value={name}
          onChange={(e: ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
          error={fieldError ?? undefined}
          fullWidth
          autoFocus
          disabled={loading}
        />

        <div style={actionsStyles}>
          <Button variant="ghost" type="button" onClick={handleClose} disabled={loading}>
            {t('project.create.buttons.cancel')}
          </Button>
          <Button variant="primary" type="submit" disabled={loading || !name.trim()}>
            {loading ? t('common.loading') : t('common.save')}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
