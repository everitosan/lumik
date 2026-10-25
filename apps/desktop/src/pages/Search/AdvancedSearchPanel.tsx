import { useState, type CSSProperties, type ChangeEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Checkbox, Input } from '@lumik/ui';
import type { SearchPeriod, SearchQuery } from '../../lib/types';

/** Colores de etiqueta, en el orden del filtro por color (1 rojo … 5 púrpura). */
const COLOR_SWATCHES: Record<number, string> = {
  1: '#C0392B',
  2: '#E9C349',
  3: '#27AE60',
  4: '#4B7FE8',
  5: '#9B59B6',
};

const panelStyles: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'minmax(140px, 180px) minmax(0, 1fr)',
  alignItems: 'center',
  gap: '12px 16px',
  padding: '20px 24px',
  background: 'var(--lumik-surface-container, #201f1f)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: '8px',
  boxShadow: '0 16px 40px rgba(0, 0, 0, 0.45)',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const labelStyles: CSSProperties = { color: 'var(--lumik-on-surface-variant, #c2c6d7)' };
const rowStyles: CSSProperties = { display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: '12px' };
const selectStyles: CSSProperties = {
  padding: '8px 10px',
  background: 'var(--lumik-surface-container-high, #2a2a2a)',
  color: 'inherit',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: '6px',
  fontFamily: 'inherit',
  fontSize: '14px',
};

/** `calle, retrato` → `['calle', 'retrato']` */
function splitList(value: string): string[] {
  return value.split(',').map((v) => v.trim()).filter(Boolean);
}

function periodToText(period: SearchPeriod | null): string {
  if (!period) return '';
  return period.kind === 'year' ? String(period.year) : `${period.year}-${String(period.month).padStart(2, '0')}`;
}

/** `2025` o `2025-06`; vacío o inválido → sin periodo. */
function textToPeriod(value: string): SearchPeriod | null {
  const m = value.trim().match(/^(\d{4})(?:-(\d{2}))?$/);
  if (!m) return null;
  const year = Number(m[1]);
  if (!m[2]) return { kind: 'year', year };
  const month = Number(m[2]);
  return month >= 1 && month <= 12 ? { kind: 'month', year, month } : null;
}

function starsToValue(q: SearchQuery): string {
  if (!q.stars) return '';
  return `${q.stars.mode === 'at_least' ? 'ge' : 'eq'}:${q.stars.value}`;
}

interface Props {
  initial: SearchQuery;
  onSearch: (query: SearchQuery) => void;
  onClose: () => void;
}

/**
 * Búsqueda avanzada al estilo Gmail (spec `photo-search`): cada campo es un
 * operador. Se precarga con lo escrito en el cuadro y, al buscar, escribe ahí
 * los operadores equivalentes (eso lo hace quien lo abre).
 */
export function AdvancedSearchPanel({ initial, onSearch, onClose }: Props) {
  const { t } = useTranslation();
  const [text, setText] = useState(initial.text.join(' '));
  const [tags, setTags] = useState(initial.tags.join(', '));
  const [notTags, setNotTags] = useState(initial.not_tags.join(', '));
  const [stars, setStars] = useState(starsToValue(initial));
  const [colors, setColors] = useState<number[]>(initial.colors);
  const [camera, setCamera] = useState(initial.camera ?? '');
  const [lens, setLens] = useState(initial.lens ?? '');
  const [period, setPeriod] = useState(periodToText(initial.period));
  const [after, setAfter] = useState(initial.after ?? '');
  const [before, setBefore] = useState(initial.before ?? '');
  const [project, setProject] = useState(initial.project ?? '');
  const [approved, setApproved] = useState(initial.approved);
  const [delivered, setDelivered] = useState(initial.delivered);
  const [pair, setPair] = useState(initial.pair);
  const [includeArchived, setIncludeArchived] = useState(initial.include_archived);

  const toggleColor = (c: number) =>
    setColors((cs) => (cs.includes(c) ? cs.filter((x) => x !== c) : [...cs, c].sort()));

  const clear = () => {
    for (const set of [setText, setTags, setNotTags, setStars, setCamera, setLens, setPeriod, setAfter, setBefore, setProject]) set('');
    setColors([]);
    for (const set of [setApproved, setDelivered, setPair, setIncludeArchived]) set(false);
  };

  const submit = () => {
    const [mode, value] = stars ? stars.split(':') : [];
    onSearch({
      // Palabras sueltas, como si se escribieran en el cuadro (todas deben estar).
      text: text.trim() ? text.trim().split(/\s+/) : [],
      tags: splitList(tags),
      not_tags: splitList(notTags),
      stars: mode ? { mode: mode === 'ge' ? 'at_least' : 'exact', value: Number(value) } : null,
      colors,
      camera: camera.trim() || null,
      lens: lens.trim() || null,
      period: textToPeriod(period),
      after: after || null,
      before: before || null,
      project: project.trim() || null,
      approved,
      delivered,
      pair,
      include_archived: includeArchived,
    });
  };

  const field = (value: string, set: (v: string) => void, placeholder?: string) => (
    <Input value={value} placeholder={placeholder} onChange={(e: ChangeEvent<HTMLInputElement>) => set(e.target.value)} fullWidth />
  );

  return (
    <form
      style={panelStyles}
      onSubmit={(e) => { e.preventDefault(); submit(); }}
      onKeyDown={(e) => { if (e.key === 'Escape') onClose(); }}
    >
      <span style={labelStyles}>{t('search.panel.text')}</span>
      {field(text, setText, t('search.panel.textPlaceholder'))}

      <span style={labelStyles}>{t('search.panel.tags')}</span>
      {field(tags, setTags, t('search.panel.tagsPlaceholder'))}

      <span style={labelStyles}>{t('search.panel.notTags')}</span>
      {field(notTags, setNotTags)}

      <span style={labelStyles}>{t('search.panel.stars')}</span>
      <div style={rowStyles}>
        <select style={selectStyles} value={stars} onChange={(e) => setStars(e.target.value)} aria-label={t('search.panel.stars')}>
          <option value="">{t('search.panel.any')}</option>
          {[1, 2, 3, 4, 5].map((n) => (
            <option key={`ge${n}`} value={`ge:${n}`}>{t('search.panel.atLeast', { count: n })}</option>
          ))}
          {[0, 1, 2, 3, 4, 5].map((n) => (
            <option key={`eq${n}`} value={`eq:${n}`}>{t('search.panel.exactly', { count: n })}</option>
          ))}
        </select>
        <span style={labelStyles}>{t('search.panel.color')}</span>
        {[1, 2, 3, 4, 5].map((c) => (
          <button
            key={c}
            type="button"
            aria-pressed={colors.includes(c)}
            aria-label={t(`photo.colors.${c}`)}
            title={t(`photo.colors.${c}`)}
            onClick={() => toggleColor(c)}
            style={{
              width: '24px', height: '24px', borderRadius: '50%', padding: 0, cursor: 'pointer',
              background: COLOR_SWATCHES[c],
              border: colors.includes(c) ? '2px solid var(--lumik-on-surface, #e5e2e1)' : '2px solid transparent',
              outline: colors.includes(c) ? '2px solid var(--lumik-primary, #1a73e8)' : 'none',
            }}
          />
        ))}
      </div>

      <span style={labelStyles}>{t('search.panel.camera')}</span>
      <div style={{ ...rowStyles, flexWrap: 'nowrap' }}>
        {field(camera, setCamera, 'X-T5')}
        {field(lens, setLens, t('search.panel.lensPlaceholder'))}
      </div>

      <span style={labelStyles}>{t('search.panel.date')}</span>
      <div style={rowStyles}>
        <div style={{ width: '140px' }}>{field(period, setPeriod, t('search.panel.periodPlaceholder'))}</div>
        <span style={labelStyles}>{t('search.panel.after')}</span>
        <input type="date" value={after} onChange={(e) => setAfter(e.target.value)} style={selectStyles} aria-label={t('search.panel.after')} />
        <span style={labelStyles}>{t('search.panel.before')}</span>
        <input type="date" value={before} onChange={(e) => setBefore(e.target.value)} style={selectStyles} aria-label={t('search.panel.before')} />
      </div>

      <span style={labelStyles}>{t('search.panel.project')}</span>
      {field(project, setProject)}

      <span style={labelStyles}>{t('search.panel.state')}</span>
      <div style={rowStyles}>
        <Checkbox label={t('search.panel.approved')} checked={approved} onChange={setApproved} />
        <Checkbox label={t('search.panel.delivered')} checked={delivered} onChange={setDelivered} />
        <Checkbox label={t('search.panel.pair')} checked={pair} onChange={setPair} />
      </div>

      <span />
      <Checkbox label={t('search.panel.includeArchived')} checked={includeArchived} onChange={setIncludeArchived} />

      <span />
      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '12px' }}>
        <Button variant="ghost" type="button" onClick={clear}>{t('search.panel.clear')}</Button>
        <Button variant="primary" type="submit">{t('search.panel.search')}</Button>
      </div>
    </form>
  );
}
