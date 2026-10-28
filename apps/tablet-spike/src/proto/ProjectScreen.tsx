// Vista del proyecto (como `ProjectDetail` del desktop): el grid de las fotos,
// "Importar" y, abajo a la derecha, "Filtros", que abre los filtros de
// visualización (aprobadas, papelera, estrellas ≥/=, colores y tags). Tocar
// una foto abre el visor, que recorre solo las que pasan los filtros.
import { FlashList } from '@shopify/flash-list';
import { Image } from 'expo-image';
import { useMemo, useState } from 'react';
import { Pressable, StyleSheet, Text, useWindowDimensions, View } from 'react-native';

import type { PhotoInfo } from '@lumik/core-native';

// Mismos colores que `@lumik/ui` (PhotoCard) y la barra lateral del desktop.
const LABEL_COLORS = ['#C0392B', '#E9C349', '#27AE60', '#4B7FE8', '#9B59B6'];
const CELL = 150;

export type GridPhoto = { info: PhotoInfo; thumb: string };

export type Filters = {
  approvedOnly: boolean;
  /** Ver la papelera en lugar de las fotos del proyecto (como el desktop). */
  trash: boolean;
  minStars: number | null;
  starsMode: 'inclusive' | 'exact';
  colors: Set<number>;
  tags: Set<string>;
};

export const NO_FILTERS: Filters = { approvedOnly: false, trash: false, minStars: null, starsMode: 'inclusive', colors: new Set(), tags: new Set() };

const splitList = (v?: string) =>
  (v ?? '')
    .split(',')
    .map((x) => x.trim())
    .filter(Boolean);

/** Índices (en `photos`) de las que pasan los filtros, con la misma lógica del desktop. */
export function applyFilters(photos: GridPhoto[], f: Filters): number[] {
  const out: number[] = [];
  photos.forEach((p, i) => {
    const info = p.info;
    if (f.approvedOnly && !info.culled) return;
    if (f.minStars !== null && (f.starsMode === 'exact' ? info.stars !== f.minStars : info.stars < f.minStars)) return;
    if (f.tags.size > 0 && !splitList(info.tags).some((t) => f.tags.has(t))) return;
    if (f.colors.size > 0 && !splitList(info.colorLabel).some((c) => f.colors.has(Number(c)))) return;
    out.push(i);
  });
  return out;
}

/** Cuántos filtros de visualización hay puestos. */
const activeCount = (f: Filters) =>
  Number(f.approvedOnly) + Number(f.trash) + Number(f.minStars !== null) + Number(f.colors.size > 0) + Number(f.tags.size > 0);

type Props = {
  title: string;
  photos: GridPhoto[];
  /** Fotos en la papelera. */
  trash: GridPhoto[];
  filters: Filters;
  onFilters: (f: Filters) => void;
  /** Abre el visor en la foto `position` de la lista filtrada (de las fotos o de la papelera). */
  onOpenPhoto: (fromTrash: boolean, visible: number[], position: number) => void;
  onBack: () => void;
  onImport?: () => void;
  /** Miniaturas que aún se copian al caché (`done/total`), o `null`. */
  fetching: { done: number; total: number } | null;
  notice?: string;
};

export function ProjectScreen({ title, photos, trash, filters, onFilters, onOpenPhoto, onBack, onImport, fetching, notice }: Props) {
  const { width } = useWindowDimensions();
  const columns = Math.max(3, Math.floor((width - 32) / CELL));
  const source = filters.trash ? trash : photos;
  const visible = useMemo(() => applyFilters(source, filters), [source, filters]);
  const approved = useMemo(() => photos.filter((p) => p.info.culled).length, [photos]);
  const allTags = useMemo(() => [...new Set(photos.flatMap((p) => splitList(p.info.tags)))].sort(), [photos]);
  const [showPanel, setShowPanel] = useState(false);
  const active = activeCount(filters);
  const set = (change: Partial<Filters>) => onFilters({ ...filters, ...change });
  const toggle = <T,>(s: Set<T>, v: T) => {
    const next = new Set(s);
    if (next.has(v)) next.delete(v);
    else next.add(v);
    return next;
  };

  return (
    <View style={styles.root}>
      <View style={styles.header}>
        <Pressable style={({ pressed }) => [styles.pill, pressed && styles.pressed]} onPress={onBack} hitSlop={8}>
          <Text style={styles.pillText}>‹ Proyectos</Text>
        </Pressable>
        <Text style={styles.title} numberOfLines={1}>
          {title}
        </Text>
        {fetching && (
          <Text style={styles.muted}>
            Miniaturas {fetching.done}/{fetching.total}
          </Text>
        )}
        {onImport && (
          <Pressable style={({ pressed }) => [styles.primary, pressed && styles.pressed]} onPress={onImport}>
            <Text style={styles.primaryText}>Importar</Text>
          </Pressable>
        )}
      </View>
      {notice && <Text style={styles.notice}>{notice}</Text>}

      <View style={styles.grid}>
        {visible.length === 0 ? (
          <Text style={styles.empty}>
            {filters.trash
              ? 'La papelera está vacía.'
              : photos.length
                ? 'Ninguna foto pasa los filtros.'
                : 'Este proyecto no tiene fotos aún.'}
          </Text>
        ) : (
          <FlashList
            key={columns}
            data={visible}
            numColumns={columns}
            extraData={source}
            keyExtractor={(i) => source[i].info.id}
            renderItem={({ item: i, index: position }) => {
              const p = source[i];
              const labels = splitList(p.info.colorLabel).map(Number);
              return (
                <Pressable style={styles.cell} onPress={() => onOpenPhoto(filters.trash, visible, position)}>
                  <View style={styles.thumbFrame}>
                    <Image source={{ uri: p.thumb }} recyclingKey={p.info.id} style={styles.thumb} contentFit="cover" transition={0} />
                    {p.info.culled && <Text style={styles.approved}>✓</Text>}
                    {labels.length > 0 && (
                      <View style={styles.labels}>
                        {labels.map((l) => (
                          <View key={l} style={[styles.label, { backgroundColor: LABEL_COLORS[l - 1] }]} />
                        ))}
                      </View>
                    )}
                    {p.info.stars > 0 && <Text style={styles.stars}>{'★'.repeat(p.info.stars)}</Text>}
                  </View>
                </Pressable>
              );
            }}
          />
        )}
      </View>

      {/* Pie: contadores a la izquierda; los filtros, abajo a la derecha. */}
      <View style={styles.footer}>
        <Text style={styles.stats}>
          <Text style={styles.statValue}>{photos.length.toLocaleString('es-MX')}</Text> fotos ·{' '}
          <Text style={styles.statApproved}>{approved}</Text> aprobadas
          {filters.trash ? `  ·  papelera (${trash.length})` : visible.length !== photos.length ? `  ·  ${visible.length} visibles` : ''}
        </Text>
        <View style={styles.spacer} />
        <Pressable
          style={({ pressed }) => [styles.filterButton, (showPanel || active > 0) && styles.filterButtonOn, pressed && styles.pressed]}
          onPress={() => setShowPanel((v) => !v)}
        >
          <Text style={[styles.filterButtonText, (showPanel || active > 0) && styles.filterButtonTextOn]}>
            Filtros{active > 0 ? ` (${active})` : ''}
          </Text>
        </Pressable>
      </View>

      {showPanel && (
        <View style={styles.panel}>
          <View style={styles.panelRow}>
            <Toggle label="Aprobadas" on={filters.approvedOnly} onPress={() => set({ approvedOnly: !filters.approvedOnly })} />
            <Toggle label={`Papelera (${trash.length})`} on={filters.trash} onPress={() => set({ trash: !filters.trash })} />
          </View>

          <Text style={styles.panelLabel}>Estrellas</Text>
          <View style={styles.panelRow}>
            <Pressable
              style={styles.mode}
              onPress={() => set({ starsMode: filters.starsMode === 'exact' ? 'inclusive' : 'exact' })}
              hitSlop={6}
            >
              <Text style={styles.modeText}>{filters.starsMode === 'exact' ? '=' : '≥'}</Text>
            </Pressable>
            {[1, 2, 3, 4, 5].map((n) => (
              <Pressable key={n} style={styles.mark} onPress={() => set({ minStars: filters.minStars === n ? null : n })}>
                <Text style={[styles.star, filters.minStars !== null && n <= filters.minStars && styles.starOn]}>★</Text>
              </Pressable>
            ))}
          </View>

          <Text style={styles.panelLabel}>Colores</Text>
          <View style={styles.panelRow}>
            {LABEL_COLORS.map((color, k) => {
              const on = filters.colors.has(k + 1);
              return (
                <Pressable key={color} style={styles.mark} onPress={() => set({ colors: toggle(filters.colors, k + 1) })}>
                  <View style={[styles.dot, { borderColor: color }, on && { backgroundColor: color }]} />
                </Pressable>
              );
            })}
          </View>

          <Text style={styles.panelLabel}>Tags</Text>
          <View style={[styles.panelRow, styles.wrap]}>
            {allTags.length === 0 && <Text style={styles.muted}>Sin etiquetas disponibles</Text>}
            {allTags.map((t) => (
              <Toggle key={t} label={t} on={filters.tags.has(t)} onPress={() => set({ tags: toggle(filters.tags, t) })} />
            ))}
          </View>

          {active > 0 && (
            <Pressable style={styles.clear} onPress={() => onFilters(NO_FILTERS)} hitSlop={8}>
              <Text style={styles.clearText}>Quitar filtros</Text>
            </Pressable>
          )}
        </View>
      )}
    </View>
  );
}

function Toggle({ label, on, onPress }: { label: string; on: boolean; onPress: () => void }) {
  return (
    <Pressable style={({ pressed }) => [styles.toggle, on && styles.toggleOn, pressed && styles.pressed]} onPress={onPress}>
      <Text style={[styles.toggleText, on && styles.toggleTextOn]}>{label}</Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#131313' },
  header: { flexDirection: 'row', alignItems: 'center', gap: 16, paddingHorizontal: 16, paddingVertical: 12 },
  pill: { minHeight: 44, paddingHorizontal: 16, justifyContent: 'center', borderRadius: 22, backgroundColor: '#262626' },
  pillText: { color: '#fff', fontSize: 15, fontWeight: '600' },
  title: { color: '#fff', fontSize: 24, fontWeight: '600', flex: 1 },
  primary: { minHeight: 48, paddingHorizontal: 22, justifyContent: 'center', borderRadius: 24, backgroundColor: '#2c3a5c' },
  primaryText: { color: '#c7d6ff', fontSize: 15, fontWeight: '600' },
  pressed: { opacity: 0.7 },
  muted: { color: '#9a9a9a', fontSize: 14, fontVariant: ['tabular-nums'] },
  notice: { color: '#e9c349', fontSize: 14, paddingHorizontal: 16, paddingBottom: 8 },
  grid: { flex: 1, paddingHorizontal: 12 },
  empty: { color: '#9a9a9a', fontSize: 16, padding: 24 },
  cell: { flex: 1, aspectRatio: 1, padding: 4 },
  thumbFrame: { flex: 1, borderRadius: 6, overflow: 'hidden', backgroundColor: '#1f1f1f' },
  thumb: { flex: 1 },
  approved: {
    position: 'absolute',
    top: 5,
    right: 5,
    width: 24,
    height: 24,
    borderRadius: 12,
    overflow: 'hidden',
    textAlign: 'center',
    lineHeight: 24,
    fontSize: 14,
    backgroundColor: '#e9c349',
    color: '#131313',
    fontWeight: '700',
  },
  labels: { position: 'absolute', top: 6, left: 6, flexDirection: 'row', gap: 3 },
  label: { width: 10, height: 10, borderRadius: 5 },
  stars: { position: 'absolute', bottom: 4, left: 6, color: '#e9c349', fontSize: 11, textShadowRadius: 3, textShadowColor: '#000' },
  footer: { flexDirection: 'row', alignItems: 'center', gap: 4, paddingHorizontal: 16, minHeight: 64, borderTopWidth: 1, borderTopColor: '#262626' },
  stats: { color: '#9a9a9a', fontSize: 14, marginRight: 12 },
  statValue: { color: '#fff', fontWeight: '600' },
  statApproved: { color: '#e9c349', fontWeight: '600' },
  spacer: { flex: 1 },
  toggle: { minHeight: 40, paddingHorizontal: 14, justifyContent: 'center', borderRadius: 20, backgroundColor: '#1f1f1f' },
  toggleOn: { backgroundColor: '#2c3a5c' },
  toggleText: { color: '#ddd', fontSize: 14 },
  toggleTextOn: { color: '#c7d6ff', fontWeight: '600' },
  mode: { width: 32, height: 44, alignItems: 'center', justifyContent: 'center' },
  modeText: { color: '#9a9a9a', fontSize: 18, fontWeight: '700' },
  mark: { width: 40, height: 44, alignItems: 'center', justifyContent: 'center' },
  star: { color: '#555', fontSize: 22 },
  starOn: { color: '#e9c349' },
  dot: { width: 18, height: 18, borderRadius: 9, borderWidth: 2 },
  filterButton: { minHeight: 48, paddingHorizontal: 22, justifyContent: 'center', borderRadius: 24, backgroundColor: '#262626' },
  filterButtonOn: { backgroundColor: '#2c3a5c' },
  filterButtonText: { color: '#ddd', fontSize: 15, fontWeight: '600' },
  filterButtonTextOn: { color: '#c7d6ff' },
  panel: {
    position: 'absolute',
    right: 16,
    bottom: 76,
    width: 440,
    padding: 16,
    gap: 6,
    borderRadius: 14,
    backgroundColor: '#1d1d1d',
    borderWidth: 1,
    borderColor: '#2a2a2a',
  },
  panelRow: { flexDirection: 'row', alignItems: 'center', gap: 6 },
  wrap: { flexWrap: 'wrap' },
  panelLabel: { color: '#9a9a9a', fontSize: 13, marginTop: 8 },
  clear: { alignSelf: 'flex-end', minHeight: 40, justifyContent: 'center', marginTop: 4 },
  clearText: { color: '#c7d6ff', fontSize: 14 },
});
