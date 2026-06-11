import { useState, useRef, useEffect, type CSSProperties } from 'react';
import { Icon } from '@lumik/ui';
import type { ColorLabel } from '@lumik/ui';
import type { Photo } from '../../../lib/types';
import type { HistogramBins } from './PhotoViewer';

// ─── Constants ────────────────────────────────────────────────────────────────

const MAX_STARS = 5;

const COLOR_MAP: Record<ColorLabel, string> = {
  1: '#C0392B',
  2: '#E9C349',
  3: '#27AE60',
  4: '#4B7FE8',
  5: '#9B59B6',
};

// ─── Styles ───────────────────────────────────────────────────────────────────

const panelStyle: CSSProperties = {
  width: '300px',
  flexShrink: 0,
  display: 'flex',
  flexDirection: 'column',
  borderLeft: '1px solid var(--lumik-outline-variant, #424654)',
  overflowY: 'auto',
  background: 'var(--lumik-surface-container-low, #1c1b1b)',
  padding: '20px',
  gap: '20px',
};

const sectionStyle: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '10px' };

const labelStyle: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '10px',
  fontWeight: 500,
  letterSpacing: '0.1em',
  color: 'var(--lumik-outline, #8c90a0)',
  textTransform: 'uppercase',
};

const dividerStyle: CSSProperties = {
  height: '1px',
  background: 'var(--lumik-outline-variant, #424654)',
  flexShrink: 0,
};

// ─── Histogram ────────────────────────────────────────────────────────────────

function Histogram({ bins }: { bins: HistogramBins }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const W = canvas.width;
    const H = canvas.height;
    ctx.clearRect(0, 0, W, H);

    let max = 1;
    for (let i = 1; i < 255; i++) {
      if (bins.r[i] > max) max = bins.r[i];
      if (bins.g[i] > max) max = bins.g[i];
      if (bins.b[i] > max) max = bins.b[i];
    }

    const pad = 10;
    const drawW = W - pad * 2;
    const drawH = H - pad * 2;

    ctx.strokeStyle = 'rgba(255,255,255,0.06)';
    ctx.lineWidth = 1;
    for (let i = 1; i < 4; i++) {
      const x = pad + (i / 4) * drawW;
      ctx.beginPath(); ctx.moveTo(x, pad); ctx.lineTo(x, pad + drawH); ctx.stroke();
    }
    for (let i = 1; i < 3; i++) {
      const y = pad + (i / 3) * drawH;
      ctx.beginPath(); ctx.moveTo(pad, y); ctx.lineTo(pad + drawW, y); ctx.stroke();
    }

    const drawChannel = (data: Uint32Array, color: string) => {
      ctx.beginPath();
      for (let i = 0; i < 256; i++) {
        const x = pad + (i / 255) * drawW;
        const normalized = Math.sqrt(data[i] / max);
        const y = pad + drawH - normalized * drawH;
        if (i === 0) ctx.moveTo(x, pad + drawH);
        ctx.lineTo(x, y);
      }
      ctx.lineTo(pad + drawW, pad + drawH);
      ctx.closePath();
      ctx.fillStyle = color;
      ctx.fill();
    };

    ctx.globalCompositeOperation = 'screen';
    drawChannel(bins.r, 'rgba(220, 60, 60, 0.75)');
    drawChannel(bins.g, 'rgba(60, 200, 60, 0.75)');
    drawChannel(bins.b, 'rgba(60, 120, 220, 0.75)');
    ctx.globalCompositeOperation = 'source-over';
  }, [bins]);

  return (
    <canvas
      ref={canvasRef}
      width={260}
      height={160}
      style={{
        display: 'block',
        width: '100%',
        height: '160px',
        borderRadius: '6px',
        background: 'rgba(10, 10, 10, 0.6)',
      }}
    />
  );
}

// ─── Params grid ──────────────────────────────────────────────────────────────

interface ParamCell { label: string; value: string | null; }

function ParamsGrid({ params }: { params: [ParamCell, ParamCell, ParamCell, ParamCell] }) {
  return (
    <div style={{
      display: 'grid',
      gridTemplateColumns: '1fr 1fr',
      border: '1px solid var(--lumik-outline-variant, #424654)',
      borderRadius: 'var(--lumik-radius-md, 8px)',
      overflow: 'hidden',
    }}>
      {params.map((cell, i) => (
        <div
          key={cell.label}
          style={{
            padding: '12px 16px',
            display: 'flex',
            flexDirection: 'column',
            gap: '6px',
            borderRight: i % 2 === 0 ? '1px solid var(--lumik-outline-variant, #424654)' : undefined,
            borderBottom: i < 2 ? '1px solid var(--lumik-outline-variant, #424654)' : undefined,
          }}
        >
          <span style={{ fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)', fontSize: '10px', fontWeight: 500, letterSpacing: '0.1em', textTransform: 'uppercase' as const, color: 'var(--lumik-outline, #8c90a0)' }}>
            {cell.label}
          </span>
          <span style={{ fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)', fontSize: '22px', fontWeight: 700, lineHeight: '1', color: cell.value ? 'var(--lumik-on-surface, #e5e2e1)' : 'var(--lumik-outline-variant, #424654)' }}>
            {cell.value ?? '—'}
          </span>
        </div>
      ))}
    </div>
  );
}

// ─── Camera spec row ──────────────────────────────────────────────────────────

function CameraSpecRow({ label, value }: { label: string; value: string | null }) {
  if (!value) return null;
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '3px' }}>
      <span style={labelStyle}>{label}</span>
      <span style={{ fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)', fontSize: '26px', fontWeight: 700, color: 'var(--lumik-on-surface, #e5e2e1)', lineHeight: '1.2' }}>
        {value}
      </span>
    </div>
  );
}

// ─── Star input ───────────────────────────────────────────────────────────────

function StarInput({ value, onChange }: { value: number; onChange: (v: number) => void }) {
  return (
    <div style={{ display: 'flex', gap: '2px' }}>
      {Array.from({ length: MAX_STARS }, (_, i) => (
        <button
          key={i}
          onClick={() => onChange(value === i + 1 ? 0 : i + 1)}
          title={`${i + 1} estrellas`}
          style={{
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            width: '32px', height: '32px', padding: 0,
            background: 'transparent', border: 'none',
            borderRadius: 'var(--lumik-radius, 4px)', cursor: 'pointer',
            color: i < value ? 'var(--lumik-secondary, #e9c349)' : 'var(--lumik-outline-variant, #424654)',
          }}
        >
          <Icon name={i < value ? 'star-filled' : 'star'} size={20} />
        </button>
      ))}
    </div>
  );
}

// ─── Color input ──────────────────────────────────────────────────────────────

function ColorInput({ selected, onChange }: { selected: ColorLabel[]; onChange: (v: ColorLabel[]) => void }) {
  const ALL_COLORS = [1, 2, 3, 4, 5] as ColorLabel[];
  const toggle = (label: ColorLabel) => {
    onChange(selected.includes(label) ? selected.filter((l) => l !== label) : [...selected, label]);
  };
  return (
    <div style={{ display: 'flex', gap: '10px' }}>
      {ALL_COLORS.map((label) => {
        const active = selected.includes(label);
        return (
          <button
            key={label}
            onClick={() => toggle(label)}
            title={`Color ${label}`}
            style={{
              width: '24px', height: '24px', borderRadius: '50%',
              backgroundColor: COLOR_MAP[label],
              border: active ? '2px solid var(--lumik-on-surface, #e5e2e1)' : '2px solid transparent',
              outline: active ? '1px solid rgba(255,255,255,0.3)' : 'none',
              cursor: 'pointer', padding: 0, flexShrink: 0,
              boxSizing: 'border-box' as const,
            }}
          />
        );
      })}
    </div>
  );
}

// ─── Tag manager ──────────────────────────────────────────────────────────────

function TagManager({ tags, onChange }: { tags: string[]; onChange: (tags: string[]) => void }) {
  const [adding, setAdding] = useState(false);
  const [input, setInput] = useState('');

  const confirmAdd = () => {
    const trimmed = input.trim();
    if (trimmed && !tags.includes(trimmed)) onChange([...tags, trimmed]);
    setInput('');
    setAdding(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    e.stopPropagation();
    if (e.key === 'Enter') confirmAdd();
    if (e.key === 'Escape') { setInput(''); setAdding(false); }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px', alignItems: 'center' }}>
        {tags.map((tag) => (
          <span
            key={tag}
            style={{
              display: 'inline-flex', alignItems: 'center', gap: '4px',
              padding: '4px 8px 4px 10px',
              fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
              fontSize: '16px', fontWeight: 500,
              borderRadius: 'var(--lumik-radius, 4px)',
              backgroundColor: 'var(--lumik-surface-container, #201f1f)',
              color: 'var(--lumik-on-surface-variant, #c2c6d7)',
              border: '1px solid var(--lumik-outline-variant, #424654)',
            }}
          >
            {tag}
            <button
              onClick={() => onChange(tags.filter((t) => t !== tag))}
              title="Eliminar tag"
              style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: '14px', height: '14px', padding: 0, background: 'transparent', border: 'none', cursor: 'pointer', color: 'var(--lumik-outline, #8c90a0)', fontSize: '12px', lineHeight: 1, borderRadius: '2px' }}
            >
              ×
            </button>
          </span>
        ))}
      </div>
      {adding ? (
        <input
          autoFocus
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={confirmAdd}
          placeholder="Nuevo tag…"
          style={{
            fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
            fontSize: '11px', padding: '4px 8px',
            background: 'var(--lumik-surface-container, #201f1f)',
            border: '1px solid var(--lumik-primary, #b0c6ff)',
            borderRadius: 'var(--lumik-radius, 4px)',
            color: 'var(--lumik-on-surface, #e5e2e1)',
            outline: 'none', width: '100%', boxSizing: 'border-box' as const,
          }}
        />
      ) : (
        <button
          onClick={() => setAdding(true)}
          style={{
            alignSelf: 'flex-start', display: 'flex', alignItems: 'center', gap: '4px',
            padding: '4px 10px',
            fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
            fontSize: '16px', background: 'transparent',
            border: '1px dashed var(--lumik-outline-variant, #424654)',
            borderRadius: 'var(--lumik-radius, 4px)',
            color: 'var(--lumik-outline, #8c90a0)', cursor: 'pointer',
          }}
        >
          + Tag
        </button>
      )}
    </div>
  );
}

// ─── Cull input ───────────────────────────────────────────────────────────────

function CullInput({ culled, onChange }: { culled: boolean; onChange: (v: boolean) => void }) {
  return (
    <label style={{ display: 'flex', alignItems: 'center', gap: '10px', cursor: 'pointer' }}>
      <span
        onClick={() => onChange(!culled)}
        role="checkbox"
        aria-checked={culled}
        style={{
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          width: '22px', height: '22px', borderRadius: '50%',
          border: `2px solid ${culled ? 'var(--lumik-primary, #b0c6ff)' : 'var(--lumik-outline-variant, #424654)'}`,
          backgroundColor: culled ? 'var(--lumik-primary, #b0c6ff)' : 'transparent',
          flexShrink: 0,
          transition: 'background-color 0.15s, border-color 0.15s',
          cursor: 'pointer',
        }}
      >
        {culled && (
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M2 6l3 3 5-5" stroke="var(--lumik-on-primary, #001a41)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        )}
      </span>
      <span style={{ fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '13px', color: 'var(--lumik-on-surface, #e5e2e1)' }}>
        Select photo
      </span>
    </label>
  );
}

// ─── PhotoSidebar ─────────────────────────────────────────────────────────────

export interface PhotoSidebarProps {
  photo: Photo;
  histogramBins: HistogramBins | null;
  localStars: number;
  localColorLabels: ColorLabel[];
  localTags: string[];
  localCulled: boolean;
  onStarsChange: (stars: number) => void;
  onColorChange: (labels: ColorLabel[]) => void;
  onTagsChange: (tags: string[]) => void;
  onCulledChange: (culled: boolean) => void;
}

export function PhotoSidebar({
  photo,
  histogramBins,
  localStars,
  localColorLabels,
  localTags,
  localCulled,
  onStarsChange,
  onColorChange,
  onTagsChange,
  onCulledChange,
}: PhotoSidebarProps) {
  return (
    <div style={panelStyle}>

      {/* Histogram */}
      <div style={sectionStyle}>
        <span style={labelStyle}>Histogram</span>
        <div style={{ width: '100%', height: '160px', borderRadius: '6px', background: 'rgba(10, 10, 10, 0.6)', overflow: 'hidden', flexShrink: 0 }}>
          {histogramBins && <Histogram bins={histogramBins} />}
        </div>
      </div>

      <div style={dividerStyle} />

      {/* Parameters */}
      <div style={sectionStyle}>
        <span style={labelStyle}>Parameters</span>
        <ParamsGrid params={[
          { label: 'ISO',    value: photo.iso != null ? String(photo.iso) : null },
          { label: 'F-Stop', value: photo.aperture },
          { label: 'SS',     value: photo.shutter_speed },
          { label: 'EV',     value: photo.exposure_compensation != null ? photo.exposure_compensation.toFixed(1) : null },
        ]} />
      </div>

      <div style={dividerStyle} />

      {/* Camera specs */}
      {(photo.original_camera || photo.lens_model || photo.focal_length) && (
        <>
          <div style={{ ...sectionStyle, gap: '14px' }}>
            <span style={labelStyle}>Camera Specs</span>
            <CameraSpecRow label="Camera" value={photo.original_camera} />
            <CameraSpecRow label="Optics" value={photo.lens_model} />
            <CameraSpecRow label="Focal"  value={photo.focal_length} />
          </div>
          <div style={dividerStyle} />
        </>
      )}

      {/* Rating */}
      <div style={sectionStyle}>
        <span style={labelStyle}>Rating</span>
        <StarInput value={localStars} onChange={onStarsChange} />
        <span style={{ ...labelStyle, marginTop: '4px' }}>Color</span>
        <ColorInput selected={localColorLabels} onChange={onColorChange} />
        <span style={{ ...labelStyle, marginTop: '4px' }}>Tags</span>
        <TagManager tags={localTags} onChange={onTagsChange} />
      </div>

      <div style={dividerStyle} />

      {/* Cull */}
      <div style={sectionStyle}>
        <span style={labelStyle}>Cull</span>
        <CullInput culled={localCulled} onChange={onCulledChange} />
      </div>

    </div>
  );
}
