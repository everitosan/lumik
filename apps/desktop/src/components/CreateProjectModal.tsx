import { useState, useEffect, useMemo, useRef, type ChangeEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal, Input, Button, DriveSelector, type Drive } from '@lumik/ui';
import type { DetectedDevice } from '../lib/types';
import { DateField } from './DateField';

interface CreateProjectModalProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (data: ProjectFormData) => void;
  devices: DetectedDevice[];
  loading?: boolean;
  error?: string | null;
}

export interface ProjectFormData {
  name: string;
  description: string;
  sessionDate: string;
  deviceUuid: string;
}

const formStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '20px',
};

const columnsStyles: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: '1fr 1fr',
  gap: '20px',
  alignItems: 'start',
};

const columnStyles: React.CSSProperties = {
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

const fieldErrorStyles: React.CSSProperties = {
  fontSize: '12px',
  color: 'var(--lumik-error, #ffb4ab)',
  marginTop: '-12px',
};

function devicesToDrives(devices: DetectedDevice[]): Drive[] {
  return devices.map((d) => ({
    id: d.uuid,
    name: d.name,
    uuid: d.uuid,
    totalBytes: d.total_bytes ?? 0,
    usedBytes: (d.total_bytes ?? 0) - (d.available_bytes ?? 0),
    connected: true,
    mountPath: d.mount_point,
  }));
}

export function CreateProjectModal({
  open,
  onClose,
  onSubmit,
  devices,
  loading,
  error,
}: CreateProjectModalProps) {
  const { t } = useTranslation();
  // Read-only drives can't hold new projects (the OS mounted them without write).
  const writableDevices = useMemo(() => devices.filter((d) => !d.read_only), [devices]);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [sessionDate, setSessionDate] = useState('');
  const [deviceUuid, setDeviceUuid] = useState('');
  const [errors, setErrors] = useState<Partial<Record<'name' | 'deviceUuid', string>>>({});

  const prevOpen = useRef(open);
  // Reset form fields when modal opens
  useEffect(() => {
    if (open && !prevOpen.current) {
      setName('');
      setDescription('');
      setSessionDate('');
      setDeviceUuid('');
      setErrors({});
    }
    prevOpen.current = open;
  }, [open]);

  // Auto-select the only device if there's exactly one
  useEffect(() => {
    if (open && writableDevices.length === 1 && !deviceUuid) {
      setDeviceUuid(writableDevices[0].uuid);
    }
  }, [open, writableDevices, deviceUuid]);

  const validate = (): boolean => {
    const newErrors: Partial<Record<'name' | 'deviceUuid', string>> = {};
    if (!name.trim()) newErrors.name = t('project.create.validation.nameRequired');
    if (!deviceUuid) newErrors.deviceUuid = t('project.create.validation.deviceRequired');
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (loading || !validate()) return;
    onSubmit({
      name: name.trim(),
      description: description.trim(),
      sessionDate,
      deviceUuid,
    });
  };

  const handleClose = () => {
    if (loading) return;
    onClose();
  };

  const drives = devicesToDrives(writableDevices);

  return (
    <Modal
      title={t('project.create.title')}
      open={open}
      onClose={handleClose}
      closable
      style={{ width: '680px' }}
    >
      <form onSubmit={handleSubmit} style={formStyles}>
        {error && <div style={errorStyles}>{error}</div>}

        <div style={columnsStyles}>
          {/* Left column: name + description + date */}
          <div style={columnStyles}>
            <Input
              label={t('project.create.name')}
              placeholder={t('project.create.namePlaceholder')}
              value={name}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
              error={errors.name}
              fullWidth
              autoFocus
              disabled={loading}
            />

            <Input
              label={t('project.create.description')}
              variant="textarea"
              placeholder={t('project.create.descriptionPlaceholder')}
              value={description}
              onChange={(e: ChangeEvent<HTMLTextAreaElement>) => setDescription(e.target.value)}
              fullWidth
              disabled={loading}
            />

            <DateField
              label={t('project.create.date')}
              value={sessionDate}
              onChange={setSessionDate}
              placeholder={t('components.createProjectModal.datePlaceholder')}
              disabled={loading}
            />
          </div>

          {/* Right column: device selector */}
          <div>
            <DriveSelector
              drives={drives}
              selectedId={deviceUuid}
              onSelect={(drive: Drive) => setDeviceUuid(drive.id)}
              label={t('project.create.device')}
            />
            {errors.deviceUuid && (
              <span style={fieldErrorStyles}>{errors.deviceUuid}</span>
            )}
          </div>
        </div>

        <div style={actionsStyles}>
          <Button variant="ghost" type="button" onClick={handleClose} disabled={loading}>
            {t('project.create.buttons.cancel')}
          </Button>
          <Button variant="primary" type="submit" disabled={loading || !deviceUuid}>
            {loading ? t('common.loading') : t('project.create.buttons.create')}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
