// Visor de culling (spec `tablet-culling`): la foto a todo el ancho y una tira
// de miniaturas abajo, para darle a la foto el mayor espacio posible. Aprobar y
// quitar la aprobación son solo gestos (ver `Viewer`), con una guía la primera
// vez y "Deshacer" en un snackbar. Estrellas (izquierda) y etiquetas de color
// (derecha) van sobre la foto, justo encima de la tira; un toque en la foto
// activa el modo limpio, que oculta la tira, las estrellas y las etiquetas.
// Cada cambio se guarda en el proyecto por `CullingBackend`.
import { FlashList, type FlashListRef } from '@shopify/flash-list';
import { Image } from 'expo-image';
import * as Haptics from 'expo-haptics';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';

import type { PhotoInfo } from '@lumik/core-native';

import { GestureGuide } from './GestureGuide';
import { Icon, type IconName } from './Icon';
import { PhotoInfoPanel } from './PhotoInfoPanel';
import { readPrefs, savePrefs } from './prefs';
import { Viewer } from './Viewer';

const THUMB = 72;
// Mismos colores que `@lumik/ui` (PhotoCard) y la barra lateral del desktop.
const LABEL_COLORS = ['#C0392B', '#E9C349', '#27AE60', '#4B7FE8', '#9B59B6'];
const UNDO_MS = 4000;

// `labels[i]`: etiquetas de color (1–5) de la foto i; puede tener varias.
type Marks = { approved: boolean[]; rating: number[]; labels: number[][] };
type Undo = { label: string; previous: Marks; revert?: () => void };

export type CullItem = {
  info: PhotoInfo;
  thumb: string;
  /** Lo que muestra el visor (la vista previa, o la miniatura mientras llega). */
  preview: string;
  /** `preview` ya trae la rotación aplicada (la miniatura): el visor no la gira otra vez. */
  previewRotated?: boolean;
  /** Archivo sin `file://` para el histograma. */
  histPath: string;
};

/** Dónde se guarda cada cambio. Si un método lanza, el cambio no se aplica. */
export type CullingBackend = {
  rate(index: number, stars: number, labels: number[]): void;
  approve(index: number, approved: boolean): void;
  /** Gira la foto `delta` grados (±90). */
  rotate?(index: number, delta: number): void;
  /** La foto actual cambió: momento de traer las vistas previas cercanas. */
  onIndex?(index: number): void;
};

// "1,3,5" (formato del desktop) → [1, 3, 5]
const parseLabels = (value?: string) =>
  (value ?? '')
    .split(',')
    .map(Number)
    .filter((n) => n >= 1 && n <= 5);

type CullingProps = {
  photos: CullItem[];
  onCleanChange: (clean: boolean) => void;
  backend?: CullingBackend;
  /** Regresar (al grid del proyecto); sin él no hay botón. */
  onBack?: () => void;
  backLabel?: string;
  /** Foto con la que abre el visor. */
  initialIndex?: number;
  /** Importar al proyecto (como en el desktop, desde dentro); sin él no hay botón. */
  onImport?: () => void;
  /** Aviso fijo arriba (p. ej. proyecto en solo lectura) con una acción. */
  notice?: { text: string; detail?: string; warn?: boolean; actionLabel: string; onAction: () => void };
};

export function Culling({ photos, onCleanChange, backend, onBack, backLabel = 'Proyectos', initialIndex = 0, onImport, notice }: CullingProps) {
  const count = photos.length;
  const [index, setIndex] = useState(() => Math.min(Math.max(0, initialIndex), Math.max(0, count - 1)));
  const [marks, setMarks] = useState<Marks>(() => ({
    approved: photos.map((p) => p.info.culled),
    rating: photos.map((p) => p.info.stars),
    labels: photos.map((p) => parseLabels(p.info.colorLabel)),
  }));
  const [error, setError] = useState<string | null>(null);
  // El visor se abre como se dejó la última vez.
  const [clean, setClean] = useState(() => readPrefs().viewer?.clean ?? false);
  const [showInfo, setShowInfo] = useState(() => readPrefs().viewer?.showInfo ?? true);
  useEffect(() => savePrefs({ viewer: { clean, showInfo } }), [clean, showInfo]);
  // La guía se abre sola la primera vez; después, con "?".
  const [guideStep, setGuideStep] = useState<number | null>(() => (readPrefs().guideSeen ? null : 0));
  const closeGuide = useCallback(() => {
    setGuideStep(null);
    savePrefs({ guideSeen: true });
  }, []);
  const [undo, setUndo] = useState<Undo | null>(null);
  const stripRef = useRef<FlashListRef<CullItem>>(null);
  const items = useMemo(() => photos.map((p) => ({ uri: p.preview, rotation: p.previewRotated ? 0 : p.info.rotation })), [photos]);

  // El visor siempre tiene la vecina siguiente y la anterior en memoria.
  useEffect(() => {
    backend?.onIndex?.(index);
    const around = [index - 1, index + 1].filter((i) => i >= 0 && i < count);
    Image.prefetch(around.map((i) => photos[i].preview), 'memory');
  }, [index, photos, count, backend]);

  useEffect(() => {
    if (!error) return;
    const t = setTimeout(() => setError(null), 6000);
    return () => clearTimeout(t);
  }, [error]);

  // La tira sigue a la foto actual, centrada.
  useEffect(() => {
    stripRef.current?.scrollToIndex({ index, animated: true, viewPosition: 0.5 });
  }, [index]);

  // El snackbar se va solo.
  useEffect(() => {
    if (!undo) return;
    const t = setTimeout(() => setUndo(null), UNDO_MS);
    return () => clearTimeout(t);
  }, [undo]);

  const step = useCallback(
    (delta: 1 | -1) => setIndex((i) => Math.min(count - 1, Math.max(0, i + delta))),
    [count],
  );

  // Primero se guarda (`persist`); si falla, la marca no cambia. `revert`
  // deshace lo guardado si se toca "Deshacer".
  const change = useCallback((label: string, edit: (m: Marks) => Marks, persist?: () => void, revert?: () => void) => {
    try {
      persist?.();
    } catch (e) {
      Haptics.notificationAsync(Haptics.NotificationFeedbackType.Error);
      setError(String(e));
      return;
    }
    Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light);
    setMarks((m) => {
      setUndo({ label, previous: m, revert });
      return edit(m);
    });
  }, []);

  const approve = useCallback(
    (approved: boolean) => {
      if (marks.approved[index] === approved) return;
      const i = index;
      change(
        approved ? 'Aprobada' : 'Aprobación quitada',
        (m) => {
          const next = m.approved.slice();
          next[i] = approved;
          return { ...m, approved: next };
        },
        backend && (() => backend.approve(i, approved)),
        backend && (() => backend.approve(i, !approved)),
      );
    },
    [change, index, marks.approved, backend],
  );

  const toggleApprove = useCallback(() => approve(!marks.approved[index]), [approve, index, marks.approved]);

  const rate = useCallback(
    (stars: number) => {
      const i = index;
      const before = marks.rating[i];
      const labels = marks.labels[i];
      const value = before === stars ? 0 : stars;
      change(
        value ? `${value} ${value === 1 ? 'estrella' : 'estrellas'}` : 'Sin estrellas',
        (m) => {
          const next = m.rating.slice();
          next[i] = value;
          return { ...m, rating: next };
        },
        backend && (() => backend.rate(i, value, labels)),
        backend && (() => backend.rate(i, before, labels)),
      );
    },
    [change, index, marks.rating, marks.labels, backend],
  );

  const toggleLabel = useCallback(
    (label: number) => {
      const i = index;
      const before = marks.labels[i];
      const stars = marks.rating[i];
      const has = before.includes(label);
      const after = has ? before.filter((l) => l !== label) : [...before, label].sort();
      change(
        has ? 'Etiqueta quitada' : 'Etiqueta puesta',
        (m) => {
          const next = m.labels.slice();
          next[i] = after;
          return { ...m, labels: next };
        },
        backend && (() => backend.rate(i, stars, after)),
        backend && (() => backend.rate(i, stars, before)),
      );
    },
    [change, index, marks.labels, marks.rating, backend],
  );

  const rotate = useCallback(
    (delta: number) => {
      try {
        backend?.rotate?.(index, delta);
      } catch (e) {
        setError(String(e));
      }
    },
    [backend, index],
  );

  const toggleClean = useCallback(() => setClean((c) => !c), []);
  useEffect(() => onCleanChange(clean), [clean, onCleanChange]);

  const approvedCount = marks.approved.filter(Boolean).length;
  const isApproved = marks.approved[index];

  return (
    <View style={styles.root}>
      <View style={styles.viewerPane}>
        <Viewer
          items={items}
          index={index}
          onApprove={approve}
          onToggleApprove={toggleApprove}
          onStep={step}
          onTap={toggleClean}
        />

        {showInfo && guideStep === null && (
          <PhotoInfoPanel info={photos[index].info} previewPath={photos[index].histPath} />
        )}

        {backend?.rotate && !clean && (
          <View style={styles.rotateBar} pointerEvents="box-none">
            <RoundToggle icon="rotate-ccw" on={false} onPress={() => rotate(-90)} />
            <RoundToggle icon="rotate-cw" on={false} onPress={() => rotate(90)} />
          </View>
        )}

        <View style={styles.topBar} pointerEvents="box-none">
          <View style={styles.topLeft}>
            {onBack && (
              <Pressable style={({ pressed }) => [styles.navPill, styles.navPillBack, pressed && styles.navPillPressed]} onPress={onBack} hitSlop={8}>
                <Icon name="chevron-left" size={18} />
                <Text style={styles.navPillText}>{backLabel}</Text>
              </Pressable>
            )}
            <Text style={styles.chip}>
              {index + 1} / {count}
            </Text>
            {isApproved && <Text style={styles.badge}>Aprobada</Text>}
            <Text style={styles.perfText}>
              {photos[index].info.fileName} · {approvedCount} aprobadas
            </Text>
          </View>
          <View style={styles.topRight}>
            {onImport && (
              <Pressable style={({ pressed }) => [styles.navPill, pressed && styles.navPillPressed]} onPress={onImport} hitSlop={8}>
                <Text style={styles.navPillText}>Importar</Text>
              </Pressable>
            )}
            <RoundToggle icon="help" on={guideStep !== null} onPress={() => setGuideStep(guideStep === null ? 0 : null)} />
            <RoundToggle icon="info" on={showInfo} onPress={() => setShowInfo((v) => !v)} />
          </View>
        </View>

        {notice && (
          <View style={[styles.notice, notice.warn && styles.noticeWarn]}>
            <View style={styles.noticeTexts}>
              <Text style={styles.noticeText}>{notice.text}</Text>
              {notice.detail && <Text style={styles.noticeDetail}>{notice.detail}</Text>}
            </View>
            <Pressable style={styles.noticeAction} onPress={notice.onAction}>
              <Text style={styles.noticeActionText}>{notice.actionLabel}</Text>
            </Pressable>
          </View>
        )}

        {!clean && (
          <View style={styles.marksRow} pointerEvents="box-none">
            <View style={styles.pill}>
              {[1, 2, 3, 4, 5].map((s) => (
                <Pressable key={s} style={styles.mark} onPress={() => rate(s)}>
                  <Icon name="star-filled" size={24} color={s <= marks.rating[index] ? '#e9c349' : '#666'} />
                </Pressable>
              ))}
            </View>
            <View style={styles.pill}>
              {LABEL_COLORS.map((color, i) => {
                const on = marks.labels[index].includes(i + 1);
                return (
                  <Pressable key={color} style={styles.mark} onPress={() => toggleLabel(i + 1)}>
                    <View style={[styles.dot, { borderColor: color }, on && { backgroundColor: color }]} />
                  </Pressable>
                );
              })}
            </View>
          </View>
        )}

        {undo && guideStep === null && (
          <View style={[styles.snackbar, clean && styles.snackbarClean]}>
            <Text style={styles.snackText}>{undo.label}</Text>
            <Pressable
              onPress={() => {
                try {
                  undo.revert?.();
                  setMarks(undo.previous);
                } catch (e) {
                  setError(String(e));
                }
                setUndo(null);
              }}
              style={styles.snackAction}
            >
              <Text style={styles.snackActionText}>DESHACER</Text>
            </Pressable>
          </View>
        )}

        {error && (
          <View style={[styles.snackbar, styles.snackbarError, clean && styles.snackbarClean]}>
            <Text style={styles.snackErrorText} numberOfLines={3}>
              {error}
            </Text>
          </View>
        )}

        {guideStep !== null && <GestureGuide step={guideStep} onStep={setGuideStep} onClose={closeGuide} />}
      </View>

      {!clean && (
        <View style={styles.strip}>
          <FlashList
            ref={stripRef}
            data={photos}
            horizontal
            extraData={{ index, marks }}
            keyExtractor={(p, i) => `${i}-${p.info.id}`}
            showsHorizontalScrollIndicator={false}
            initialScrollIndex={index}
            renderItem={({ item, index: i }) => (
              <Thumb
                photo={item}
                index={i}
                selected={i === index}
                approved={marks.approved[i]}
                rating={marks.rating[i]}
                labels={marks.labels[i]}
                onPress={setIndex}
              />
            )}
          />
        </View>
      )}
    </View>
  );
}

type ThumbProps = {
  photo: CullItem;
  index: number;
  selected: boolean;
  approved: boolean;
  rating: number;
  labels: number[];
  onPress: (index: number) => void;
};

function Thumb({ photo, index, selected, approved, rating, labels, onPress }: ThumbProps) {
  return (
    <Pressable style={styles.cell} onPress={() => onPress(index)}>
      <View style={[styles.thumbFrame, selected && styles.thumbSelected]}>
        <Image source={{ uri: photo.thumb }} recyclingKey={photo.thumb} style={styles.thumb} contentFit="cover" transition={0} />
        {approved && (
          <View style={styles.thumbApproved}>
            <Icon name="check" size={14} strokeWidth={3} color="#131313" />
          </View>
        )}
        {rating > 0 && (
          <View style={styles.thumbRating}>
            {Array.from({ length: rating }, (_, k) => (
              <Icon key={k} name="star-filled" size={9} color="#e9c349" />
            ))}
          </View>
        )}
        {labels.length > 0 && (
          <View style={styles.thumbLabels}>
            {labels.map((l) => (
              <View key={l} style={[styles.thumbLabel, { backgroundColor: LABEL_COLORS[l - 1] }]} />
            ))}
          </View>
        )}
      </View>
    </Pressable>
  );
}

function RoundToggle({ icon, on, onPress }: { icon: IconName; on: boolean; onPress: () => void }) {
  return (
    <Pressable style={[styles.round, on && styles.roundOn]} onPress={onPress} hitSlop={6}>
      <Icon name={icon} size={22} color={on ? '#131313' : '#fff'} />
    </Pressable>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#131313' },
  viewerPane: { flex: 1 },
  topBar: { position: 'absolute', top: 12, left: 16, right: 16, flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between' },
  topLeft: { flexDirection: 'row', alignItems: 'center', gap: 8 },
  topRight: { flexDirection: 'row', alignItems: 'center', gap: 10 },
  // Girar: a la izquierda, centrado en vertical sobre la foto.
  rotateBar: { position: 'absolute', left: 16, top: 0, bottom: 0, justifyContent: 'center', gap: 10 },
  chip: { color: '#ddd', fontSize: 15, backgroundColor: 'rgba(19,19,19,0.7)', paddingHorizontal: 10, paddingVertical: 4, borderRadius: 8 },
  badge: { color: '#131313', backgroundColor: '#e9c349', fontWeight: '700', paddingHorizontal: 10, paddingVertical: 4, borderRadius: 8 },
  navPillBack: { flexDirection: 'row', alignItems: 'center', gap: 4, paddingLeft: 10 },
  navPill: { minHeight: 44, paddingHorizontal: 16, justifyContent: 'center', borderRadius: 22, backgroundColor: 'rgba(19,19,19,0.8)' },
  navPillPressed: { opacity: 0.7 },
  navPillText: { color: '#fff', fontSize: 15, fontWeight: '600' },
  round: { width: 44, height: 44, borderRadius: 22, alignItems: 'center', justifyContent: 'center', backgroundColor: 'rgba(19,19,19,0.7)' },
  roundOn: { backgroundColor: '#e9e9e9' },
  strip: { height: THUMB + 8, borderTopWidth: 1, borderTopColor: '#262626' },
  cell: { width: THUMB, height: THUMB + 8, padding: 3 },
  thumbFrame: { flex: 1, borderRadius: 6, overflow: 'hidden', borderWidth: 3, borderColor: 'transparent' },
  thumbSelected: { borderColor: '#e9e9e9' },
  thumb: { flex: 1, backgroundColor: '#1f1f1f' },
  thumbApproved: {
    position: 'absolute',
    top: 3,
    right: 3,
    width: 22,
    height: 22,
    borderRadius: 11,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: '#e9c349',
  },
  thumbRating: { position: 'absolute', bottom: 3, left: 4, flexDirection: 'row', gap: 1 },
  thumbLabels: { position: 'absolute', top: 3, left: 3, flexDirection: 'row', gap: 2 },
  thumbLabel: { width: 8, height: 8, borderRadius: 4 },
  marksRow: { position: 'absolute', left: 12, right: 12, bottom: 10, flexDirection: 'row', justifyContent: 'space-between' },
  pill: { flexDirection: 'row', borderRadius: 22, backgroundColor: 'rgba(19,19,19,0.7)' },
  mark: { width: 44, height: 44, alignItems: 'center', justifyContent: 'center' },
  dot: { width: 20, height: 20, borderRadius: 10, borderWidth: 2 },
  perfText: { color: '#888', fontSize: 11, fontVariant: ['tabular-nums'], backgroundColor: 'rgba(19,19,19,0.6)', paddingHorizontal: 6, paddingVertical: 2, borderRadius: 6 },
  snackbar: {
    position: 'absolute',
    alignSelf: 'center',
    width: 420,
    bottom: 66,
    minHeight: 52,
    borderRadius: 8,
    paddingLeft: 20,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: '#e9e9e9',
  },
  snackbarClean: { bottom: 20 },
  notice: {
    position: 'absolute',
    top: 68,
    left: 16,
    right: 16,
    flexDirection: 'row',
    alignItems: 'center',
    gap: 16,
    padding: 14,
    borderRadius: 12,
    backgroundColor: 'rgba(28,40,64,0.95)',
  },
  noticeWarn: { backgroundColor: 'rgba(72,48,12,0.95)' },
  noticeTexts: { flex: 1, gap: 2 },
  noticeText: { color: '#fff', fontSize: 16, fontWeight: '600' },
  noticeDetail: { color: '#ddd', fontSize: 14 },
  noticeAction: { minHeight: 48, paddingHorizontal: 18, justifyContent: 'center', borderRadius: 24, backgroundColor: '#e9e9e9' },
  noticeActionText: { color: '#131313', fontSize: 15, fontWeight: '700' },
  snackbarError: { backgroundColor: '#3a1414', paddingRight: 20 },
  snackErrorText: { color: '#fca5a5', fontSize: 14, flexShrink: 1, paddingVertical: 10 },
  snackText: { color: '#131313', fontSize: 16 },
  snackAction: { minHeight: 52, paddingHorizontal: 20, justifyContent: 'center' },
  snackActionText: { color: '#2563eb', fontSize: 15, fontWeight: '700' },
});
