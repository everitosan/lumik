import { useState, type CSSProperties, type KeyboardEvent, type MouseEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { CardMenu, type CardMenuItem } from '../CardMenu';
import { Icon } from '../Icon';
import { FormatBadge, type PhotoFormat } from '../FormatBadge';
import { MarkToggle } from '../MarkToggle';

export type ColorLabel = 1 | 2 | 3 | 4 | 5;

export interface PhotoCardProps {
  filename: string;
  thumbnailUrl?: string;
  stars?: number;
  culled?: boolean;
  captureDate?: string;
  colorLabels?: ColorLabel[];
  /** Formato de la foto (`RAW+JPG`, `RAW`, `JPG`), en la esquina superior derecha de la miniatura. */
  format?: PhotoFormat | null;
  /** Recibe el evento para distinguir Ctrl/Cmd/Shift (spec photo-marking). */
  onClick?: (e: MouseEvent | KeyboardEvent) => void;
  onStarClick?: (value: number, e: MouseEvent) => void;
  /** Each handler, when provided, adds its item to the 3-dot menu in the header.
   *  Omit one to hide that option (e.g. onRevealInFolder on Android). If none are
   *  provided the menu button is hidden entirely. */
  /** Agregar la foto a un álbum (spec photo-albums). */
  onAddToAlbum?: () => void;
  onRevealInFolder?: () => void;
  onTrash?: () => void;
  onRestore?: () => void;
  /** En la sección "Papelera": borrarla para siempre. */
  onPurge?: () => void;
  /** Marcada (spec photo-marking): borde primario y casilla marcada. */
  marked?: boolean;
  /** Si viene, la tarjeta muestra la casilla de marcar al pasar el ratón (y siempre si está marcada). */
  onToggleMark?: () => void;
  /** Opciones propias de quien usa la tarjeta (p. ej. reemplazar o quitar de un álbum), al final del menú. */
  extraMenuItems?: CardMenuItem[];
  className?: string;
  style?: CSSProperties;
}

const MAX_STARS = 5;

const COLOR_LABEL_MAP: Record<Exclude<ColorLabel, 0>, string> = {
  1: '#C0392B',
  2: '#E9C349',
  3: '#27AE60',
  4: '#4B7FE8',
  5: '#9B59B6',
};

function basename(path: string): string {
  return path.split('/').pop() ?? path;
}

function parseCaptureDate(raw: string): { date: string; time: string } {
  // EXIF stores dates as "2024:11:07 13:59:07" — normalize to ISO before parsing.
  const normalized = /^\d{4}:\d{2}:\d{2} \d{2}:\d{2}:\d{2}$/.test(raw)
    ? raw.slice(0, 10).replace(/:/g, '-') + 'T' + raw.slice(11)
    : raw;
  const d = new Date(normalized);
  if (isNaN(d.getTime())) return { date: raw, time: '—' };
  const pad = (n: number) => String(n).padStart(2, '0');
  return {
    date: `${d.getFullYear()}/${pad(d.getMonth() + 1)}/${pad(d.getDate())}`,
    time: `${pad(d.getHours())}:${pad(d.getMinutes())}`,
  };
}

const cardStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 8px)',
  overflow: 'hidden',
  cursor: 'pointer',
  transition: 'transform 200ms ease, box-shadow 200ms ease',
  userSelect: 'none',
};

const thumbnailWrapStyles: CSSProperties = {
  position: 'relative',
  width: '100%',
  aspectRatio: '4/3',
  backgroundColor: 'var(--lumik-surface-container-lowest, #0e0e0e)',
  overflow: 'hidden',
  flexShrink: 0,
};

const thumbnailImgStyles: CSSProperties = {
  width: '100%',
  height: '100%',
  objectFit: 'contain',
  display: 'block',
};

const formatBadgePosition: CSSProperties = {
  position: 'absolute',
  top: '6px',
  right: '6px',
};

const placeholderStyles: CSSProperties = {
  width: '100%',
  height: '100%',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  color: 'var(--lumik-outline, #8c90a0)',
};


export const PHOTO_CARD_HEADER_HEIGHT = 29; // padding 6+6 + line-height ~16 + border 1
export const PHOTO_CARD_FOOTER_HEIGHT = 72;

const FOOTER_HEIGHT = PHOTO_CARD_FOOTER_HEIGHT;

const footerStyles: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: '1fr 1fr',
  gridTemplateRows: '1fr 1fr',
  gap: '4px 8px',
  padding: '8px',
  height: `${FOOTER_HEIGHT}px`,
  overflow: 'hidden',
  borderTop: '1px solid var(--lumik-outline-variant, #424654)',
  alignItems: 'center',
  boxSizing: 'border-box',
};

const headerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '4px',
  padding: '6px 4px 6px 8px',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '11px',
  fontWeight: 500,
  letterSpacing: '0.03em',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const filenameStyles: CSSProperties = {
  flex: 1,
  minWidth: 0,
  lineHeight: '16px',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};

const cellStyles: CSSProperties = {
  overflow: 'hidden',
  minWidth: 0,
  minHeight: 0,
  display: 'flex',
  alignItems: 'center',
};

const dateStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '12px',
  fontWeight: 400,
  letterSpacing: '0.02em',
  color: 'var(--lumik-outline, #8c90a0)',
  whiteSpace: 'nowrap',
};

const starsRowStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '2px',
};

const colorDotsRowStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '4px',
};

function StarRating({
  value,
  onChange,
}: {
  value: number;
  onChange?: (v: number, e: MouseEvent) => void;
}) {
  return (
    <div style={starsRowStyles}>
      {Array.from({ length: MAX_STARS }, (_, i) => {
        const filled = i < value;
        return (
          <button
            key={i}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: '12px',
              height: '12px',
              padding: 0,
              background: 'none',
              border: 'none',
              cursor: onChange ? 'pointer' : 'default',
              color: filled
                ? 'var(--lumik-secondary, #e9c349)'
                : 'var(--lumik-outline-variant, #424654)',
            }}
            onClick={(e) => {
              e.stopPropagation();
              onChange?.(i + 1 === value ? 0 : i + 1, e);
            }}
            aria-label={`${i + 1} star${i + 1 !== 1 ? 's' : ''}`}
          >
            <Icon name={filled ? 'star-filled' : 'star'} size={10} />
          </button>
        );
      })}
    </div>
  );
}

function ColorLabels({ value }: { value: ColorLabel[] }) {
  if (value.length === 0) return <span />;
  return (
    <div style={colorDotsRowStyles}>
      {value.map((label) => {
        const color = COLOR_LABEL_MAP[label];
        return (
          <span
            key={label}
            style={{
              display: 'inline-block',
              width: '12px',
              height: '12px',
              borderRadius: '50%',
              backgroundColor: color,
              border: 'none',
              flexShrink: 0,
            }}
            aria-label={`Color label ${label}`}
          />
        );
      })}
    </div>
  );
}

export function PhotoCard({
  filename,
  thumbnailUrl,
  stars = 0,
  culled = false,
  captureDate,
  colorLabels = [],
  format,
  onClick,
  onStarClick,
  onAddToAlbum,
  onRevealInFolder,
  onTrash,
  onRestore,
  onPurge,
  extraMenuItems,
  marked = false,
  onToggleMark,
  className,
  style,
}: PhotoCardProps) {
  const { t } = useTranslation();
  const [hovered, setHovered] = useState(false);
  const clampedStars = Math.max(0, Math.min(MAX_STARS, Math.round(stars)));
  const { date: captureDay, time: captureTime } = captureDate
    ? parseCaptureDate(captureDate)
    : { date: '—', time: '—' };

  const menuItems: CardMenuItem[] = [];
  if (onAddToAlbum) {
    menuItems.push({ icon: 'album', label: t('components.photoCard.menu.addToAlbum'), onSelect: onAddToAlbum });
  }
  if (onRevealInFolder) {
    menuItems.push({ icon: 'folder-open', label: t('components.photoCard.menu.revealInFolder'), onSelect: onRevealInFolder });
  }
  if (onRestore) {
    menuItems.push({ icon: 'archive-restore', label: t('components.photoCard.menu.restore'), onSelect: onRestore });
  }
  if (onTrash) {
    menuItems.push({ icon: 'trash', label: t('components.photoCard.menu.trash'), onSelect: onTrash, danger: true });
  }
  if (onPurge) {
    menuItems.push({ icon: 'trash', label: t('components.photoCard.menu.purge'), onSelect: onPurge, danger: true });
  }
  if (extraMenuItems) menuItems.push(...extraMenuItems);

  return (
    <div
      style={{
        ...cardStyles,
        ...(culled && { border: '2px solid var(--lumik-primary-container, #3a4a6b)' }),
        ...(marked && { border: '2px solid var(--lumik-primary, #b0c6ff)', boxShadow: '0 0 0 2px rgba(176, 198, 255, 0.25)' }),
        ...style,
      }}
      className={className}
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClick?.(e); } }}
      role="button"
      tabIndex={0}
    >
      <div style={headerStyles}>
        <span style={filenameStyles} title={filename}>{basename(filename)}</span>
        <CardMenu label={t('components.photoCard.menu.label')} items={menuItems} buttonSize={16} />
      </div>

      <div style={thumbnailWrapStyles}>
        {thumbnailUrl ? (
          <img src={thumbnailUrl} alt={filename} style={thumbnailImgStyles} />
        ) : (
          <div style={placeholderStyles}>
            <Icon name="image" size={40} />
          </div>
        )}
        {format && <FormatBadge format={format} style={formatBadgePosition} />}
        {onToggleMark && (hovered || marked) && (
          <MarkToggle marked={marked} onToggle={onToggleMark} style={{ position: 'absolute', top: '6px', left: '6px' }} />
        )}
      </div>

      <div style={footerStyles}>
        <div style={cellStyles}>
          <span style={dateStyles}>{captureDay}</span>
        </div>
        <div style={{ ...cellStyles, justifyContent: 'flex-end' }}>
          <StarRating value={clampedStars} onChange={onStarClick} />
        </div>
        <div style={cellStyles}>
          <span style={dateStyles}>{captureTime}</span>
        </div>
        <div style={{ ...cellStyles, justifyContent: 'flex-end' }}>
          <ColorLabels value={colorLabels} />
        </div>
      </div>
    </div>
  );
}
