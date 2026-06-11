import type { CSSProperties, ReactNode } from 'react';
import { useEffect, useCallback } from 'react';
import { Icon } from '../Icon';

export interface ModalProps {
  title: string;
  closable?: boolean;
  children: ReactNode;
  open?: boolean;
  onClose?: () => void;
  className?: string;
  style?: CSSProperties;
}

const overlayStyles: CSSProperties = {
  position: 'fixed',
  inset: 0,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  backgroundColor: 'rgba(0, 0, 0, 0.6)',
  backdropFilter: 'blur(4px)',
  zIndex: 1000,
};

const modalStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  minWidth: '400px',
  maxWidth: '90vw',
  maxHeight: '90vh',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-lg, 12px)',
  boxShadow: '0 24px 48px rgba(0, 0, 0, 0.4)',
  overflow: 'hidden',
};

const headerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '16px 20px',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
};

const titleStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '18px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  margin: 0,
};

const closeButtonStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: '32px',
  height: '32px',
  padding: 0,
  backgroundColor: 'transparent',
  border: 'none',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  cursor: 'pointer',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  transition: 'background-color var(--lumik-transition-fast, 150ms ease)',
};

const contentStyles: CSSProperties = {
  flex: 1,
  padding: '20px',
  overflow: 'auto',
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

export function Modal({
  title,
  closable = true,
  children,
  open = true,
  onClose,
  className,
  style,
}: ModalProps) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape' && closable && onClose) {
        onClose();
      }
    },
    [closable, onClose]
  );

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget && closable && onClose) {
      onClose();
    }
  };

  useEffect(() => {
    if (open) {
      document.addEventListener('keydown', handleKeyDown);
      document.body.style.overflow = 'hidden';
    }

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.body.style.overflow = '';
    };
  }, [open, handleKeyDown]);

  if (!open) return null;

  return (
    <div style={overlayStyles} onClick={handleOverlayClick}>
      <div
        style={{ ...modalStyles, ...style }}
        className={className}
        role="dialog"
        aria-modal="true"
        aria-labelledby="modal-title"
      >
        <div style={headerStyles}>
          <h2 id="modal-title" style={titleStyles}>
            {title}
          </h2>
          {closable && (
            <button
              style={closeButtonStyles}
              onClick={onClose}
              aria-label="Close modal"
            >
              <Icon name="x" size="md" />
            </button>
          )}
        </div>
        <div style={contentStyles}>{children}</div>
      </div>
    </div>
  );
}
