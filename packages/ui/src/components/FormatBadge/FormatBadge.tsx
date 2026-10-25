import type { CSSProperties, MouseEvent } from 'react';
import { useTranslation } from 'react-i18next';

/** Formato de una foto: un par RAW+JPEG, un RAW suelto o un JPEG suelto. */
export type PhotoFormat = 'RAW+JPG' | 'RAW' | 'JPG';

const RAW_EXTENSIONS = ['cr2', 'cr3', 'nef', 'arw', 'raf', 'orf', 'rw2', 'dng'];
const JPEG_EXTENSIONS = ['jpg', 'jpeg'];

/**
 * Formato a partir del archivo original y del JPG de cámara de un par.
 * Devuelve `null` para formatos sin indicador (TIFF).
 */
export function photoFormat(originalPath: string, cameraJpgPath?: string | null): PhotoFormat | null {
  if (cameraJpgPath) return 'RAW+JPG';
  const ext = originalPath.split('.').pop()?.toLowerCase() ?? '';
  if (RAW_EXTENSIONS.includes(ext)) return 'RAW';
  if (JPEG_EXTENSIONS.includes(ext)) return 'JPG';
  return null;
}

export interface FormatBadgeProps {
  format: PhotoFormat;
  /** En un par, cuál de los dos se está viendo (se resalta). */
  showing?: 'RAW' | 'JPG';
  /** Con un par, pulsarlo cambia entre el JPG y el RAW. */
  onToggle?: () => void;
  style?: CSSProperties;
}

/** Fondo claro semitransparente con texto oscuro: se lee sobre fotos claras y oscuras. */
const badgeStyles: CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  gap: '1px',
  padding: '2px 6px',
  borderRadius: '4px',
  border: '1px solid rgba(19, 19, 19, 0.35)',
  backgroundColor: 'rgba(255, 255, 255, 0.82)',
  boxShadow: '0 1px 4px rgba(0, 0, 0, 0.35)',
  color: '#131313',
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '10px',
  fontWeight: 600,
  letterSpacing: '0.04em',
  lineHeight: '14px',
  pointerEvents: 'none',
};

const dimmedStyles: CSSProperties = { opacity: 0.45 };

/**
 * Indicador del formato de una foto (`RAW+JPG`, `RAW` o `JPG`), para la
 * esquina superior derecha de la miniatura y del visor. En un par puede
 * mostrar cuál de los dos se ve y servir de botón para cambiar.
 */
export function FormatBadge({ format, showing, onToggle, style }: FormatBadgeProps) {
  const { t } = useTranslation();
  const isPair = format === 'RAW+JPG';
  const segments = isPair ? (['RAW', 'JPG'] as const) : [format];

  const content = segments.map((segment, i) => (
    <span key={segment} style={isPair && showing && showing !== segment ? dimmedStyles : undefined}>
      {i > 0 && '+'}
      {segment}
    </span>
  ));

  if (isPair && onToggle) {
    const label = showing === 'RAW' ? t('components.formatBadge.showJpg') : t('components.formatBadge.showRaw');
    return (
      <button
        type="button"
        onClick={(e: MouseEvent) => {
          e.stopPropagation();
          onToggle();
        }}
        aria-label={label}
        title={label}
        style={{ ...badgeStyles, pointerEvents: 'auto', cursor: 'pointer', ...style }}
      >
        {content}
      </button>
    );
  }
  return <span style={{ ...badgeStyles, ...style }}>{content}</span>;
}
