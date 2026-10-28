// Experimento 2.1 del spike: progreso desde un hilo de Rust (uniffi) mientras
// una lista se desplaza sola. La UI no repinta por evento: guarda el último en
// un ref y lo pinta en cada frame, como lo haría la app real.
import { StatusBar } from 'expo-status-bar';
import { useEffect, useRef, useState } from 'react';
import { FlatList, Pressable, StyleSheet, Text, View } from 'react-native';
import {
  startProgressBurst,
  startTestImport,
  type ImportListener,
  type ImportProgressEvent,
  type ImportSummary,
} from '@lumik/core-native';

// Carpetas dentro de la sandbox de la app (build de debug: se llenan con `adb shell run-as`).
const FILES_DIR = '/data/user/0/com.lumik.tabletspike/files';
const SAMPLE_DIR = `${FILES_DIR}/sample`;
const PROJECT_DIR = `${FILES_DIR}/project`;

const ROWS = Array.from({ length: 2000 }, (_, i) => `DSCF${String(i).padStart(4, '0')}.RAF`);
const ROW_HEIGHT = 56;

type Run = {
  label: string;
  received: number;
  last?: ImportProgressEvent;
  summary?: ImportSummary;
  startedAt: number;
  jsElapsedMs?: number;
  minFps: number;
};

export default function App() {
  const [run, setRun] = useState<Run | null>(null);
  const [fps, setFps] = useState(0);
  const runRef = useRef<Run | null>(null);
  const listRef = useRef<FlatList<string>>(null);

  // Cada frame: mide los FPS del hilo JS, desplaza la lista y pinta el último evento.
  useEffect(() => {
    let frame = 0;
    let frames = 0;
    let windowStart = performance.now();
    let offset = 0;
    const tick = (now: number) => {
      frames += 1;
      if (now - windowStart >= 500) {
        const current = Math.round((frames * 1000) / (now - windowStart));
        setFps(current);
        const r = runRef.current;
        if (r && !r.summary) r.minFps = Math.min(r.minFps, current);
        frames = 0;
        windowStart = now;
      }
      offset = (offset + 6) % (ROWS.length * ROW_HEIGHT - 2000);
      listRef.current?.scrollToOffset({ offset, animated: false });
      if (runRef.current) setRun({ ...runRef.current });
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, []);

  // Cada corrida tiene su listener: si otra empieza antes de que termine, los
  // eventos tardíos no pisan la nueva.
  const listener = (r: Run): ImportListener => ({
    onProgress(event) {
      r.received += 1;
      r.last = event;
    },
    onDone(summary) {
      r.summary = summary;
      r.jsElapsedMs = Math.round(performance.now() - r.startedAt);
    },
  });

  const begin = (label: string, start: (l: ImportListener) => void) => {
    const r: Run = { label, received: 0, startedAt: performance.now(), minFps: Infinity };
    runRef.current = r;
    start(listener(r));
  };

  const last = run?.last;
  const pct = last && last.bytesTotal > 0n ? Number((last.bytesDone * 100n) / last.bytesTotal) : 0;

  return (
    <View style={styles.root}>
      <View style={styles.panel}>
        <Text style={styles.title}>Spike 2.1 · puente Rust ↔ RN</Text>
        <Text style={styles.fps}>JS {fps} fps</Text>

        <Pressable style={styles.button} onPress={() => begin('Ráfaga 1 000 (sin pausa)', (l) => startProgressBurst(1000, 0n, l))}>
          <Text style={styles.buttonText}>Ráfaga 1 000 (sin pausa)</Text>
        </Pressable>
        <Pressable style={styles.button} onPress={() => begin('Ráfaga 1 000 (1 ms)', (l) => startProgressBurst(1000, 1000n, l))}>
          <Text style={styles.buttonText}>Ráfaga 1 000 (1 ms)</Text>
        </Pressable>
        <Pressable style={styles.button} onPress={() => begin('Import de prueba', (l) => startTestImport(SAMPLE_DIR, PROJECT_DIR, l))}>
          <Text style={styles.buttonText}>Import de prueba</Text>
        </Pressable>

        {run && (
          <View style={styles.result}>
            <Text style={styles.label}>{run.label}</Text>
            <View style={styles.bar}>
              <View style={[styles.barFill, { width: `${pct}%` }]} />
            </View>
            <Text style={styles.text}>
              {last ? `${last.fileIndex}/${last.fileTotal} · ${last.fileName}` : 'Esperando…'}
            </Text>
            <Text style={styles.text}>Eventos recibidos: {run.received}</Text>
            {run.summary && (
              <>
                <Text style={styles.text}>
                  Copiadas {run.summary.copied} · fallidas {run.summary.failed} · ignoradas {run.summary.skipped}
                </Text>
                <Text style={styles.text}>
                  Rust {String(run.summary.elapsedMs)} ms · JS {run.jsElapsedMs} ms · FPS mínimo {run.minFps === Infinity ? '—' : run.minFps}
                </Text>
              </>
            )}
          </View>
        )}
      </View>

      <FlatList
        ref={listRef}
        style={styles.list}
        data={ROWS}
        keyExtractor={(item) => item}
        getItemLayout={(_, index) => ({ length: ROW_HEIGHT, offset: ROW_HEIGHT * index, index })}
        renderItem={({ item, index }) => (
          <View style={[styles.row, index % 2 === 0 && styles.rowAlt]}>
            <Text style={styles.text}>{item}</Text>
          </View>
        )}
      />
      <StatusBar style="light" />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, flexDirection: 'row', backgroundColor: '#131313' },
  panel: { width: 420, padding: 32, gap: 16 },
  title: { color: '#fff', fontSize: 22, fontWeight: '600' },
  fps: { color: '#e9c349', fontSize: 40, fontVariant: ['tabular-nums'] },
  button: { backgroundColor: '#2563eb', borderRadius: 12, minHeight: 56, justifyContent: 'center', paddingHorizontal: 20 },
  buttonText: { color: '#fff', fontSize: 18, fontWeight: '600' },
  result: { gap: 8, marginTop: 8 },
  label: { color: '#e9c349', fontSize: 16, fontWeight: '600' },
  bar: { height: 8, borderRadius: 4, backgroundColor: '#2a2a2a', overflow: 'hidden' },
  barFill: { height: 8, backgroundColor: '#e9c349' },
  text: { color: '#ddd', fontSize: 15, fontVariant: ['tabular-nums'] },
  list: { flex: 1 },
  row: { height: ROW_HEIGHT, justifyContent: 'center', paddingHorizontal: 24 },
  rowAlt: { backgroundColor: '#1b1b1b' },
});
