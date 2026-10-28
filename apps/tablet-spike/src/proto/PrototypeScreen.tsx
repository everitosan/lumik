// Experimento 3.1 del spike (Android, D5). Como el desktop: inicio (proyectos
// del disco) → proyecto (grid con filtros) → foto (visor), e importar desde
// el proyecto. Cada cambio se guarda en el disco por SAF. "‹" y el botón atrás
// de Android regresan un nivel.
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ActivityIndicator, AppState, BackHandler, StyleSheet, Text, View } from 'react-native';

import Saf from '../../modules/saf/src/LumikSafModule';
import { Culling, type CullingBackend, type CullItem } from '../CullingScreen';
import { type CardInfo, fetchCovers } from './covers';
import { createProject, type ProjectForm } from './createProject';
import { HomeScreen } from './HomeScreen';
import { ImportScreen } from './ImportScreen';
import { lockMessage } from './lock';
import { readPrefs, savePrefs } from './prefs';
import { type Filters, NO_FILTERS, ProjectScreen, type ViewMode } from './ProjectScreen';
import { discoverProjects } from './projects';
import type { ViewSettings } from '@lumik/core-native';
import { type DiskProject, ProjectSession } from './session';

type Log = { at: string; text: string; error?: boolean };
/** Visor abierto: índices (en `session.photos` o en la papelera) que recorre y dónde empieza. */
type ViewerState = { fromTrash: boolean; visible: number[]; position: number };

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
  const [opening, setOpening] = useState<string | null>(null);
  const [fetching, setFetching] = useState<{ done: number; total: number } | null>(null);
  const [filters, setFiltersState] = useState<Filters>(NO_FILTERS);
  const [viewMode, setViewModeState] = useState<ViewMode>('by-date');
  const [viewer, setViewer] = useState<ViewerState | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  // Proyecto al que se está importando.
  const [importing, setImporting] = useState<DiskProject | null>(null);
  const [version, setVersion] = useState(0);
  const [log, setLog] = useState<Log[]>([]);
  const sessionRef = useRef<ProjectSession | null>(null);
  // Como el desktop: un proyecto sin fotos abre el import, una sola vez.
  const autoImported = useRef(new Set<string>());

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

  const refreshProjects = useCallback(
    (tree: string) =>
      run('Buscando proyectos', async () => {
        const found = await discoverProjects(tree, note);
        setProjects(found);
        // Nombres y portadas llegan después, sin frenar la lista.
        fetchCovers(found, (p, info) => setCards((c) => new Map(c).set(p.uri, info)));
      }),
    [note, run],
  );

  useEffect(() => {
    if (!disk) return;
    setProjects(null);
    refreshProjects(disk);
  }, [disk, refreshProjects]);

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

  // Abrir: la base y el candado, y a la vista; las miniaturas que falten se
  // copian al caché en segundo plano (mientras tanto se ven del disco).
  const open = (project: DiskProject) =>
    run('Abriendo proyecto', async () => {
      setOpening(titleOf(project));
      try {
        const s = await ProjectSession.open(project, () => setVersion((v) => v + 1), note);
        sessionRef.current = s;
        // Los filtros y el modo de vista con que el desktop dejó el proyecto (la papelera empieza apagada).
        setFiltersState(fromSettings(s.viewSettings));
        setViewModeState(s.viewSettings.viewMode === 'grid' ? 'grid' : 'by-date');
        setViewer(null);
        setSession(s);
        s.fetchThumbs((done, total) => setFetching(done < total ? { done, total } : null))
          .catch((e) => note(`Miniaturas: ${String(e)}`, true))
          .finally(() => setFetching(null));
        if (!s.readOnly && s.photos.length === 0 && !autoImported.current.has(project.uri)) {
          autoImported.current.add(project.uri);
          s.close();
          sessionRef.current = null;
          setSession(null);
          setImporting(project);
        }
      } finally {
        setOpening(null);
      }
    });

  // Cada cambio de filtros o de vista se guarda en el proyecto, como el desktop.
  const saveView = (f: Filters, mode: ViewMode) => {
    try {
      sessionRef.current?.saveView(toSettings(f, mode));
    } catch (e) {
      note(`Guardar los filtros: ${String(e)}`, true);
    }
  };
  const setFilters = (f: Filters) => {
    setFiltersState(f);
    saveView(f, viewMode);
  };
  const setViewMode = (mode: ViewMode) => {
    setViewModeState(mode);
    saveView(filters, mode);
  };

  const trashOrRestore = (restore: boolean, i: number) =>
    run(restore ? 'Restaurando' : 'Moviendo a la papelera', async () => {
      const s = sessionRef.current;
      if (!s) return;
      if (restore) s.restorePhoto(s.trash[i]);
      else s.trashPhoto(s.photos[i]);
    });

  const closeViewer = () => {
    setViewer(null);
    onCleanChange(false);
  };

  const close = () => {
    try {
      sessionRef.current?.close();
    } catch (e) {
      note(`Al cerrar: ${String(e)}`, true);
    }
    sessionRef.current = null;
    setSession(null);
    setViewer(null);
    onCleanChange(false);
    // Las tarjetas muestran de nuevo quién tiene cada proyecto.
    if (disk) refreshProjects(disk);
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

  // Atrás (gesto o botón de Android): del visor al grid y del grid al inicio.
  // El import maneja el suyo; en el inicio, el sistema hace lo de siempre.
  useEffect(() => {
    const sub = BackHandler.addEventListener('hardwareBackPress', () => {
      if (viewer) {
        closeViewer();
        return true;
      }
      if (sessionRef.current) {
        close();
        return true;
      }
      return false;
    });
    return () => sub.remove();
  });

  // El grid y el visor leen `session.photos`, que cambia en su lugar: `version` avisa.
  const grid = useMemo(
    () => (session?.photos ?? []).map((p) => ({ info: p.info, thumb: p.thumb })),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [session, version],
  );
  const trashGrid = useMemo(
    () => (session?.trash ?? []).map((p) => ({ info: p.info, thumb: p.thumb })),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [session, version],
  );
  const viewerList = session && viewer ? (viewer.fromTrash ? session.trash : session.photos) : [];

  const items: CullItem[] = useMemo(() => {
    if (!session || !viewer) return [];
    return viewer.visible.map((i) => {
      const p = viewerList[i];
      return {
        info: p.info,
        thumb: p.thumb,
        preview: p.previewReady ? `file://${p.previewPath}` : p.thumb,
        // Sin vista previa todavía: la miniatura, si ya está en el caché.
        histPath: p.previewReady ? p.previewPath : p.thumb.startsWith('file://') ? p.thumb.slice(7) : '',
      };
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session, viewer, version]);

  // El visor habla en posiciones de su lista; la sesión, en índices de todas.
  const backend: CullingBackend | undefined = useMemo(() => {
    if (!session || !viewer) return undefined;
    const list = viewer.fromTrash ? session.trash : session.photos;
    const at = (j: number) => list[viewer.visible[j]];
    return {
      rate: (j, stars, labels) => session.rate(at(j), stars, labels),
      approve: (j, approved) => session.approve(at(j), approved),
      onIndex: (j) => [j, j + 1, j - 1, j + 2].forEach((k) => k >= 0 && k < viewer.visible.length && session.ensurePreview(at(k))),
    };
  }, [session, viewer]);

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
        <Text style={styles.text}>Abriendo {opening}</Text>
      </View>
    );
  }

  if (session && viewer && items.length > 0) {
    const lock = session.readOnly;
    return (
      <Culling
        key={`${session.project.uri}-${viewer.visible[viewer.position]}`}
        photos={items}
        initialIndex={viewer.position}
        onCleanChange={onCleanChange}
        backend={backend}
        onBack={closeViewer}
        backLabel="‹ Fotos"
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

  if (session) {
    const lock = session.readOnly;
    return (
      <ProjectScreen
        title={titleOf(session.project)}
        photos={grid}
        trash={trashGrid}
        filters={filters}
        onFilters={setFilters}
        viewMode={viewMode}
        onViewMode={setViewMode}
        onOpenPhoto={(fromTrash, visible, position) => setViewer({ fromTrash, visible, position })}
        onTrash={lock ? undefined : (i) => trashOrRestore(false, i)}
        onRestore={lock ? undefined : (i) => trashOrRestore(true, i)}
        onBack={close}
        onImport={lock ? undefined : startImport}
        fetching={fetching}
        lock={lock ? { text: `${lockMessage(lock)} · solo lectura`, onTakeControl: takeControl } : undefined}
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

const list = (v?: string) => (v ?? '').split(',').map((x) => x.trim()).filter(Boolean);

function fromSettings(s: ViewSettings): Filters {
  return {
    approvedOnly: s.showCulled,
    trash: false,
    minStars: s.minStars ?? null,
    starsMode: s.starsFilterMode === 'exact' ? 'exact' : 'inclusive',
    colors: new Set(list(s.selectedColors).map(Number)),
    tags: new Set(list(s.selectedTags)),
  };
}

function toSettings(f: Filters, mode: ViewMode): ViewSettings {
  return {
    showCulled: f.approvedOnly,
    minStars: f.minStars ?? undefined,
    starsFilterMode: f.starsMode,
    selectedTags: [...f.tags].join(','),
    selectedColors: [...f.colors].join(','),
    viewMode: mode,
  };
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
  log: { position: 'absolute', left: 112, bottom: 16, right: 180, gap: 2 },
  logText: { color: '#777', fontSize: 12 },
  logError: { color: '#f87171' },
});
