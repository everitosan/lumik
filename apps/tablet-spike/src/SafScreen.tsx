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
  vacuumInto,
  type CopyResult,
} from '@lumik/core-native';
import { useCallback, useEffect, useState } from 'react';
import { Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';

import Saf from '../modules/saf/src/LumikSafModule';
import type { SafEntry } from '../modules/saf/src/LumikSaf.types';

const MIRROR_DIR = '/data/user/0/com.lumik.tabletspike/files/mirror';
const BINARY = 'application/octet-stream';
const TMP_NAME = 'project.db.lumik-tmp';
// Lo de la tarjeta va a una carpeta aparte para no mezclarlo con `_media/`.
const IMPORT_DIR = '_spike_import';
const MEDIA = /\.(raf|cr2|cr3|nef|arw|dng|orf|rw2|jpe?g|heic|mov|mp4)$/i;

type Project = { name: string; uri: string; files: SafEntry[] };
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

  // Proyectos = carpetas de la raíz del disco que tienen `project.db`.
  const scan = useCallback(
    (tree: string) =>
      run('Buscar proyectos', async () => {
        const started = Date.now();
        const root = Saf.rootDocument(tree);
        const found: Project[] = [];
        for (const dir of (await Saf.listChildren(root)).filter((e) => e.isDirectory)) {
          let files = await Saf.listChildren(dir.uri);
          // Una devolución interrumpida entre borrar el original y renombrar
          // deja solo el temporal: se termina aquí.
          const tmp = files.find((f) => f.name === TMP_NAME);
          if (tmp && !files.some((f) => f.name === 'project.db')) {
            Saf.rename(tmp.uri, 'project.db');
            note(`${dir.name}: devolución interrumpida recuperada`);
            files = await Saf.listChildren(dir.uri);
          }
          if (files.some((f) => f.name === 'project.db')) found.push({ name: dir.name, uri: dir.uri, files });
        }
        found.sort((a, b) => a.name.localeCompare(b.name));
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
      if (!card || !project) return;
      const existing = project.files.find((f) => f.name === IMPORT_DIR && f.isDirectory);
      const dest = existing?.uri ?? Saf.createDirectory(project.uri, IMPORT_DIR);
      const totalBytes = sum(card.files);
      const state = { done: 0, total: card.files.length, bytes: 0, totalBytes, startedAt: Date.now() };
      setCopy({ ...state });
      for (const file of card.files) {
        const dst = Saf.createFile(dest, file.name, BINARY);
        const before = state.bytes;
        const result = await fdCopy(Saf.openFd(file.uri, 'r'), Saf.openFd(dst, 'w'), (b) => {
          state.bytes = before + b;
          setCopy({ ...state });
        });
        state.done += 1;
        state.bytes = before + Number(result.bytes);
        setCopy({ ...state });
      }
      const secs = (Date.now() - state.startedAt) / 1000;
      note(`Copiados ${state.done} archivos, ${gb(state.bytes)} en ${secs.toFixed(1)} s (${mbps(state.bytes, secs)})`);
    });

  const secs = copy ? (Date.now() - copy.startedAt) / 1000 : 0;

  return (
    <ScrollView style={styles.root} contentContainerStyle={styles.content}>
      {busy && <Text style={styles.busy}>{busy}…</Text>}
      <Section title="1. Disco">
        <Text style={styles.text}>
          Permisos guardados: {trees.length ? trees.map((t) => decodeURIComponent(t.split('/tree/')[1] ?? t)).join(' · ') : 'ninguno'}
        </Text>
        <Button label={disk ? 'Elegir otra carpeta del disco' : 'Elegir la carpeta del disco'} onPress={pickDisk} disabled={!!busy} />
        {projects && (
          <View style={styles.list}>
            <Text style={styles.warn}>Usa un proyecto de prueba: devolver reescribe su project.db.</Text>
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

      {project && (
        <Section title="3. Tarjeta → proyecto">
          <Button label={card ? 'Elegir otra carpeta de la tarjeta' : 'Elegir la carpeta de la tarjeta (DCIM)'} onPress={pickCard} disabled={!!busy} />
          {card && (
            <>
              <Text style={styles.text}>
                {card.files.length} archivos · {gb(sum(card.files))} → {project.name}/{IMPORT_DIR}
              </Text>
              <Button label="Copiar" onPress={copyCard} disabled={!!busy} />
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

function fdCopy(src: number, dst: number, onProgress: (bytes: number) => void): Promise<CopyResult> {
  return new Promise((resolve, reject) =>
    startFdCopy(src, dst, {
      onProgress: (b) => onProgress(Number(b)),
      onDone: resolve,
      onError: (m) => reject(new Error(m)),
    }),
  );
}

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
