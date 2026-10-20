import type { CSSProperties, KeyboardEvent, MouseEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '../Icon';
import { Loader } from '../Loader';

export interface BackupJobCardProps {
  /** Project name; truncated with an ellipsis when it does not fit. */
  name: string;
  /** Files already uploaded. */
  done: number;
  total: number;
  /** Files that failed. When > 0 and not running, the card shows its error state. */
  failed?: number;
  running: boolean;
  /** Extra line under the progress, e.g. why a retry could not start. */
  message?: string;
  onClick?: () => void;
  /** Shown only in the error state. */
  onRetry?: () => void;
  /** Shown only in the error state, as an × in the header. */
  onDismiss?: () => void;
  className?: string;
  style?: CSSProperties;
}

const containerStyles = (clickable: boolean, hasError: boolean): CSSProperties => ({
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
  padding: '10px 12px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  border: `1px solid ${hasError ? 'var(--lumik-error, #ffb4ab)' : 'var(--lumik-outline-variant, #424654)'}`,
  cursor: clickable ? 'pointer' : 'default',
});

const headerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  minWidth: 0,
};

const nameStyles: CSSProperties = {
  flex: 1,
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const countStyles = (hasError: boolean): CSSProperties => ({
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  color: hasError ? 'var(--lumik-error, #ffb4ab)' : 'var(--lumik-on-surface-variant, #c2c6d7)',
  whiteSpace: 'nowrap',
});

const iconBtnStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: '22px',
  height: '22px',
  padding: 0,
  border: 'none',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  background: 'transparent',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  cursor: 'pointer',
  flexShrink: 0,
};

const progressContainerStyles: CSSProperties = {
  width: '100%',
  height: '4px',
  backgroundColor: 'var(--lumik-surface-container-highest, #353534)',
  borderRadius: '2px',
  overflow: 'hidden',
};

const progressBarStyles = (percentage: number, hasError: boolean): CSSProperties => ({
  height: '100%',
  width: `${percentage}%`,
  backgroundColor: hasError ? 'var(--lumik-error, #ffb4ab)' : 'var(--lumik-primary, #b0c6ff)',
  transition: 'width 0.3s ease',
});

const footerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '8px',
};

const failedTextStyles: CSSProperties = {
  fontSize: '12px',
  color: 'var(--lumik-error, #ffb4ab)',
};

const retryBtnStyles: CSSProperties = {
  padding: '2px 8px',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  background: 'transparent',
  color: 'var(--lumik-primary, #b0c6ff)',
  fontSize: '12px',
  cursor: 'pointer',
};

const messageStyles: CSSProperties = {
  fontSize: '12px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

/** Backup progress of one project, for the sidebar: name, n/total and, when it
 *  finished with errors, how many failed plus retry and dismiss actions. */
export function BackupJobCard({
  name,
  done,
  total,
  failed = 0,
  running,
  message,
  onClick,
  onRetry,
  onDismiss,
  className,
  style,
}: BackupJobCardProps) {
  const { t } = useTranslation();
  const hasError = !running && failed > 0;
  const percentage = total > 0 ? (done / total) * 100 : 0;

  // The whole card opens the detail; its own buttons must not.
  const stop = (fn?: () => void) => (e: MouseEvent) => {
    e.stopPropagation();
    fn?.();
  };
  const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (!onClick || e.target !== e.currentTarget) return;
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onClick();
    }
  };

  return (
    <div
      style={{ ...containerStyles(!!onClick, hasError), ...style }}
      className={className}
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      onClick={onClick}
      onKeyDown={handleKeyDown}
    >
      <div style={headerStyles}>
        {running && <Loader size={12} />}
        <span style={nameStyles} title={name}>{name}</span>
        <span style={countStyles(hasError)}>{done}/{total}</span>
        {hasError && onDismiss && (
          <button
            type="button"
            style={iconBtnStyles}
            onClick={stop(onDismiss)}
            title={t('components.backupJobCard.dismiss')}
            aria-label={t('components.backupJobCard.dismiss')}
          >
            <Icon name="x" size={14} />
          </button>
        )}
      </div>

      <div style={progressContainerStyles}>
        <div style={progressBarStyles(percentage, hasError)} />
      </div>

      {hasError && (
        <div style={footerStyles}>
          <span style={failedTextStyles}>{t('components.backupJobCard.failed', { count: failed })}</span>
          {onRetry && (
            <button type="button" style={retryBtnStyles} onClick={stop(onRetry)}>
              {t('components.backupJobCard.retry')}
            </button>
          )}
        </div>
      )}

      {message && <span style={messageStyles}>{message}</span>}
    </div>
  );
}
