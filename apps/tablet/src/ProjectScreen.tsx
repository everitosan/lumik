// Vista del proyecto (como `ProjectDetail` del desktop): las fotos agrupadas
// por día o en cuadrícula, con las tarjetas del desktop (nombre, formato, fecha,
// estrellas y colores) y su menú (mostrar en carpeta, mover a la papelera), "Importar" y, abajo a
// la derecha, "Filtros" (aprobadas, estrellas ≥/=, colores y tags). Las fotos en la
// papelera están en la sección «Papelera» del inicio.
// Tocar una foto abre el visor, que recorre solo las que pasan los filtros.
import { FlashList } from '@shopify/flash-list';
import { Image } from 'expo-image';
import { useEffect, useMemo, useState } from 'react';
import { Modal, Pressable, StyleSheet, Text, useWindowDimensions, View } from 'react-native';

import type { PhotoInfo } from '@lumik/core-native';

import { Icon } from './Icon';

// Mismos colores que `@lumik/ui` (PhotoCard) y la barra lateral del desktop.
const LABEL_COLORS = ['#C0392B', '#E9C349', '#27AE60', '#4B7FE8', '#9B59B6'];
// Como el desktop: tarjetas de al menos 200 de ancho, foto 4:3 y nombre y datos.
const CARD_MIN = 200;
const GAP = 12;
const CARD_HEADER = 30;
const CARD_FOOTER = 52;
const SEPARATOR = 64;
const RAW_EXTENSIONS = ['cr2', 'cr3', 'nef', 'arw', 'raf', 'orf', 'rw2', 'dng'];
const JPEG_EXTENSIONS = ['jpg', 'jpeg'];

export type GridPhoto = { info: PhotoInfo; thumb: string };
export type ViewMode = 'by-date' | 'grid';

export type Filters = {
  approvedOnly: boolean;
  /** Ver la papelera en lugar de las fotos del proyecto (como el desktop). */
  minStars: number | null;
  starsMode: 'inclusive' | 'exact';
  colors: Set<number>;
  tags: Set<string>;
};

export const NO_FILTERS: Filters = { approvedOnly: false, minStars: null, starsMode: 'inclusive', colors: new Set(), tags: new Set() };

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
  Number(f.approvedOnly) + Number(f.minStars !== null) + Number(f.colors.size > 0) + Number(f.tags.size > 0);

/** Formato de la foto, como `photoFormat` de `@lumik/ui`. */
function photoFormat(info: PhotoInfo): string | null {
  if (info.cameraJpgPath) return 'RAW+JPG';
  const ext = info.dngPath.split('.').pop()?.toLowerCase() ?? '';
  if (RAW_EXTENSIONS.includes(ext)) return 'RAW';
  if (JPEG_EXTENSIONS.includes(ext)) return 'JPG';
  return null;
}

/** EXIF ("2024:11:07 13:59:07") o ISO → "2024-11-07". */
const captureDay = (raw: string) => raw.slice(0, 10).replace(/:/g, '-');

function dayLabel(day: string): string {
  if (day === 'sin-fecha') return 'Sin fecha';
  const d = new Date(`${day}T00:00:00`);
  if (isNaN(d.getTime())) return day;
  const label = d.toLocaleDateString('es-MX', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' });
  // Solo la inicial: "Domingo, 28 de junio de 2026".
  return label.charAt(0).toUpperCase() + label.slice(1);
}

/** Fecha y hora de la tarjeta, como `PhotoCard`. */
function dateAndTime(raw?: string): { date: string; time: string } {
  if (!raw) return { date: '—', time: '—' };
  const m = /^(\d{4})[:-](\d{2})[:-](\d{2})[ T](\d{2}):(\d{2})/.exec(raw);
  return m ? { date: `${m[1]}/${m[2]}/${m[3]}`, time: `${m[4]}:${m[5]}` } : { date: raw, time: '—' };
}

/** Una fila de la lista: el separador de un día o una fila de tarjetas. */
type Row =
  | { type: 'day'; key: string; label: string }
  | { type: 'photos'; key: string; cells: { index: number; position: number }[] };

/** Filas como el desktop: en "por fecha", cada día con su separador (sin fecha al final). */
function buildRows(source: GridPhoto[], visible: number[], columns: number, mode: ViewMode): Row[] {
  const groups: [string, { index: number; position: number }[]][] = [];
  if (mode === 'grid') {
    groups.push(['', visible.map((index, position) => ({ index, position }))]);
  } else {
    const byDay = new Map<string, { index: number; position: number }[]>();
    visible.forEach((index, position) => {
      const raw = source[index].info.captureDate;
      const day = raw ? captureDay(raw) : 'sin-fecha';
      const bucket = byDay.get(day);
      if (bucket) bucket.push({ index, position });
      else byDay.set(day, [{ index, position }]);
    });
    const days = [...byDay.keys()].sort((a, b) => (a === 'sin-fecha' ? 1 : b === 'sin-fecha' ? -1 : a.localeCompare(b)));
    for (const day of days) groups.push([day, byDay.get(day)!]);
  }
  const rows: Row[] = [];
  for (const [day, cells] of groups) {
    if (day) rows.push({ type: 'day', key: `d-${day}`, label: dayLabel(day) });
    for (let i = 0; i < cells.length; i += columns) {
      rows.push({ type: 'photos', key: `r-${day}-${i}`, cells: cells.slice(i, i + columns) });
    }
  }
  return rows;
}

type Props = {
  title: string;
  photos: GridPhoto[];
  filters: Filters;
  onFilters: (f: Filters) => void;
  viewMode: ViewMode;
  onViewMode: (mode: ViewMode) => void;
  /** Abre el visor en la foto `position` de la lista filtrada. */
  onOpenPhoto: (visible: number[], position: number) => void;
  /** Mover a la papelera (índice en `photos`); sin él (solo lectura), la tarjeta no lo ofrece. */
  onTrash?: (index: number) => void;
  /** Mostrar en carpeta: abre en la app de Archivos la carpeta de la foto (índice en la lista que se ve). */
  onReveal?: (index: number) => void;
  /** Mandar a la papelera las fotos marcadas (índices en `photos`); devuelve las que
   *  fallaron. Sin él (solo lectura) no hay modo marcar (spec photo-marking). */
  onTrashMany?: (indices: number[]) => Promise<{ name: string; reason: string }[]>;
  onBack: () => void;
  onImport?: () => void;
  /** Miniaturas que aún se copian al caché (`done/total`), o `null`. */
  fetching: { done: number; total: number } | null;
  /** Candado de otro dispositivo: el aviso y "Tomar el control", como el banner del desktop. */
  lock?: { text: string; onTakeControl: () => void };
};

export function ProjectScreen(props: Props) {
  const { title, photos, filters, onFilters, viewMode, onViewMode, onOpenPhoto, onTrash, onReveal, onTrashMany, onBack, onImport, fetching, lock } =
    props;
  const { width } = useWindowDimensions();
  const inner = width - 2 * 16;
  const columns = Math.max(2, Math.floor((inner + GAP) / (CARD_MIN + GAP)));
  const cardWidth = (inner - (columns - 1) * GAP) / columns;
  // Las fotos en la papelera no se ven aquí: están en la sección «Papelera» del inicio.
  const source = photos;
  const visible = useMemo(() => applyFilters(source, filters), [source, filters]);
  const rows = useMemo(() => buildRows(source, visible, columns, viewMode), [source, visible, columns, viewMode]);
  // Como el desktop: las aprobadas de lo que se ve, y los tags de la lista que se ve.
  const approved = useMemo(() => visible.filter((i) => source[i].info.culled).length, [source, visible]);
  const allTags = useMemo(() => [...new Set(source.flatMap((p) => splitList(p.info.tags)))].sort(), [source]);
  const [showPanel, setShowPanel] = useState(false);
  // Menú de la tarjeta y confirmación de la papelera.
  const [menu, setMenu] = useState<number | null>(null);
  // Modo marcar (spec photo-marking): `null` fuera del modo; si no, los índices marcados.
  const [marked, setMarked] = useState<Set<number> | null>(null);
  const [confirmMany, setConfirmMany] = useState(false);
  const [manyBusy, setManyBusy] = useState(false);
  const [manyFailed, setManyFailed] = useState<{ name: string; reason: string }[] | null>(null);
  // Las marcas no sobreviven a un cambio de filtros o de vista, ni a que cambie
  // el número de fotos (los índices dejarían de valer). Las miniaturas que llegan
  // al caché cambian `photos` sin cambiar su largo: esas no las borran.
  useEffect(() => setMarked(null), [filters, viewMode, photos.length]);
  const toggleMarked = (index: number) =>
    setMarked((m) => {
      const next = new Set(m ?? []);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  const [confirmTrash, setConfirmTrash] = useState<number | null>(null);
  const active = activeCount(filters);
  const set = (change: Partial<Filters>) => onFilters({ ...filters, ...change });
  const toggle = <T,>(s: Set<T>, v: T) => {
    const next = new Set(s);
    if (next.has(v)) next.delete(v);
    else next.add(v);
    return next;
  };
  const hasMenu = !!onReveal || !!onTrash;
  const menuPhoto = menu !== null ? source[menu] : null;

  const renderCard = (index: number, position: number) => {
    const p = source[index];
    const labels = splitList(p.info.colorLabel).map(Number);
    const format = photoFormat(p.info);
    const { date, time } = dateAndTime(p.info.captureDate);
    return (
      <Pressable
        key={p.info.id}
        style={({ pressed }) => [
          styles.card,
          { width: cardWidth },
          p.info.culled && styles.cardApproved,
          marked?.has(index) && styles.cardMarked,
          pressed && styles.pressed,
        ]}
        onPress={() => (marked ? toggleMarked(index) : onOpenPhoto(visible, position))}
        // El toque largo entra al modo marcar; el menú queda en el botón ⋮.
        onLongPress={onTrashMany ? () => setMarked((m) => m ?? new Set([index])) : hasMenu ? () => setMenu(index) : undefined}
      >
        <View style={styles.cardHeader}>
          <Text style={styles.fileName} numberOfLines={1}>
            {p.info.fileName}
          </Text>
          {hasMenu && (
            <Pressable style={styles.menuButton} onPress={() => setMenu(index)} hitSlop={10}>
              <Icon name="menu" size={18} color="#c2c6d7" />
            </Pressable>
          )}
        </View>
        <View style={[styles.thumbFrame, { height: cardWidth * 0.75 }]}>
          <Image source={{ uri: p.thumb }} recyclingKey={p.info.id} style={styles.thumb} contentFit="contain" transition={0} />
          {format && <Text style={styles.format}>{format}</Text>}
          {marked && (
            <View style={[styles.markBox, marked.has(index) && styles.markBoxOn]}>
              {marked.has(index) && <Icon name="check" size={16} color="#002d6f" />}
            </View>
          )}
        </View>
        <View style={styles.cardFooter}>
          <View style={styles.footerRow}>
            <Text style={styles.date}>{date}</Text>
            <View style={styles.cardStars}>
              {[1, 2, 3, 4, 5].map((n) => (
                <Icon key={n} name="star-filled" size={12} color={n <= p.info.stars ? '#e9c349' : '#424654'} />
              ))}
            </View>
          </View>
          <View style={styles.footerRow}>
            <Text style={styles.date}>{time}</Text>
            <View style={styles.labels}>
              {labels.map((l) => (
                <View key={l} style={[styles.label, { backgroundColor: LABEL_COLORS[l - 1] }]} />
              ))}
            </View>
          </View>
        </View>
      </Pressable>
    );
  };

  return (
    <View style={styles.root}>
      <View style={styles.header}>
        <Pressable style={({ pressed }) => [styles.pill, styles.pillBack, pressed && styles.pressed]} onPress={onBack} hitSlop={8}>
          <Icon name="chevron-left" size={18} />
          <Text style={styles.pillText}>Proyectos</Text>
        </Pressable>
        <Text style={styles.title} numberOfLines={1}>
          {title}
        </Text>
        {fetching && (
          <Text style={styles.muted}>
            Miniaturas {fetching.done}/{fetching.total}
          </Text>
        )}
        <View style={styles.segmented}>
          {(
            [
              ['by-date', 'Por fecha'],
              ['grid', 'Cuadrícula'],
            ] as const
          ).map(([mode, label]) => (
            <Pressable key={mode} style={[styles.segment, viewMode === mode && styles.segmentOn]} onPress={() => onViewMode(mode)}>
              <Text style={[styles.segmentText, viewMode === mode && styles.segmentTextOn]}>{label}</Text>
            </Pressable>
          ))}
        </View>
        {onImport && (
          <Pressable style={({ pressed }) => [styles.primary, pressed && styles.pressed]} onPress={onImport}>
            <Text style={styles.primaryText}>Importar</Text>
          </Pressable>
        )}
      </View>
      {lock && (
        <View style={styles.banner}>
          <Text style={styles.bannerText}>{lock.text}</Text>
          <Pressable style={({ pressed }) => [styles.bannerButton, pressed && styles.pressed]} onPress={lock.onTakeControl}>
            <Text style={styles.bannerButtonText}>Tomar el control</Text>
          </Pressable>
        </View>
      )}

      <View style={styles.grid}>
        {visible.length === 0 ? (
          <Text style={styles.empty}>
            {photos.length ? 'Ninguna foto pasa los filtros.' : 'Este proyecto no tiene fotos aún.'}
          </Text>
        ) : (
          <FlashList
            data={rows}
            extraData={source}
            keyExtractor={(r) => r.key}
            getItemType={(r) => r.type}
            renderItem={({ item: r }) =>
              r.type === 'day' ? (
                <View style={styles.day}>
                  <Text style={styles.dayText}>{r.label}</Text>
                </View>
              ) : (
                <View style={styles.row}>{r.cells.map((c) => renderCard(c.index, c.position))}</View>
              )
            }
          />
        )}
      </View>

      {/* En el modo marcar, la barra de acciones ocupa el lugar del pie. */}
      {marked && (
        <View style={styles.footer}>
          <Text style={styles.markedCount}>
            {marked.size === 1 ? '1 foto marcada' : `${marked.size} fotos marcadas`}
          </Text>
          <View style={styles.spacer} />
          <Pressable
            style={({ pressed }) => [styles.dangerButton, pressed && styles.pressed, marked.size === 0 && styles.off]}
            disabled={marked.size === 0}
            onPress={() => {
              setManyFailed(null);
              setConfirmMany(true);
            }}
          >
            <Text style={styles.dangerButtonText}>Mover a la papelera</Text>
          </Pressable>
          <Pressable style={({ pressed }) => [styles.filterButton, pressed && styles.pressed]} onPress={() => setMarked(null)}>
            <Text style={styles.filterButtonText}>Listo</Text>
          </Pressable>
        </View>
      )}

      {/* Pie: contadores a la izquierda; los filtros, abajo a la derecha. */}
      <View style={[styles.footer, marked && styles.hidden]}>
        <Text style={styles.stats}>
          <Text style={styles.statValue}>{photos.length.toLocaleString('es-MX')}</Text> fotos ·{' '}
          <Text style={styles.statApproved}>{approved}</Text> aprobadas
          {visible.length !== photos.length ? `  ·  ${visible.length} visibles` : ''}
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
                <Icon name="star-filled" size={24} color={filters.minStars !== null && n <= filters.minStars ? '#e9c349' : '#555'} />
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

      {menuPhoto && menu !== null && (
        <Modal transparent animationType="fade" onRequestClose={() => setMenu(null)}>
          <Pressable style={styles.scrim} onPress={() => setMenu(null)}>
            <View style={styles.menu}>
              <Text style={styles.menuTitle} numberOfLines={1}>
                {menuPhoto.info.fileName}
              </Text>
              {onReveal && (
                <Pressable
                  style={({ pressed }) => [styles.menuItem, pressed && styles.pressed]}
                  onPress={() => {
                    setMenu(null);
                    onReveal(menu);
                  }}
                >
                  <Text style={styles.menuItemText}>Mostrar en carpeta</Text>
                </Pressable>
              )}
              {onTrash && (
                <Pressable
                  style={({ pressed }) => [styles.menuItem, pressed && styles.pressed]}
                  onPress={() => {
                    setMenu(null);
                    setConfirmTrash(menu);
                  }}
                >
                  <Text style={[styles.menuItemText, styles.danger]}>Mover a la papelera</Text>
                </Pressable>
              )}
            </View>
          </Pressable>
        </Modal>
      )}

      {confirmMany && (
        <Modal transparent animationType="fade" onRequestClose={() => {
          if (!manyBusy) setConfirmMany(false);
        }}>
          <View style={styles.scrim}>
            <View style={styles.sheet}>
              <Text style={styles.sheetTitle}>Mover fotos a la papelera</Text>
              {manyFailed ? (
                <>
                  <Text style={styles.sheetText}>
                    {manyFailed.length === 1 ? 'No se pudo mover 1 foto:' : `No se pudieron mover ${manyFailed.length} fotos:`}
                  </Text>
                  {manyFailed.map((f, i) => (
                    <Text key={i} style={styles.muted}>
                      {f.name}: {f.reason}
                    </Text>
                  ))}
                </>
              ) : (
                <>
                  <Text style={styles.sheetText}>
                    {marked?.size === 1
                      ? 'La foto marcada se moverá a la papelera del proyecto (_trash/).'
                      : `Las ${marked?.size ?? 0} fotos marcadas se moverán a la papelera del proyecto (_trash/).`}
                  </Text>
                  <Text style={styles.muted}>Podrás restaurarlas o eliminarlas definitivamente desde la sección «Papelera» del inicio.</Text>
                </>
              )}
              <View style={styles.sheetButtons}>
                <Pressable style={styles.textButton} onPress={() => setConfirmMany(false)} disabled={manyBusy}>
                  <Text style={styles.textButtonText}>{manyFailed ? 'Cerrar' : 'Cancelar'}</Text>
                </Pressable>
                {!manyFailed && (
                  <Pressable
                    style={({ pressed }) => [styles.dangerButton, pressed && styles.pressed, manyBusy && styles.off]}
                    disabled={manyBusy}
                    onPress={async () => {
                      if (!onTrashMany || !marked) return;
                      setManyBusy(true);
                      const failed = await onTrashMany([...marked]);
                      setManyBusy(false);
                      setMarked(null);
                      if (failed.length > 0) setManyFailed(failed);
                      else setConfirmMany(false);
                    }}
                  >
                    <Text style={styles.dangerButtonText}>{manyBusy ? 'Moviendo…' : 'Mover a la papelera'}</Text>
                  </Pressable>
                )}
              </View>
            </View>
          </View>
        </Modal>
      )}

      {confirmTrash !== null && photos[confirmTrash] && (
        <Modal transparent animationType="fade" onRequestClose={() => setConfirmTrash(null)}>
          <View style={styles.scrim}>
            <View style={styles.sheet}>
              <Text style={styles.sheetTitle}>Mover foto a la papelera</Text>
              <Text style={styles.sheetText}>«{photos[confirmTrash].info.fileName}» se moverá a la papelera del proyecto (_trash/).</Text>
              <Text style={styles.muted}>Podrás restaurarla o eliminarla definitivamente desde la sección «Papelera» del inicio.</Text>
              <View style={styles.sheetButtons}>
                <Pressable style={styles.textButton} onPress={() => setConfirmTrash(null)}>
                  <Text style={styles.textButtonText}>Cancelar</Text>
                </Pressable>
                <Pressable
                  style={({ pressed }) => [styles.dangerButton, pressed && styles.pressed]}
                  onPress={() => {
                    const i = confirmTrash;
                    setConfirmTrash(null);
                    onTrash?.(i);
                  }}
                >
                  <Text style={styles.dangerButtonText}>Mover a la papelera</Text>
                </Pressable>
              </View>
            </View>
          </View>
        </Modal>
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
  pillBack: { flexDirection: 'row', alignItems: 'center', gap: 4, paddingLeft: 10 },
  pill: { minHeight: 44, paddingHorizontal: 16, justifyContent: 'center', borderRadius: 22, backgroundColor: '#262626' },
  pillText: { color: '#fff', fontSize: 15, fontWeight: '600' },
  title: { color: '#fff', fontSize: 24, fontWeight: '600', flex: 1 },
  primary: { minHeight: 48, paddingHorizontal: 22, justifyContent: 'center', borderRadius: 24, backgroundColor: '#2c3a5c' },
  primaryText: { color: '#c7d6ff', fontSize: 15, fontWeight: '600' },
  pressed: { opacity: 0.7 },
  muted: { color: '#9a9a9a', fontSize: 14, fontVariant: ['tabular-nums'] },
  grid: { flex: 1, paddingHorizontal: 16 },
  empty: { color: '#9a9a9a', fontSize: 16, padding: 24 },
  banner: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 16,
    marginHorizontal: 16,
    marginBottom: 8,
    paddingHorizontal: 16,
    paddingVertical: 8,
    borderRadius: 10,
    backgroundColor: '#2a2414',
  },
  bannerText: { color: '#e9c349', fontSize: 14, flex: 1 },
  bannerButton: { minHeight: 40, paddingHorizontal: 16, justifyContent: 'center', borderRadius: 20, backgroundColor: '#e9c349' },
  bannerButtonText: { color: '#131313', fontSize: 14, fontWeight: '700' },
  segmented: { flexDirection: 'row', borderRadius: 22, backgroundColor: '#1f1f1f', padding: 3 },
  segment: { minHeight: 42, paddingHorizontal: 16, justifyContent: 'center', borderRadius: 20 },
  segmentOn: { backgroundColor: '#333' },
  segmentText: { color: '#9a9a9a', fontSize: 14 },
  segmentTextOn: { color: '#fff', fontWeight: '600' },
  day: { height: SEPARATOR, justifyContent: 'flex-end', paddingBottom: 10, marginBottom: GAP, borderBottomWidth: 1, borderBottomColor: '#333' },
  dayText: { color: '#c2c6d7', fontSize: 17, fontWeight: '600' },
  row: { flexDirection: 'row', gap: GAP, paddingBottom: GAP },
  card: { borderRadius: 8, borderWidth: 1, borderColor: '#424654', backgroundColor: '#201f1f', overflow: 'hidden' },
  cardApproved: { borderWidth: 2, borderColor: '#3a4a6b' },
  cardMarked: { borderWidth: 2, borderColor: '#b0c6ff' },
  markBox: {
    position: 'absolute',
    top: 8,
    left: 8,
    width: 26,
    height: 26,
    borderRadius: 7,
    borderWidth: 2,
    borderColor: 'rgba(255,255,255,0.85)',
    backgroundColor: 'rgba(0,0,0,0.45)',
    alignItems: 'center',
    justifyContent: 'center',
  },
  markBoxOn: { backgroundColor: '#b0c6ff', borderColor: '#b0c6ff' },
  markedCount: { color: '#b0c6ff', fontSize: 16, fontWeight: '600' },
  hidden: { display: 'none' },
  off: { opacity: 0.4 },
  cardHeader: { height: CARD_HEADER, flexDirection: 'row', alignItems: 'center', paddingLeft: 8, borderBottomWidth: 1, borderBottomColor: '#424654' },
  fileName: { flex: 1, color: '#c2c6d7', fontSize: 12, fontFamily: 'monospace' },
  menuButton: { width: 36, height: CARD_HEADER, alignItems: 'center', justifyContent: 'center' },
  thumbFrame: { backgroundColor: '#0e0e0e' },
  thumb: { flex: 1 },
  format: {
    position: 'absolute',
    top: 6,
    right: 6,
    paddingHorizontal: 6,
    paddingVertical: 2,
    borderRadius: 4,
    overflow: 'hidden',
    backgroundColor: 'rgba(0,0,0,0.65)',
    color: '#e5e2e1',
    fontSize: 10,
    fontWeight: '700',
    fontFamily: 'monospace',
  },
  cardFooter: { height: CARD_FOOTER, paddingHorizontal: 8, justifyContent: 'center', gap: 4, borderTopWidth: 1, borderTopColor: '#424654' },
  footerRow: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between' },
  date: { color: '#8c90a0', fontSize: 12, fontFamily: 'monospace' },
  cardStars: { flexDirection: 'row', gap: 1 },
  labels: { flexDirection: 'row', gap: 4 },
  label: { width: 12, height: 12, borderRadius: 6 },
  scrim: { flex: 1, alignItems: 'center', justifyContent: 'center', backgroundColor: 'rgba(0,0,0,0.6)' },
  menu: { width: 360, paddingVertical: 8, borderRadius: 14, backgroundColor: '#1d1d1d' },
  menuTitle: { color: '#9a9a9a', fontSize: 13, paddingHorizontal: 20, paddingVertical: 10, fontFamily: 'monospace' },
  menuItem: { minHeight: 52, paddingHorizontal: 20, justifyContent: 'center' },
  menuItemText: { color: '#fff', fontSize: 16 },
  // #ffb4ab sobre #1d1d1d = 9.93:1 (AAA); #f87171 daba 6.09:1.
  danger: { color: '#ffb4ab' },
  sheet: { width: 520, padding: 24, gap: 8, borderRadius: 16, backgroundColor: '#1d1d1d' },
  sheetTitle: { color: '#fff', fontSize: 22, fontWeight: '600', marginBottom: 8 },
  sheetText: { color: '#ddd', fontSize: 15 },
  sheetButtons: { flexDirection: 'row', justifyContent: 'flex-end', alignItems: 'center', gap: 12, marginTop: 16 },
  textButton: { minHeight: 48, paddingHorizontal: 16, justifyContent: 'center' },
  textButtonText: { color: '#c7d6ff', fontSize: 15 },
  dangerButton: { minHeight: 48, paddingHorizontal: 22, justifyContent: 'center', borderRadius: 24, backgroundColor: '#7f1d1d' },
  // Blanco sobre #7f1d1d = 10.02:1 (AAA); #fecaca daba 6.93:1.
  dangerButtonText: { color: '#fff', fontSize: 15, fontWeight: '600' },
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
