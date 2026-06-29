import { useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '../Icon';

export interface DriveInfoProps {
  name: string;
  uuid?: string;
  usedBytes: number;
  totalBytes: number;
  connected?: boolean;
  /** When provided, an eject button is shown. Receives no args; the parent
   *  knows which device this is. Should resolve once the OS eject completes. */
  onEject?: () => Promise<void> | void;
  className?: string;
  style?: CSSProperties;
}

const containerStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  padding: '12px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
};

const headerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
};

const nameContainerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
};

const nameStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const statusDotStyles = (connected: boolean): CSSProperties => ({
  width: '8px',
  height: '8px',
  borderRadius: '50%',
  backgroundColor: connected
    ? 'var(--lumik-secondary, #e9c349)'
    : 'var(--lumik-outline, #8c90a0)',
});

const ejectBtnStyles = (disabled: boolean): CSSProperties => ({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: '28px',
  height: '28px',
  padding: 0,
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  background: 'transparent',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  cursor: disabled ? 'default' : 'pointer',
  opacity: disabled ? 0.5 : 1,
  flexShrink: 0,
});

const capacityTextStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const progressContainerStyles: CSSProperties = {
  width: '100%',
  height: '4px',
  backgroundColor: 'var(--lumik-surface-container-highest, #353534)',
  borderRadius: '2px',
  overflow: 'hidden',
};

const progressBarStyles = (percentage: number): CSSProperties => ({
  height: '100%',
  width: `${percentage}%`,
  backgroundColor:
    percentage > 90
      ? 'var(--lumik-error, #ffb4ab)'
      : percentage > 70
        ? 'var(--lumik-secondary, #e9c349)'
        : 'var(--lumik-primary, #b0c6ff)',
  transition: 'width 0.3s ease',
});

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export function DriveInfo({
  name,
  uuid,
  usedBytes,
  totalBytes,
  connected = true,
  onEject,
  className,
  style,
}: DriveInfoProps) {
  const { t } = useTranslation();
  const [ejecting, setEjecting] = useState(false);
  const usedPercentage = totalBytes > 0 ? (usedBytes / totalBytes) * 100 : 0;
  const freeBytes = totalBytes - usedBytes;

  const handleEject = async () => {
    if (!onEject || ejecting) return;
    setEjecting(true);
    try {
      await onEject();
    } finally {
      setEjecting(false);
    }
  };

  return (
    <div style={{ ...containerStyles, ...style }} className={className}>
      <div style={headerStyles}>
        <div style={nameContainerStyles}>
          <Icon name="drive" size="md" color="var(--lumik-on-surface-variant, #c2c6d7)" />
          <span style={nameStyles}>{name}</span>
          <span style={statusDotStyles(connected)} title={connected ? t('components.driveInfo.connected') : t('components.driveInfo.disconnected')} />
        </div>
        {onEject && (
          <button
            type="button"
            style={ejectBtnStyles(ejecting)}
            onClick={handleEject}
            disabled={ejecting}
            title={ejecting ? t('components.driveInfo.ejecting') : t('components.driveInfo.eject')}
            aria-label={t('components.driveInfo.eject')}
          >
            <Icon name="eject" size="sm" />
          </button>
        )}
      </div>

      <div style={progressContainerStyles}>
        <div style={progressBarStyles(usedPercentage)} />
      </div>

      <div style={capacityTextStyles}>
        {formatBytes(freeBytes)} {t('components.driveInfo.freeOf')} {formatBytes(totalBytes)}
      </div>

      {uuid && (
        <div style={{ ...capacityTextStyles, fontSize: '10px', opacity: 0.7 }}>
          UUID: {uuid.slice(0, 8)}...
        </div>
      )}
    </div>
  );
}
