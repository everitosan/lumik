// Experimento 2.5 del spike (Android): el disco por SAF.
//  1. Elegir la carpeta del disco una vez; el permiso sobrevive al reinicio.
//  2. Abrir un proyecto en modo espejado (D2): `project.db` se copia a la
//     sandbox, se edita ahí y se devuelve con `VACUUM INTO` + renombrado.
//  3. Copiar la tarjeta al proyecto leyendo y escribiendo por fd en Rust.
import {
  copyFdToPath,
  copyPathToFd,
  openProject,
  resetDir,
  setStars,
  startFdCopy,
  startFdRead,
  startFdWrite,
  vacuumInto,
  type CopyListener,
  type CopyResult,
} from '@lumik/core-native';
import { useCallback, useEffect, useState } from 'react';
import { Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';

import Saf from '../modules/saf/src/LumikSafModule';
import { discoverProjects } from './proto/projects';
import type { DiskProject } from './proto/session';
import type { SafEntry } from '../modules/saf/src/LumikSaf.types';

const MIRROR_DIR = '/data/user/0/com.lumik.tabletspike/files/mirror';
const BINARY = 'application/octet-stream';
const TMP_NAME = 'project.db.lumik-tmp';
// Lo de la tarjeta va a una carpeta aparte para no mezclarlo con `_media/`:
// dentro del proyecto abierto o, sin proyecto, en la carpeta elegida del disco.
const IMPORT_DIR = '_spike_import';
const LOOSE_IMPORT_DIR = '_lumik_spike_import';
const WRITE_TEST_BYTES = 1_000_000_000;
const MEDIA = /\.(raf|cr2|cr3|nef|arw|dng|orf|rw2|jpe?g|heic|mov|mp4)$/i;

type Project = DiskProject;
type Log = { at: string; text: string; error?: boolean };

export default function SafScreen() {
  const [trees, setTrees] = useState<string[]>(() => Saf.persistedTrees());
  const [disk, setDisk] = useState<string | null>(() => Saf.persistedTrees()[0] ?? null);
  const [projects, setProjects] = useState<Project[] | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [mirror, setMirror] = useState<{ dir: string; photos: number; firstId?: string } | null>(null);
  const [card, setCard] = useState<{ uri: string; files: SafEntry[] } | null>(null);
  const [copy, setCopy] = useState<{ done: number; total: number; bytes: number; totalBytes: number; startedAt: number } | null>(null);
  const [log, setLog] = useState<Log[]>([]);
  // Mientras algo corre, los botones se deshabilitan: sin esto se tocaban
  // varias veces porque no se veía que la acción había empezado.
  const [busy, setBusy] = useState<string | null>(null);
  // Leer y escribir a la vez (un hilo cada uno) o por turnos, para comparar.
  const [pipelined, setPipelined] = useState(true);

  const note = useCallback((text: string, error = false) => {
    const at = new Date().toTimeString().slice(0, 8);
    setLog((l) => [{ at, text, error }, ...l].slice(0, 40));
  }, []);

  const run = useCallback(
    async (label: string, fn: () => Promise<void> | void) => {
      setBusy(label);
      try {
        await fn();
      } catch (e) {
        note(`${label}: ${String(e)}`, true);
      } finally {
        setBusy(null);
      }
    },
    [note],
  );

  // Proyectos = carpetas con `project.db`, buscadas como el desktop.
  const scan = useCallback(
    (tree: string) =>
      run('Buscar proyectos', async () => {
        const started = Date.now();
        const found = await discoverProjects(tree, note);
        setProjects(found);
        note(`${found.length} proyectos en ${Date.now() - started} ms`);
      }),
    [note, run],
  );

  useEffect(() => {
    if (disk) scan(disk);
  }, [disk, scan]);

  const pickDisk = () =>
    run('Elegir disco', async () => {
      const tree = await Saf.pickDirectory();
      if (!tree) return;
      setTrees(Saf.persistedTrees());
      setDisk(tree);
      note(`Permiso guardado: ${decodeURIComponent(tree)}`);
    });

  // Espejo: `project.db` (y su `-wal`, si el desktop no cerró limpio) a la sandbox.
  const openMirror = (p: Project) =>
    run('Abrir proyecto', async () => {
      const started = Date.now();
      const dir = `${MIRROR_DIR}/${p.name}`;
      resetDir(dir);
      for (const suffix of ['', '-wal']) {
        const file = p.files.find((f) => f.name === `project.db${suffix}`);
        if (file) copyFdToPath(Saf.openFd(file.uri, 'r'), `${dir}/project.db${suffix}`);
      }
      const photos = openProject(dir);
      setProject(p);
      setMirror({ dir, photos: photos.length, firstId: photos[0]?.id });
      note(`${p.name}: ${photos.length} fotos, copia de trabajo en ${Date.now() - started} ms`);
    });

  const rateFirst = () =>
    run('Calificar', () => {
      if (!mirror?.firstId) return;
      setStars(mirror.dir, mirror.firstId, 5);
      note('Primera foto con ★★★★★ en la copia de trabajo');
    });

  // Devolver: `VACUUM INTO` a un temporal, escribirlo junto al original con
  // otro nombre, borrar el original (y su `-wal`/`-shm`) y renombrar.
  const giveBack = () =>
    run('Devolver', async () => {
      if (!project || !mirror) return;
      const started = Date.now();
      const out = `${mirror.dir}/out.db`;
      vacuumInto(`${mirror.dir}/project.db`, out);
      const stale = (await Saf.listChildren(project.uri)).find((e) => e.name === TMP_NAME);
      if (stale) Saf.delete(stale.uri);
      const tmp = Saf.createFile(project.uri, TMP_NAME, BINARY);
      const written = copyPathToFd(out, Saf.openFd(tmp, 'w'));
      const current = await Saf.listChildren(project.uri);
      for (const name of ['project.db', 'project.db-wal', 'project.db-shm']) {
        const f = current.find((e) => e.name === name);
        if (f) Saf.delete(f.uri);
      }
      Saf.rename(tmp, 'project.db');
      const files = await Saf.listChildren(project.uri);
      setProject({ ...project, files });
      note(`project.db devuelto (${kb(written.bytes)}) en ${Date.now() - started} ms`);
    });

  const pickCard = () =>
    run('Elegir tarjeta', async () => {
      const tree = await Saf.pickDirectory();
      if (!tree) return;
      setTrees(Saf.persistedTrees());
      const started = Date.now();
      const files = await listMedia(Saf.rootDocument(tree));
      setCard({ uri: tree, files });
      note(`Tarjeta: ${files.length} archivos, ${gb(sum(files))} (listado en ${Date.now() - started} ms)`);
    });

  const copyCard = () =>
    run('Copiar', async () => {
      if (!card || !disk) return;
      const parent = project ? project.uri : Saf.rootDocument(disk);
      const name = project ? IMPORT_DIR : LOOSE_IMPORT_DIR;
      const existing = (await Saf.listChildren(parent)).find((f) => f.name === name && f.isDirectory);
      const dest = existing?.uri ?? Saf.createDirectory(parent, name);
      const totalBytes = sum(card.files);
      const state = { done: 0, total: card.files.length, bytes: 0, totalBytes, startedAt: Date.now() };
      setCopy({ ...state });
      for (const file of card.files) {
        const dst = Saf.createFile(dest, file.name, BINARY);
        const before = state.bytes;
        const result = await fdCopy(Saf.openFd(file.uri, 'r'), Saf.openFd(dst, 'w'), pipelined, (b) => {
          state.bytes = before + b;
          setCopy({ ...state });
        });
        state.done += 1;
        state.bytes = before + Number(result.bytes);
        setCopy({ ...state });
      }
      const secs = (Date.now() - state.startedAt) / 1000;
      const mode = pipelined ? 'en paralelo' : 'por turnos';
      note(`Copiados ${state.done} archivos ${mode}, ${gb(state.bytes)} en ${secs.toFixed(1)} s (${mbps(state.bytes, secs)})`);
    });

  // Diagnóstico: la tarjeta y el disco por separado, para saber cuál limita
  // la copia (o si el límite es usarlos a la vez por el hub).
  const measureRead = () =>
    run('Medir lectura', async () => {
      if (!card) return;
      const started = Date.now();
      let bytes = 0;
      for (const file of card.files) {
        const r = await fdTask((l) => startFdRead(Saf.openFd(file.uri, 'r'), l));
        bytes += Number(r.bytes);
      }
      const secs = (Date.now() - started) / 1000;
      note(`Lectura de la tarjeta: ${gb(bytes)} en ${secs.toFixed(1)} s (${mbps(bytes, secs)})`);
    });

  const measureWrite = () =>
    run('Medir escritura', async () => {
      if (!disk) return;
      const parent = project ? project.uri : Saf.rootDocument(disk);
      const file = Saf.createFile(parent, '_lumik_prueba_escritura.bin', BINARY);
      const started = Date.now();
      const r = await fdTask((l) => startFdWrite(Saf.openFd(file, 'w'), BigInt(WRITE_TEST_BYTES), l));
      const secs = (Date.now() - started) / 1000;
      Saf.delete(file);
      note(`Escritura en el disco: ${gb(Number(r.bytes))} en ${secs.toFixed(1)} s (${mbps(Number(r.bytes), secs)})`);
    });

  const secs = copy ? (Date.now() - copy.startedAt) / 1000 : 0;

  return (
    <ScrollView style={styles.root} contentContainerStyle={styles.content}>
      {busy && <Text style={styles.busy}>{busy}…</Text>}
      <Section title="1. Disco">
        {/* Cada permiso guardado se puede usar como disco sin volver al selector. */}
        <Text style={styles.text}>Permisos guardados{trees.length ? ' (toca uno para usarlo como disco):' : ': ninguno'}</Text>
        <View style={styles.chips}>
          {trees.map((t) => (
            <Pressable
              key={t}
              style={({ pressed }) => [styles.chip, t === disk && styles.chipOn, pressed && styles.buttonPressed]}
              onPress={() => {
                setProject(null);
                setMirror(null);
                setDisk(t);
              }}
              disabled={!!busy}
            >
              <Text style={[styles.chipText, t === disk && styles.chipTextOn]}>{treeLabel(t)}</Text>
            </Pressable>
          ))}
        </View>
        <Button label={disk ? 'Elegir otra carpeta del disco' : 'Elegir la carpeta del disco'} onPress={pickDisk} disabled={!!busy} />
        {projects && (
          <View style={styles.list}>
            <Text style={styles.warn}>
              {projects.length
                ? 'Si usas un proyecto real, respalda antes su project.db: devolver lo reescribe.'
                : 'No hay proyectos aquí (se buscan como en el desktop: dentro de lumik/, hasta 4 niveles). La tarjeta se puede copiar igual.'}
            </Text>
            {projects.map((p) => (
              <Pressable
                key={p.uri}
                style={({ pressed }) => [styles.item, project?.uri === p.uri && styles.itemOn, pressed && styles.buttonPressed]}
                onPress={() => openMirror(p)}
                disabled={!!busy}
              >
                <Text style={styles.text}>{p.name}</Text>
              </Pressable>
            ))}
          </View>
        )}
      </Section>

      {mirror && project && (
        <Section title={`2. Proyecto espejado: ${project.name}`}>
          <Text style={styles.text}>{mirror.photos} fotos en la copia de trabajo de la sandbox.</Text>
          <View style={styles.row}>
            <Button label="★5 a la primera foto" onPress={rateFirst} disabled={!!busy} />
            <Button label="Devolver al disco" onPress={giveBack} disabled={!!busy} />
          </View>
        </Section>
      )}

      {disk && (
        <Section title={project ? '3. Tarjeta → proyecto' : '3. Tarjeta → disco'}>
          <Button label="Medir escritura en el disco (1 GB)" onPress={measureWrite} disabled={!!busy} />
          <Button label={card ? 'Elegir otra carpeta de la tarjeta' : 'Elegir la carpeta de la tarjeta (DCIM)'} onPress={pickCard} disabled={!!busy} />
          {card && (
            <>
              <Text style={styles.text}>
                {card.files.length} archivos · {gb(sum(card.files))} → {project ? `${project.name}/${IMPORT_DIR}` : LOOSE_IMPORT_DIR}
              </Text>
              <Pressable style={styles.toggle} onPress={() => setPipelined((p) => !p)} disabled={!!busy}>
                <View style={[styles.check, pipelined && styles.checkOn]} />
                <Text style={styles.text}>Leer y escribir en paralelo</Text>
              </Pressable>
              <View style={styles.row}>
                <Button label="Copiar" onPress={copyCard} disabled={!!busy} />
                <Button label="Medir lectura de la tarjeta" onPress={measureRead} disabled={!!busy} />
              </View>
            </>
          )}
          {copy && (
            <>
              <View style={styles.bar}>
                <View style={[styles.barFill, { width: `${(copy.bytes / Math.max(1, copy.totalBytes)) * 100}%` }]} />
              </View>
              <Text style={styles.text}>
                {copy.done}/{copy.total} · {gb(copy.bytes)} de {gb(copy.totalBytes)} · {mbps(copy.bytes, secs)}
              </Text>
            </>
          )}
        </Section>
      )}

      <Section title="Registro">
        {log.map((l, i) => (
          <Text key={i} style={[styles.log, l.error && styles.logError]}>
            {l.at} {l.text}
          </Text>
        ))}
      </Section>
    </ScrollView>
  );
}

// Fotos y videos de la tarjeta, recorriendo subcarpetas (DCIM/100_FUJI/…).
async function listMedia(dirUri: string): Promise<SafEntry[]> {
  const out: SafEntry[] = [];
  for (const e of await Saf.listChildren(dirUri)) {
    if (e.isDirectory) out.push(...(await listMedia(e.uri)));
    else if (MEDIA.test(e.name)) out.push(e);
  }
  return out;
}

function fdCopy(src: number, dst: number, pipelined: boolean, onProgress: (bytes: number) => void): Promise<CopyResult> {
  return fdTask((l) => startFdCopy(src, dst, pipelined, l), onProgress);
}

// Una operación de Rust con `CopyListener`, como promesa.
function fdTask(start: (listener: CopyListener) => void, onProgress?: (bytes: number) => void): Promise<CopyResult> {
  return new Promise((resolve, reject) =>
    start({
      onProgress: (b) => onProgress?.(Number(b)),
      onDone: resolve,
      onError: (m) => reject(new Error(m)),
    }),
  );
}

// "content://…/tree/primary%3ALumikDisco" → "primary:LumikDisco"; un disco USB
// aparece con el id de su volumen ("FB5F-B3EF:").
const treeLabel = (tree: string) => decodeURIComponent(tree.split('/tree/')[1] ?? tree);
const sum = (files: SafEntry[]) => files.reduce((s, f) => s + f.size, 0);
const kb = (b: bigint | number) => `${(Number(b) / 1024).toFixed(0)} KB`;
const gb = (b: number) => `${(b / 1e9).toFixed(2)} GB`;
const mbps = (b: number, secs: number) => (secs > 0 ? `${(b / 1e6 / secs).toFixed(0)} MB/s` : '—');

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <View style={styles.section}>
      <Text style={styles.title}>{title}</Text>
      {children}
    </View>
  );
}

function Button({ label, onPress, disabled }: { label: string; onPress: () => void; disabled?: boolean }) {
  return (
    <Pressable
      style={({ pressed }) => [styles.button, pressed && styles.buttonPressed, disabled && styles.buttonOff]}
      onPress={onPress}
      disabled={disabled}
    >
      <Text style={styles.buttonText}>{label}</Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#131313' },
  content: { padding: 24, gap: 20 },
  section: { gap: 10, padding: 16, borderRadius: 12, backgroundColor: '#1b1b1b' },
  title: { color: '#fff', fontSize: 18, fontWeight: '600' },
  text: { color: '#ddd', fontSize: 15 },
  warn: { color: '#e9c349', fontSize: 14 },
  list: { gap: 6 },
  toggle: { flexDirection: 'row', alignItems: 'center', gap: 10, minHeight: 44 },
  check: { width: 22, height: 22, borderRadius: 4, borderWidth: 2, borderColor: '#9a9a9a' },
  checkOn: { backgroundColor: '#e9c349', borderColor: '#e9c349' },
  chips: { flexDirection: 'row', flexWrap: 'wrap', gap: 8 },
  chip: { minHeight: 44, justifyContent: 'center', paddingHorizontal: 14, borderRadius: 22, backgroundColor: '#262626' },
  chipOn: { backgroundColor: '#e9e9e9' },
  chipText: { color: '#ddd', fontSize: 14 },
  chipTextOn: { color: '#131313', fontWeight: '600' },
  item: { minHeight: 48, justifyContent: 'center', paddingHorizontal: 14, borderRadius: 8, backgroundColor: '#262626' },
  itemOn: { backgroundColor: '#2563eb' },
  row: { flexDirection: 'row', gap: 12 },
  button: { alignSelf: 'flex-start', minHeight: 48, justifyContent: 'center', paddingHorizontal: 18, borderRadius: 10, backgroundColor: '#2563eb' },
  buttonOff: { opacity: 0.4 },
  buttonPressed: { opacity: 0.7 },
  busy: { color: '#e9c349', fontSize: 15 },
  buttonText: { color: '#fff', fontSize: 16, fontWeight: '600' },
  bar: { height: 8, borderRadius: 4, backgroundColor: '#2a2a2a', overflow: 'hidden' },
  barFill: { height: 8, backgroundColor: '#e9c349' },
  log: { color: '#9a9a9a', fontSize: 13, fontVariant: ['tabular-nums'] },
  logError: { color: '#f87171' },
});
