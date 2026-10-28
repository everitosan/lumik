// Experimento 2.4 del spike: visor a todo el ancho y una tira de miniaturas
// abajo, para darle a la foto el mayor espacio posible. Aprobar y quitar la
// aprobación son solo gestos (ver `Viewer`), con una guía la primera vez y
// "Deshacer" en un snackbar. Estrellas (izquierda) y etiquetas de color
// (derecha) van sobre la foto, justo encima de la tira; un toque en la foto
// activa el modo limpio, que oculta la tira, las estrellas y las etiquetas.
// Las fotos y sus datos de captura vienen de un `project.db` real: en modo
// demo (2 000 en la sandbox, sin guardar) o de un proyecto del disco, que
// guarda cada cambio por `CullingBackend`.
import { FlashList, type FlashListRef, useBenchmark } from '@shopify/flash-list';
import { Image } from 'expo-image';
import * as Haptics from 'expo-haptics';
import { type RefObject, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ActivityIndicator, Pressable, StyleSheet, Text, View } from 'react-native';
import { useFrameCallback, useSharedValue } from 'react-native-reanimated';
import { scheduleOnRN } from 'react-native-worklets';

import type { PhotoInfo } from '@lumik/core-native';

import { GestureGuide } from './GestureGuide';
import { PhotoInfoPanel } from './PhotoInfoPanel';
import { loadPhotos } from './photos';
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
  /** Archivo sin `file://` para el histograma. */
  histPath: string;
};

/** Dónde se guarda cada cambio. Si un método lanza, el cambio no se aplica. */
export type CullingBackend = {
  rate(index: number, stars: number, labels: number[]): void;
  approve(index: number, approved: boolean): void;
  /** La foto actual cambió: momento de traer las vistas previas cercanas. */
  onIndex?(index: number): void;
};

// "1,3,5" (formato del desktop) → [1, 3, 5]
const parseLabels = (value?: string) =>
  (value ?? '')
    .split(',')
    .map(Number)
    .filter((n) => n >= 1 && n <= 5);

// Abrir el proyecto y preparar las 2 000 fotos bloquea unos segundos la
// primera vez: se hace tras el primer frame, con un aviso.
export default function CullingScreen({ onCleanChange }: { onCleanChange: (clean: boolean) => void }) {
  const [photos, setPhotos] = useState<CullItem[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const t = setTimeout(() => {
      try {
        setPhotos(
          loadPhotos().map((p) => ({ info: p.info, thumb: p.thumb, preview: p.preview, histPath: p.previewPath })),
        );
      } catch (e) {
        setError(String(e));
      }
    }, 50);
    return () => clearTimeout(t);
  }, []);

  if (!photos) {
    return (
      <View style={[styles.root, styles.center]}>
        {error ? <Text style={styles.error}>{error}</Text> : <ActivityIndicator color="#e9c349" size="large" />}
      </View>
    );
  }
  return <Culling photos={photos} onCleanChange={onCleanChange} />;
}

type CullingProps = {
  photos: CullItem[];
  onCleanChange: (clean: boolean) => void;
  backend?: CullingBackend;
  /** Volver al inicio (prototipo); sin él no hay botón. */
  onBack?: () => void;
  /** Aviso fijo arriba (p. ej. proyecto en solo lectura) con una acción. */
  notice?: { text: string; detail?: string; warn?: boolean; actionLabel: string; onAction: () => void };
};

export function Culling({ photos, onCleanChange, backend, onBack, notice }: CullingProps) {
  const count = photos.length;
  const [index, setIndex] = useState(0);
  const [marks, setMarks] = useState<Marks>(() => ({
    approved: photos.map((p) => p.info.culled),
    rating: photos.map((p) => p.info.stars),
    labels: photos.map((p) => parseLabels(p.info.colorLabel)),
  }));
  const [error, setError] = useState<string | null>(null);
  const [clean, setClean] = useState(false);
  const [showInfo, setShowInfo] = useState(true);
  // La guía se abre sola al entrar; el spike no recuerda si ya se vio.
  const [guideStep, setGuideStep] = useState<number | null>(0);
  const [undo, setUndo] = useState<Undo | null>(null);
  const [bench, setBench] = useState<string | null>(null);
  const stripRef = useRef<FlashListRef<CullItem>>(null);
  const fps = useFps();
  const items = useMemo(() => photos.map((p) => ({ uri: p.preview, rotation: p.info.rotation })), [photos]);

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

  const toggleClean = useCallback(() => setClean((c) => !c), []);
  useEffect(() => onCleanChange(clean), [clean, onCleanChange]);

  const { startBenchmark, isBenchmarkRunning } = useBenchmark(
    stripRef as RefObject<FlashListRef<CullItem>>,
    (res) => setBench(res.js ? `tira: JS prom. ${res.js.averageFPS.toFixed(0)} · mín. ${res.js.minFPS.toFixed(0)}` : 'sin datos'),
    { startManually: true, repeatCount: 1 },
  );

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

        <View style={styles.topBar} pointerEvents="box-none">
          <View style={styles.topLeft}>
            {onBack && <RoundToggle label="‹" on={false} onPress={onBack} />}
            <Text style={styles.chip}>
              {index + 1} / {count}
            </Text>
            {isApproved && <Text style={styles.badge}>Aprobada</Text>}
            <Pressable onPress={startBenchmark} disabled={isBenchmarkRunning} hitSlop={8}>
              <Text style={styles.perfText}>
                {photos[index].info.fileName} · {approvedCount} aprobadas · UI {fps.ui} · JS {fps.js} fps
                {isBenchmarkRunning ? ' · midiendo…' : bench ? ` · ${bench}` : ''}
              </Text>
            </Pressable>
          </View>
          <View style={styles.topRight}>
            <RoundToggle label="?" on={guideStep !== null} onPress={() => setGuideStep(guideStep === null ? 0 : null)} />
            <RoundToggle label="i" on={showInfo} onPress={() => setShowInfo((v) => !v)} italic />
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
                  <Text style={[styles.starText, s <= marks.rating[index] && styles.starOn]}>★</Text>
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

        {guideStep !== null && <GestureGuide step={guideStep} onStep={setGuideStep} onClose={() => setGuideStep(null)} />}
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
        {approved && <Text style={styles.thumbApproved}>✓</Text>}
        {rating > 0 && <Text style={styles.thumbRating}>{'★'.repeat(rating)}</Text>}
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

function RoundToggle({ label, on, onPress, italic }: { label: string; on: boolean; onPress: () => void; italic?: boolean }) {
  return (
    <Pressable style={[styles.round, on && styles.roundOn]} onPress={onPress} hitSlop={6}>
      <Text style={[styles.roundText, italic && styles.italic, on && styles.roundTextOn]}>{label}</Text>
    </Pressable>
  );
}

// FPS del hilo de UI (frame callback de Reanimated) y del hilo JS
// (requestAnimationFrame), promediados cada medio segundo.
function useFps() {
  const [fps, setFps] = useState({ ui: 0, js: 0 });
  const jsRef = useRef(0);

  useEffect(() => {
    let frame = 0;
    let frames = 0;
    let start = performance.now();
    const tick = (now: number) => {
      frames += 1;
      if (now - start >= 500) {
        jsRef.current = Math.round((frames * 1000) / (now - start));
        frames = 0;
        start = now;
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, []);

  const report = useCallback((ui: number) => setFps({ ui, js: jsRef.current }), []);
  const uiFrames = useSharedValue(0);
  const uiStart = useSharedValue(0);
  useFrameCallback((info) => {
    'worklet';
    if (uiStart.value === 0) uiStart.value = info.timestamp;
    uiFrames.value += 1;
    const elapsed = info.timestamp - uiStart.value;
    if (elapsed >= 500) {
      scheduleOnRN(report, Math.round((uiFrames.value * 1000) / elapsed));
      uiFrames.value = 0;
      uiStart.value = info.timestamp;
    }
  });

  return fps;
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#131313' },
  center: { alignItems: 'center', justifyContent: 'center' },
  error: { color: '#f87171', fontSize: 16, padding: 24 },
  viewerPane: { flex: 1 },
  topBar: { position: 'absolute', top: 12, left: 16, right: 16, flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between' },
  topLeft: { flexDirection: 'row', alignItems: 'center', gap: 8 },
  topRight: { flexDirection: 'row', alignItems: 'center', gap: 10 },
  chip: { color: '#ddd', fontSize: 15, backgroundColor: 'rgba(19,19,19,0.7)', paddingHorizontal: 10, paddingVertical: 4, borderRadius: 8 },
  badge: { color: '#131313', backgroundColor: '#e9c349', fontWeight: '700', paddingHorizontal: 10, paddingVertical: 4, borderRadius: 8 },
  round: { width: 44, height: 44, borderRadius: 22, alignItems: 'center', justifyContent: 'center', backgroundColor: 'rgba(19,19,19,0.7)' },
  roundOn: { backgroundColor: '#e9e9e9' },
  roundText: { color: '#fff', fontSize: 20, fontWeight: '700' },
  italic: { fontStyle: 'italic' },
  roundTextOn: { color: '#131313' },
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
    overflow: 'hidden',
    textAlign: 'center',
    lineHeight: 22,
    fontSize: 13,
    backgroundColor: '#e9c349',
    color: '#131313',
    fontWeight: '700',
  },
  thumbRating: { position: 'absolute', bottom: 2, left: 4, color: '#e9c349', fontSize: 9, textShadowRadius: 3, textShadowColor: '#000' },
  thumbLabels: { position: 'absolute', top: 3, left: 3, flexDirection: 'row', gap: 2 },
  thumbLabel: { width: 8, height: 8, borderRadius: 4 },
  marksRow: { position: 'absolute', left: 12, right: 12, bottom: 10, flexDirection: 'row', justifyContent: 'space-between' },
  pill: { flexDirection: 'row', borderRadius: 22, backgroundColor: 'rgba(19,19,19,0.7)' },
  mark: { width: 44, height: 44, alignItems: 'center', justifyContent: 'center' },
  starText: { color: '#666', fontSize: 22 },
  starOn: { color: '#e9c349' },
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
