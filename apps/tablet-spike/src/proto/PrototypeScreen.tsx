// Experimento 3.1 del spike (Android, D5): inicio → importar la tarjeta a un
// proyecto del disco → culling, con cada cambio guardado en el disco por SAF.
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ActivityIndicator, AppState, StyleSheet, Text, View } from 'react-native';

import Saf from '../../modules/saf/src/LumikSafModule';
import { Culling, type CullingBackend, type CullItem } from '../CullingScreen';
import { HomeScreen } from './HomeScreen';
import { ImportScreen } from './ImportScreen';
import { discoverProjects } from './projects';
import { type DiskProject, ProjectSession } from './session';

type Log = { at: string; text: string; error?: boolean };

export default function PrototypeScreen({ onCleanChange }: { onCleanChange: (clean: boolean) => void }) {
  const [trees, setTrees] = useState<string[]>(() => Saf.persistedTrees());
  const [disk, setDisk] = useState<string | null>(() => defaultDisk(Saf.persistedTrees()));
  const [projects, setProjects] = useState<DiskProject[] | null>(null);
  const [session, setSession] = useState<ProjectSession | null>(null);
  const [opening, setOpening] = useState<{ name: string; done: number; total: number } | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [version, setVersion] = useState(0);
  const [log, setLog] = useState<Log[]>([]);
  const sessionRef = useRef<ProjectSession | null>(null);

  const note = useCallback((text: string, error = false) => {
    const at = new Date().toTimeString().slice(0, 8);
    if (error) console.warn(text);
    setLog((l) => [{ at, text, error }, ...l].slice(0, 30));
  }, []);

  const run = useCallback(
    async (label: string, fn: () => Promise<void>) => {
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

  useEffect(() => {
    if (!disk) return;
    setProjects(null);
    run('Buscando proyectos', async () => setProjects(await discoverProjects(disk, note)));
  }, [disk, note, run]);

  // Al pasar a segundo plano, lo pendiente vuelve al disco (D2).
  useEffect(() => {
    const sub = AppState.addEventListener('change', (state) => {
      if (state !== 'active') sessionRef.current?.flush();
    });
    return () => sub.remove();
  }, []);

  const pickDisk = () =>
    run('Eligiendo disco', async () => {
      const tree = await Saf.pickDirectory();
      if (!tree) return;
      setTrees(Saf.persistedTrees());
      setDisk(tree);
    });

  const open = (project: DiskProject) =>
    run('Abriendo proyecto', async () => {
      setOpening({ name: project.name, done: 0, total: 0 });
      const s = await ProjectSession.open(project, () => setVersion((v) => v + 1), note);
      await s.fetchThumbs((done, total) => setOpening({ name: project.name, done, total }));
      sessionRef.current = s;
      setSession(s);
      setOpening(null);
    }).finally(() => setOpening(null));

  const close = () => {
    session?.flush();
    sessionRef.current = null;
    setSession(null);
    onCleanChange(false);
  };

  // Lo que ve el culling: la vista previa si ya está en el caché, si no la miniatura.
  const photos: CullItem[] = useMemo(
    () =>
      (session?.photos ?? []).map((p) => ({
        info: p.info,
        thumb: p.thumb,
        preview: p.previewReady ? `file://${p.previewPath}` : p.thumb,
        histPath: p.previewReady ? p.previewPath : p.thumb.replace('file://', ''),
      })),
    // `version` cambia cuando llega una vista previa.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [session, version],
  );

  const backend: CullingBackend | undefined = useMemo(
    () =>
      session
        ? {
            rate: (i, stars, labels) => session.rate(i, stars, labels),
            approve: (i, approved) => session.approve(i, approved),
            onIndex: (i) => [i, i + 1, i - 1, i + 2].forEach((j) => session.ensurePreview(j)),
          }
        : undefined,
    [session],
  );

  if (importing && projects) {
    return (
      <ImportScreen
        projects={projects}
        onClose={() => setImporting(false)}
        onOpenProject={(p) => {
          setImporting(false);
          open(p);
        }}
      />
    );
  }

  if (opening) {
    return (
      <View style={[styles.root, styles.center]}>
        <ActivityIndicator color="#e9c349" size="large" />
        <Text style={styles.text}>Abriendo {opening.name}</Text>
        {opening.total > 0 && (
          <Text style={styles.muted}>
            Miniaturas {opening.done} / {opening.total}
          </Text>
        )}
      </View>
    );
  }

  if (session && photos.length > 0) {
    return <Culling key={session.project.uri} photos={photos} onCleanChange={onCleanChange} backend={backend} onBack={close} />;
  }

  return (
    <View style={styles.root}>
      <HomeScreen
        trees={trees}
        disk={disk}
        projects={projects}
        busy={busy}
        onPickDisk={pickDisk}
        onSelectDisk={setDisk}
        onOpen={open}
        onImport={() => setImporting(true)}
      />
      {log.length > 0 && (
        <View style={styles.log} pointerEvents="none">
          {log.slice(0, 4).map((l, i) => (
            <Text key={i} style={[styles.logText, l.error && styles.logError]} numberOfLines={1}>
              {l.at} {l.text}
            </Text>
          ))}
        </View>
      )}
    </View>
  );
}

// Un permiso cuyo volumen esté conectado y que no sea la DCIM de una tarjeta.
function defaultDisk(trees: string[]): string | null {
  const mounted = trees.filter((t) => {
    try {
      return Saf.exists(Saf.rootDocument(t));
    } catch {
      return false;
    }
  });
  return mounted.find((t) => !decodeURIComponent(t).endsWith('DCIM')) ?? mounted[0] ?? null;
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#131313' },
  center: { alignItems: 'center', justifyContent: 'center', gap: 12 },
  text: { color: '#fff', fontSize: 18 },
  muted: { color: '#9a9a9a', fontSize: 15, fontVariant: ['tabular-nums'] },
  log: { position: 'absolute', left: 112, bottom: 16, right: 180, gap: 2 },
  logText: { color: '#777', fontSize: 12 },
  logError: { color: '#f87171' },
});
