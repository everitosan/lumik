// Experimento 3.1 del spike (Android, D5): inicio → proyecto del disco →
// culling, e importar desde dentro del proyecto (como en el desktop), con cada
// cambio guardado en el disco por SAF. El botón "atrás" de Android regresa:
// del import al proyecto y del proyecto al inicio.
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ActivityIndicator, AppState, BackHandler, Pressable, StyleSheet, Text, View } from 'react-native';

import Saf from '../../modules/saf/src/LumikSafModule';
import { Culling, type CullingBackend, type CullItem } from '../CullingScreen';
import { type CardInfo, fetchCovers } from './covers';
import { createProject, type ProjectForm } from './createProject';
import { HomeScreen } from './HomeScreen';
import { readPrefs, savePrefs } from './prefs';
import { ImportScreen } from './ImportScreen';
import { lockMessage } from './lock';
import { discoverProjects } from './projects';
import { type DiskProject, ProjectSession } from './session';

type Log = { at: string; text: string; error?: boolean };

export default function PrototypeScreen({ onCleanChange }: { onCleanChange: (clean: boolean) => void }) {
  const [trees, setTrees] = useState<string[]>(() => diskTrees(Saf.persistedTrees()));
  const [disk, setDiskState] = useState<string | null>(() => defaultDisk(diskTrees(Saf.persistedTrees())));
  const [cards, setCards] = useState<Map<string, CardInfo>>(new Map());
  // El disco elegido se recuerda para la próxima vez.
  const setDisk = useCallback((tree: string | null) => {
    setDiskState(tree);
    if (tree) savePrefs({ disk: tree });
  }, []);
  const [projects, setProjects] = useState<DiskProject[] | null>(null);
  const [session, setSession] = useState<ProjectSession | null>(null);
  const [opening, setOpening] = useState<{ name: string; done: number; total: number } | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  // Proyecto al que se está importando.
  const [importing, setImporting] = useState<DiskProject | null>(null);
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
    run('Buscando proyectos', async () => {
      const found = await discoverProjects(disk, note);
      setProjects(found);
      // Nombres y portadas llegan después, sin frenar la lista.
      fetchCovers(found, (p, info) => setCards((c) => new Map(c).set(p.uri, info)));
    });
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
      setTrees(diskTrees(Saf.persistedTrees()));
      setDisk(tree);
    });

  const titleOf = (p: DiskProject) => cards.get(p.uri)?.title ?? p.name.split('/').pop() ?? p.name;

  const open = (project: DiskProject) =>
    run('Abriendo proyecto', async () => {
      const name = titleOf(project);
      setOpening({ name, done: 0, total: 0 });
      const s = await ProjectSession.open(project, () => setVersion((v) => v + 1), note);
      await s.fetchThumbs((done, total) => setOpening({ name, done, total }));
      sessionRef.current = s;
      setSession(s);
      setOpening(null);
    }).finally(() => setOpening(null));

  const close = () => {
    try {
      session?.close();
    } catch (e) {
      note(`Al cerrar: ${String(e)}`, true);
    }
    sessionRef.current = null;
    setSession(null);
    onCleanChange(false);
    // Las tarjetas muestran de nuevo quién tiene cada proyecto.
    if (disk) run('Buscando proyectos', async () => setProjects(await discoverProjects(disk, note)));
  };

  // Tomar el control: el candado pasa a esta tablet y se vuelve a abrir desde
  // el disco, con la base tal como la dejó el otro dispositivo.
  const takeControl = () => {
    if (!session) return;
    const project = session.project;
    ProjectSession.takeControl(project, session.mirrorDir);
    sessionRef.current = null;
    setSession(null);
    open(project);
  };

  // Importar desde el proyecto: se cierra la sesión (el import toma el candado
  // por su cuenta) y al terminar se vuelve a abrir, con las fotos nuevas.
  const startImport = () => {
    if (!session) return;
    const project = session.project;
    close();
    setImporting(project);
  };

  // Atrás (gesto o botón de Android): del proyecto al inicio. El import
  // maneja el suyo; en el inicio, el sistema hace lo de siempre.
  useEffect(() => {
    const sub = BackHandler.addEventListener('hardwareBackPress', () => {
      if (sessionRef.current) {
        close();
        return true;
      }
      return false;
    });
    return () => sub.remove();
  });

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

  if (importing) {
    const project = importing;
    return (
      <ImportScreen
        project={project}
        title={titleOf(project)}
        onClose={() => {
          setImporting(null);
          open(project);
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

  // Proyecto sin fotos (p. ej. recién creado): lo único que hay que hacer es importar.
  if (session && photos.length === 0) {
    const project = session.project;
    return (
      <View style={[styles.root, styles.center]}>
        <Text style={styles.text}>{titleOf(project)}</Text>
        <Text style={styles.muted}>Todavía no tiene fotos.</Text>
        <Pressable style={({ pressed }) => [styles.importButton, pressed && { opacity: 0.7 }]} onPress={startImport}>
          <Text style={styles.importButtonText}>Importar fotos</Text>
        </Pressable>
        <Pressable onPress={close} hitSlop={8}>
          <Text style={styles.muted}>Volver a los proyectos</Text>
        </Pressable>
      </View>
    );
  }

  if (session && photos.length > 0) {
    const lock = session.readOnly;
    return (
      <Culling
        key={session.project.uri}
        photos={photos}
        onCleanChange={onCleanChange}
        backend={backend}
        onBack={close}
        onImport={lock ? undefined : startImport}
        notice={
          lock
            ? {
                text: `${lockMessage(lock)} · solo lectura`,
                detail: lock.pending
                  ? 'Tiene cambios que no llegaron al disco: si tomas el control, esos cambios quedan fuera.'
                  : 'El disco está al día.',
                warn: lock.pending,
                actionLabel: 'Tomar el control',
                onAction: takeControl,
              }
            : undefined
        }
      />
    );
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
        cards={cards}
        onCreate={async (form: ProjectForm) => {
          const dirUri = await createProject(form);
          note(`Proyecto creado: ${form.name}`);
          // El inicio pasa al disco del proyecto nuevo y lo abre.
          const target = form.disk!;
          if (target !== disk) setDisk(target);
          const found = await discoverProjects(target, note);
          setProjects(found);
          const created = found.find((p) => p.uri === dirUri);
          if (created) open(created);
        }}
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

// Los permisos que sirven como disco: volumen conectado y que no sea una
// carpeta de la tarjeta (DCIM y lo que cuelga de ella).
function diskTrees(trees: string[]): string[] {
  return trees.filter((t) => {
    if (/(^|[:/])DCIM(\/|$)/.test(decodeURIComponent(t.split('/tree/')[1] ?? t))) return false;
    try {
      return Saf.exists(Saf.rootDocument(t));
    } catch {
      return false;
    }
  });
}

// El último disco elegido, si sigue conectado; si no, el primero.
function defaultDisk(trees: string[]): string | null {
  const last = readPrefs().disk;
  return (last && trees.includes(last) ? last : trees[0]) ?? null;
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#131313' },
  center: { alignItems: 'center', justifyContent: 'center', gap: 12 },
  text: { color: '#fff', fontSize: 18 },
  muted: { color: '#9a9a9a', fontSize: 15, fontVariant: ['tabular-nums'] },
  importButton: { minHeight: 56, paddingHorizontal: 28, justifyContent: 'center', borderRadius: 16, backgroundColor: '#3b5bdb', marginTop: 8 },
  importButtonText: { color: '#fff', fontSize: 16, fontWeight: '600' },
  log: { position: 'absolute', left: 112, bottom: 16, right: 180, gap: 2 },
  logText: { color: '#777', fontSize: 12 },
  logError: { color: '#f87171' },
});
