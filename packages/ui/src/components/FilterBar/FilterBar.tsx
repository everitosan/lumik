import type { CSSProperties, ReactNode } from 'react';

export interface FilterBarProps {
  /** Zona izquierda: contadores y filtros de sí/no. */
  start?: ReactNode;
  /** Zona derecha: filtros con opciones (desplegables). */
  end?: ReactNode;
  className?: string;
  style?: CSSProperties;
}

const barStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  flexWrap: 'wrap',
  rowGap: '8px',
  columnGap: '16px',
  padding: '10px 32px',
  borderTop: '1px solid var(--lumik-outline-variant, #424654)',
  flexShrink: 0,
};

const startStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  flexWrap: 'wrap',
  gap: '16px',
};

const endStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  // Al pasar a dos líneas, la zona derecha sigue alineada a la derecha.
  marginLeft: 'auto',
};

const dividerStyles: CSSProperties = {
  width: '1px',
  height: '20px',
  backgroundColor: 'var(--lumik-outline-variant, #424654)',
  opacity: 0.5,
};

/**
 * Barra inferior de filtros (dashboard y detalle de proyecto): contadores e
 * interruptores a la izquierda, desplegables a la derecha. En anchos estrechos
 * las dos zonas pasan a dos líneas en lugar de apretarse.
 */
export function FilterBar({ start, end, className, style }: FilterBarProps) {
  return (
    <footer className={className} style={{ ...barStyles, ...style }}>
      <div style={startStyles}>{start}</div>
      {end && <div style={endStyles}>{end}</div>}
    </footer>
  );
}

/** Separador vertical entre filtros de la zona derecha. */
export function FilterBarDivider() {
  return <div style={dividerStyles} aria-hidden="true" />;
}
