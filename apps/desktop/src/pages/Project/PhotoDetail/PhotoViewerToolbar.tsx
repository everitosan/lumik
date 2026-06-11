import type { CSSProperties } from 'react';
import { Icon } from '@lumik/ui';

export interface PhotoViewerToolbarProps {
  displayScale: number;
  hasPrev: boolean;
  hasNext: boolean;
  onPrev: () => void;
  onNext: () => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onFitToScreen: () => void;
  onRotateLeft: () => void;
  onRotateRight: () => void;
}

const toolbarStyle: CSSProperties = {
  height: '52px',
  flexShrink: 0,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: '2px',
  padding: '0 16px',
  borderTop: '1px solid var(--lumik-outline-variant, #424654)',
  background: 'var(--lumik-surface-container-low, #1c1b1b)',
};

const sepStyle: CSSProperties = {
  width: '1px',
  height: '20px',
  background: 'var(--lumik-outline-variant, #424654)',
  margin: '0 6px',
  flexShrink: 0,
};

const zoomLabelStyle: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '12px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  minWidth: '46px',
  textAlign: 'center',
  flexShrink: 0,
  background: 'transparent',
  border: 'none',
  cursor: 'pointer',
  padding: '2px 4px',
  borderRadius: '4px',
};

function ToolbarBtn({
  onClick,
  disabled,
  title,
  children,
}: {
  onClick: () => void;
  disabled?: boolean;
  title?: string;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={title}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: '36px',
        height: '36px',
        padding: 0,
        background: 'transparent',
        border: 'none',
        borderRadius: 'var(--lumik-radius, 4px)',
        color: disabled
          ? 'var(--lumik-outline-variant, #424654)'
          : 'var(--lumik-on-surface-variant, #c2c6d7)',
        cursor: disabled ? 'default' : 'pointer',
      }}
    >
      {children}
    </button>
  );
}

export function PhotoViewerToolbar({
  displayScale,
  hasPrev,
  hasNext,
  onPrev,
  onNext,
  onZoomIn,
  onZoomOut,
  onFitToScreen,
  onRotateLeft,
  onRotateRight,
}: PhotoViewerToolbarProps) {
  return (
    <div style={toolbarStyle}>
      <ToolbarBtn onClick={onPrev} disabled={!hasPrev} title="Foto anterior (←)">
        <Icon name="chevron-left" size={20} />
      </ToolbarBtn>

      <div style={sepStyle} />

      <ToolbarBtn onClick={onZoomOut} title="Alejar (-)">
        <Icon name="zoom-out" size={18} />
      </ToolbarBtn>
      <button style={zoomLabelStyle} onClick={onFitToScreen} title="Ajustar a pantalla (0)">
        {displayScale}%
      </button>
      <ToolbarBtn onClick={onZoomIn} title="Acercar (+)">
        <Icon name="zoom-in" size={18} />
      </ToolbarBtn>

      <div style={sepStyle} />

      <ToolbarBtn onClick={onRotateLeft} title="Rotar izquierda ([)">
        <Icon name="rotate-ccw" size={18} />
      </ToolbarBtn>
      <ToolbarBtn onClick={onRotateRight} title="Rotar derecha (])">
        <Icon name="rotate-cw" size={18} />
      </ToolbarBtn>

      <div style={sepStyle} />

      <ToolbarBtn onClick={onNext} disabled={!hasNext} title="Siguiente foto (→)">
        <Icon name="chevron-right" size={20} />
      </ToolbarBtn>
    </div>
  );
}
