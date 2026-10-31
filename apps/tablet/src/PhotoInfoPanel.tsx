// Datos técnicos sobre el visor, como la barra lateral del desktop pero sin
// quitarle ancho a la foto: histograma RGB, exposición y equipo, en una tarjeta
// translúcida que se oculta con el botón "i".
import { type Histogram, startHistogram, type PhotoInfo } from '@lumik/core-native';
import { useEffect, useState } from 'react';
import { StyleSheet, Text, View } from 'react-native';

const BINS = 64;
const HIST_WIDTH = 224;
const HIST_HEIGHT = 96;
// Tras un cambio de foto, espera a que el dedo se detenga antes de decodificar.
const DEBOUNCE_MS = 120;

const cache = new Map<string, Histogram>();

function useHistogram(path: string) {
  const [hist, setHist] = useState<Histogram | null>(() => cache.get(path) ?? null);

  useEffect(() => {
    if (!path) {
      setHist(null);
      return;
    }
    const cached = cache.get(path);
    if (cached) {
      setHist(cached);
      return;
    }
    setHist(null);
    let alive = true;
    const t = setTimeout(() => {
      startHistogram(path, {
        onHistogram(h) {
          cache.set(path, h);
          if (alive) setHist(h);
        },
        onError(message) {
          console.warn('histograma', path, message);
        },
      });
    }, DEBOUNCE_MS);
    return () => {
      alive = false;
      clearTimeout(t);
    };
  }, [path]);

  return hist;
}

// Dos columnas para que la tarjeta quede casi cuadrada:
//   histograma
//   velocidad | diafragma
//   ISO       | EV
//   cámara
//   óptica    | focal
//   fecha     | hora
export function PhotoInfoPanel({ info, previewPath }: { info: PhotoInfo; previewPath: string }) {
  const hist = useHistogram(previewPath);
  const ev = info.exposureCompensation;
  const when = info.captureDate ? splitDate(info.captureDate) : null;

  return (
    <View style={styles.card} pointerEvents="none">
      <View style={styles.hist}>{hist && <HistogramBars hist={hist} />}</View>
      <Row left={info.shutterSpeed && `${info.shutterSpeed} s`} right={info.aperture} strong />
      <Row
        left={info.iso != null ? `ISO ${info.iso}` : undefined}
        right={ev != null ? `${ev > 0 ? '+' : ev < 0 ? '−' : '±'}${Math.abs(ev).toFixed(1)} EV` : undefined}
        strong
      />
      <View style={styles.divider} />
      {info.camera && (
        <Text style={styles.cell} numberOfLines={1}>
          {info.camera}
        </Text>
      )}
      <Row left={info.lensModel} right={info.focalLength && `${info.focalLength.replace(/\s*mm$/, '')} mm`} />
      <Row left={when?.date} right={when?.time} muted />
    </View>
  );
}

function Row({ left, right, strong, muted }: { left?: string | null; right?: string | null; strong?: boolean; muted?: boolean }) {
  if (!left && !right) return null;
  const style = [styles.cell, strong && styles.strong, muted && styles.muted];
  return (
    <View style={styles.row}>
      <Text style={[style, styles.left]} numberOfLines={1}>
        {left ?? '—'}
      </Text>
      <Text style={[style, styles.right]} numberOfLines={1}>
        {right ?? '—'}
      </Text>
    </View>
  );
}

// Misma escala que el desktop: raíz del conteo contra el máximo de 1–254
// (sin los extremos, para que un cielo quemado no aplaste el resto). Los
// canales se mezclan como el modo "screen" del desktop: donde coinciden los
// tres, gris claro; donde coinciden dos, su mezcla; arriba, el que sobresale.
const CHANNEL = ['r', 'g', 'b'] as const;
const SOLO = { r: 'rgb(220,60,60)', g: 'rgb(60,200,60)', b: 'rgb(60,120,220)' };
const PAIR: Record<string, string> = { rg: 'rgb(225,205,70)', gr: 'rgb(225,205,70)', rb: 'rgb(200,90,210)', br: 'rgb(200,90,210)', gb: 'rgb(70,200,215)', bg: 'rgb(70,200,215)' };
const ALL = 'rgb(205,205,205)';

function HistogramBars({ hist }: { hist: Histogram }) {
  const group = 256 / BINS;
  const sums = CHANNEL.map((c) =>
    Array.from({ length: BINS }, (_, i) => {
      let sum = 0;
      for (let k = 0; k < group; k++) sum += hist[c][i * group + k];
      return sum;
    }),
  );
  let max = 1;
  for (const bins of sums) for (let i = 1; i < BINS - 1; i++) max = Math.max(max, bins[i]);
  const height = (v: number) => Math.min(1, Math.sqrt(v / max)) * HIST_HEIGHT;
  const barWidth = HIST_WIDTH / BINS;

  const bars = [];
  for (let i = 0; i < BINS; i++) {
    // Canales del más bajo al más alto en este nivel.
    const order = [0, 1, 2].sort((a, b) => sums[a][i] - sums[b][i]);
    const [lo, mid, hi] = order.map((c) => height(sums[c][i]));
    const segments = [
      { from: 0, to: lo, color: ALL },
      { from: lo, to: mid, color: PAIR[CHANNEL[order[1]] + CHANNEL[order[2]]] },
      { from: mid, to: hi, color: SOLO[CHANNEL[order[2]]] },
    ];
    for (const [k, seg] of segments.entries()) {
      if (seg.to - seg.from < 0.5) continue;
      bars.push(
        <View
          key={`${i}-${k}`}
          style={{ position: 'absolute', left: i * barWidth, width: barWidth, bottom: seg.from, height: seg.to - seg.from, backgroundColor: seg.color, opacity: 0.85 }}
        />,
      );
    }
  }
  return <>{bars}</>;
}

// "2026:06:28 11:45:03" (EXIF) → { date: "28 jun 2026", time: "11:45" }
function splitDate(exif: string) {
  const m = exif.match(/^(\d{4}):(\d{2}):(\d{2}) (\d{2}):(\d{2})/);
  if (!m) return { date: exif, time: undefined };
  const months = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  return { date: `${Number(m[3])} ${months[Number(m[2]) - 1]} ${m[1]}`, time: `${m[4]}:${m[5]}` };
}

const styles = StyleSheet.create({
  card: {
    position: 'absolute',
    top: 64,
    right: 16,
    width: HIST_WIDTH + 24,
    padding: 12,
    gap: 8,
    borderRadius: 12,
    backgroundColor: 'rgba(19,19,19,0.78)',
  },
  hist: {
    width: HIST_WIDTH,
    height: HIST_HEIGHT,
    borderRadius: 4,
    overflow: 'hidden',
    backgroundColor: 'rgba(255,255,255,0.04)',
  },
  row: { flexDirection: 'row', gap: 12 },
  left: { flex: 3 },
  right: { flex: 2, textAlign: 'right' },
  cell: { color: '#ddd', fontSize: 14, fontVariant: ['tabular-nums'] },
  strong: { color: '#fff', fontSize: 17, fontWeight: '600' },
  muted: { color: '#9a9a9a', fontSize: 13 },
  divider: { height: 1, backgroundColor: 'rgba(255,255,255,0.08)', marginVertical: 2 },
});
