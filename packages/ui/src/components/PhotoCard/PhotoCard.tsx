import { type CSSProperties, type MouseEvent } from 'react';
import { Icon } from '../Icon';

export type ColorLabel = 1 | 2 | 3 | 4 | 5;

export interface PhotoCardProps {
  filename: string;
  thumbnailUrl?: string;
  stars?: number;
  culled?: boolean;
  captureDate?: string;
  colorLabels?: ColorLabel[];
  onClick?: () => void;
  onStarClick?: (value: number, e: MouseEvent) => void;
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
  padding: '6px 8px',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '11px',
  fontWeight: 500,
  letterSpacing: '0.03em',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
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
  onClick,
  onStarClick,
  className,
  style,
}: PhotoCardProps) {
  const clampedStars = Math.max(0, Math.min(MAX_STARS, Math.round(stars)));
  const { date: captureDay, time: captureTime } = captureDate
    ? parseCaptureDate(captureDate)
    : { date: '—', time: '—' };

  return (
    <div
      style={{
        ...cardStyles,
        ...(culled && { border: '2px solid var(--lumik-primary-container, #3a4a6b)' }),
        ...style,
      }}
      className={className}
      onClick={onClick}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClick?.(); } }}
      role="button"
      tabIndex={0}
    >
      <div style={headerStyles} title={filename}>{basename(filename)}</div>

      <div style={thumbnailWrapStyles}>
        {thumbnailUrl ? (
          <img src={thumbnailUrl} alt={filename} style={thumbnailImgStyles} />
        ) : (
          <div style={placeholderStyles}>
            <Icon name="image" size={40} />
          </div>
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
