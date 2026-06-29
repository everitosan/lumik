import type { CSSProperties } from 'react';
import { Icon } from '../Icon';

export interface Drive {
  /** Unique identifier for the drive */
  id: string;
  /** Display name of the drive */
  name: string;
  /** Filesystem UUID */
  uuid: string;
  /** Total capacity in bytes */
  totalBytes: number;
  /** Used space in bytes */
  usedBytes: number;
  /** Whether the drive is currently connected/mounted */
  connected?: boolean;
  /** Mount path (optional) */
  mountPath?: string;
}

export interface DriveSelectorProps {
  /** Available drives to select from */
  drives: Drive[];
  /** Currently selected drive ID */
  selectedId?: string;
  /** Callback when a drive is selected */
  onSelect?: (drive: Drive) => void;
  /** Minimum required free space in bytes (drives with less space are disabled) */
  requiredBytes?: number;
  /** Label for the selector */
  label?: string;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

const containerStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
};

const labelStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
};

const listStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
};

const getDriveCardStyles = (isSelected: boolean, isDisabled: boolean): CSSProperties => ({
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  padding: '12px 16px',
  backgroundColor: isSelected
    ? 'rgba(176, 198, 255, 0.1)'
    : 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  border: `1px solid ${
    isSelected
      ? 'var(--lumik-primary, #b0c6ff)'
      : 'var(--lumik-outline-variant, #424654)'
  }`,
  cursor: isDisabled ? 'not-allowed' : 'pointer',
  opacity: isDisabled ? 0.5 : 1,
  transition: 'var(--lumik-transition-fast, 150ms ease)',
});

const driveIconStyles = (isSelected: boolean): CSSProperties => ({
  width: '40px',
  height: '40px',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  backgroundColor: isSelected
    ? 'rgba(176, 198, 255, 0.15)'
    : 'var(--lumik-surface-container-high, #2a2a2a)',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  flexShrink: 0,
});

const driveInfoStyles: CSSProperties = {
  flex: 1,
  display: 'flex',
  flexDirection: 'column',
  gap: '2px',
  minWidth: 0,
};

const driveNameStyles = (isSelected: boolean): CSSProperties => ({
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  fontWeight: 500,
  color: isSelected
    ? 'var(--lumik-primary, #b0c6ff)'
    : 'var(--lumik-on-surface, #e5e2e1)',
});

const driveUuidStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '11px',
  color: 'var(--lumik-outline, #8c90a0)',
  whiteSpace: 'nowrap',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
};

const driveMetaStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'flex-end',
  gap: '2px',
  flexShrink: 0,
};

const driveCapacityStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '13px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const getStatusStyles = (isAvailable: boolean, hasSpace: boolean): CSSProperties => ({
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '11px',
  color: !isAvailable
    ? 'var(--lumik-outline, #8c90a0)'
    : !hasSpace
      ? 'var(--lumik-error, #ffb4ab)'
      : 'var(--lumik-secondary, #e9c349)',
});

function getStatusText(connected: boolean, hasSpace: boolean): string {
  if (!connected) return 'Desconectado';
  if (!hasSpace) return 'Sin espacio';
  return 'Disponible';
}

export function DriveSelector({
  drives,
  selectedId,
  onSelect,
  requiredBytes = 0,
  label = 'Select destination',
}: DriveSelectorProps) {
  const handleSelect = (drive: Drive, freeBytes: number) => {
    if (drive.connected === false || freeBytes < requiredBytes) return;
    onSelect?.(drive);
  };

  return (
    <div style={containerStyles}>
      {label && <span style={labelStyles}>{label}</span>}

      <div style={listStyles}>
        {drives.map((drive) => {
          const isSelected = drive.id === selectedId;
          const freeBytes = drive.totalBytes - drive.usedBytes;
          const hasSpace = freeBytes >= requiredBytes;
          const isConnected = drive.connected !== false;
          const isDisabled = !isConnected || !hasSpace;

          return (
            <div
              key={drive.id}
              style={getDriveCardStyles(isSelected, isDisabled)}
              onClick={() => handleSelect(drive, freeBytes)}
              role="button"
              tabIndex={isDisabled ? -1 : 0}
              onKeyDown={(e) => {
                if (!isDisabled && (e.key === 'Enter' || e.key === ' ')) {
                  e.preventDefault();
                  handleSelect(drive, freeBytes);
                }
              }}
            >
              <div style={driveIconStyles(isSelected)}>
                <Icon
                  name="drive"
                  size="md"
                  color={
                    isSelected
                      ? 'var(--lumik-primary, #b0c6ff)'
                      : 'var(--lumik-on-surface-variant, #c2c6d7)'
                  }
                />
              </div>

              <div style={driveInfoStyles}>
                <span style={driveNameStyles(isSelected)}>{drive.name}</span>
                <span style={driveUuidStyles}>{drive.uuid}</span>
              </div>

              <div style={driveMetaStyles}>
                <span style={driveCapacityStyles}>{formatBytes(freeBytes)} libres</span>
                <span style={getStatusStyles(isConnected, hasSpace)}>
                  {getStatusText(isConnected, hasSpace)}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
