import { useState, useEffect, useRef, useCallback, type ChangeEvent } from 'react';
import flatpickr from 'flatpickr';
import 'flatpickr/dist/flatpickr.min.css';
import '../styles/flatpickr.css';
import { Modal, Input, Button, DriveSelector, type Drive } from '@lumik/ui';
import type { DetectedDevice } from '../lib/types';

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

const dateFieldStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
};

const dateLabelStyles: React.CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '12px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  textTransform: 'uppercase' as const,
  letterSpacing: '0.05em',
};

const dateInputStyles: React.CSSProperties = {
  width: '100%',
  padding: '10px 12px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  fontFamily: 'var(--lumik-font-primary, Inter, sans-serif)',
  fontSize: '14px',
  outline: 'none',
  cursor: 'pointer',
  boxSizing: 'border-box' as const,
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
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [sessionDate, setSessionDate] = useState('');
  const [deviceUuid, setDeviceUuid] = useState('');
  const [errors, setErrors] = useState<Partial<Record<'name' | 'deviceUuid', string>>>({});

  const prevOpen = useRef(open);
  const fpInstance = useRef<ReturnType<typeof flatpickr> | null>(null);

  // Callback ref: fires exactly when the node mounts/unmounts — no timing issues.
  // appendTo: document.body escapes the modal's overflow:hidden.
  const dateInputCallbackRef = useCallback((node: HTMLInputElement | null) => {
    if (node) {
      fpInstance.current = flatpickr(node, {
        dateFormat: 'Y-m-d',
        disableMobile: true,
        appendTo: document.body,
        monthSelectorType: 'static',
        onChange: (_dates, dateStr) => setSessionDate(dateStr),
      });
    } else {
      (fpInstance.current as flatpickr.Instance | null)?.destroy();
      fpInstance.current = null;
    }
  }, []);

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
    if (open && devices.length === 1 && !deviceUuid) {
      setDeviceUuid(devices[0].uuid);
    }
  }, [open, devices, deviceUuid]);

  const validate = (): boolean => {
    const newErrors: Partial<Record<'name' | 'deviceUuid', string>> = {};
    if (!name.trim()) newErrors.name = 'El nombre del proyecto es requerido';
    if (!deviceUuid) newErrors.deviceUuid = 'Selecciona un dispositivo de destino';
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

  const drives = devicesToDrives(devices);

  return (
    <Modal
      title="New Project"
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
              label="Project Name"
              placeholder="e.g. Wedding Martinez-Lopez"
              value={name}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
              error={errors.name}
              fullWidth
              autoFocus
              disabled={loading}
            />

            <Input
              label="Description"
              variant="textarea"
              placeholder="Add notes about the project..."
              value={description}
              onChange={(e: ChangeEvent<HTMLTextAreaElement>) => setDescription(e.target.value)}
              fullWidth
              disabled={loading}
            />

            <div style={dateFieldStyles}>
              <span style={dateLabelStyles}>Session Date</span>
              <input
                ref={dateInputCallbackRef}
                type="text"
                placeholder="Select date..."
                readOnly
                disabled={loading}
                style={{
                  ...dateInputStyles,
                  opacity: loading ? 0.5 : 1,
                  cursor: loading ? 'not-allowed' : 'pointer',
                }}
              />
            </div>
          </div>

          {/* Right column: device selector */}
          <div>
            <DriveSelector
              drives={drives}
              selectedId={deviceUuid}
              onSelect={(drive: Drive) => setDeviceUuid(drive.id)}
              label="Destination Device"
            />
            {errors.deviceUuid && (
              <span style={fieldErrorStyles}>{errors.deviceUuid}</span>
            )}
          </div>
        </div>

        <div style={actionsStyles}>
          <Button variant="ghost" type="button" onClick={handleClose} disabled={loading}>
            Cancel
          </Button>
          <Button variant="primary" type="submit" disabled={loading || !deviceUuid}>
            {loading ? 'Creating...' : 'Create Project'}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
