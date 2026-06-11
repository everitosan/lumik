import type { CSSProperties } from 'react';
import { Button, Icon } from '@lumik/ui';

export interface ProjectDetailFooterProps {
  totalPhotos: number;
  culledCount: number;
  onImport?: () => void;
}

const footerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '10px 32px',
  borderTop: '1px solid var(--lumik-outline-variant, #424654)',
  flexShrink: 0,
};

const statsStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '12px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const statValueStyles: CSSProperties = {
  color: 'var(--lumik-on-surface, #e5e2e1)',
  fontWeight: 500,
};

const sepStyles: CSSProperties = {
  color: 'var(--lumik-outline-variant, #424654)',
};

const culledValueStyles: CSSProperties = {
  color: 'var(--lumik-tertiary, #ffb690)',
  fontWeight: 500,
};

export function ProjectDetailFooter({
  totalPhotos,
  culledCount,
  onImport,
}: ProjectDetailFooterProps) {
  return (
    <footer style={footerStyles}>
      <div style={statsStyles}>
        <span style={statValueStyles}>{totalPhotos.toLocaleString()}</span>
        <span>photos</span>
        <span style={sepStyles}>•</span>
        <span style={culledValueStyles}>{culledCount}</span>
        <span>culled</span>
      </div>

      <Button
        variant="primary"
        size="sm"
        leftIcon={<Icon name="import" size="sm" />}
        onClick={onImport}
      >
        Import
      </Button>
    </footer>
  );
}
