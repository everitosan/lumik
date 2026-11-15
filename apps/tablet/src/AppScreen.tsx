// Raíz de la app (Android). Como el desktop: inicio (proyectos
// del disco) → proyecto (grid con filtros) → foto (visor), e importar desde
// el proyecto. Cada cambio se guarda en el disco por SAF. "‹" y el botón atrás
// de Android regresan un nivel.
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ActivityIndicator, AppState, BackHandler, StyleSheet, Text, View } from 'react-native';

import Saf, { type SafVolume } from '../modules/saf/src/LumikSafModule';
import { Culling, type CullingBackend, type CullItem } from './CullingScreen';
import { type CardInfo, fetchCovers } from './covers';
import { createProject, type ProjectForm } from './createProject';
import { type DiskInfo, genericName, readDiskInfo, volumeOf, writeUserDiskName } from './diskName';
import { HomeScreen } from './HomeScreen';
import {
  checkProjectRestore,
  checkTrashedPhoto,
  editProject,
  ProjectLockedError,
  purgeProject,
  purgeTrashedPhoto,
  purgeTrashedPhotos,
  RestoreCollisionError,
  restoreTrashedPhotos,
  restoreProject,
  restoreTrashedPhoto,
  toggleArchived,
  trashProject,
} from './projectActions';
import { type DiskTrash, loadTrash, type TrashedPhotoItem } from './trash';
import { ImportScreen } from './ImportScreen';
import { lockMessage } from './lock';
import { type Filters, NO_FILTERS, ProjectScreen, type ViewMode } from './ProjectScreen';
import { discoverAll } from './projects';
import type { ViewSettings } from '@lumik/core-native';
import { type DiskProject, ProjectSession } from './session';

type Log = { at: string; text: string; error?: boolean };
/** Visor abierto: índices (en `session.photos`) que recorre y dónde empieza. */
type ViewerState = { visible: number[]; position: number };

export default function AppScreen({ onCleanChange }: { onCleanChange: (clean: boolean) => void }) {
  // Discos conectados con permiso; el inicio lista los proyectos de todos, como el desktop.
  const [trees, setTrees] = useState<string[]>(() => diskTrees(Saf.persistedTrees()));
  // Discos externos montados (para pedir acceso a los que no lo tienen y nombrarlos).
  const [volumes, setVolumes] = useState<SafVolume[]>(() => Saf.volumes());
  // Discos expulsados desde la app: no se listan hasta que se desconecten o se vuelvan a usar.
  const [ejected, setEjected] = useState<string[]>([]);
  const ejectedRef = useRef<string[]>([]);
  ejectedRef.current = ejected;
  const [cards, setCards] = useState<Map<string, CardInfo>>(new Map());
  const [projects, setProjects] = useState<DiskProject[] | null>(null);
  // Papelera de los discos conectados (spec trash-bin); `null` mientras se busca.
  const [trash, setTrash] = useState<DiskTrash[] | null>(null);
  // Proyectos ya leídos del disco (nombre, datos, portada): "Analizando disco… N de M".
  const [reading, setReading] = useState<{ done: number; total: number } | null>(null);
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
  // Disco recién con acceso y sin nombre: se pide uno (o se omite).
  const [naming, setNaming] = useState<string | null>(null);
  // Sube al guardar un nombre, para volver a leer los discos.
  const [diskVersion, setDiskVersion] = useState(0);
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
    (list: string[]) =>
      run('Buscando proyectos', async () => {
        const found = await discoverAll(list, note);
        setProjects(found);
        // Nombres y portadas llegan después, sin frenar la lista.
        fetchCovers(
          found,
          (p, info) => setCards((c) => new Map(c).set(p.uri, info)),
          (done, total) => setReading(done < total ? { done, total } : null),
        );
        // La papelera también, sin frenar la lista: decide si el rail la muestra.
        loadTrash(list, found, note)
          .then(setTrash)
          .catch((e) => note(`Papelera: ${String(e)}`, true));
      }),
    [note, run],
  );

  useEffect(() => {
    setProjects(null);
    refreshProjects(trees);
  }, [trees, refreshProjects]);

  // Vuelve a mirar qué discos hay: al conectar o quitar uno y al volver a la
  // app. Un disco que llega con la app abierta aparece solo, sin agregarlo otra vez.
  const refreshDisks = useCallback(() => {
    const mounted = Saf.volumes();
    setVolumes(mounted);
    // Un expulsado que ya se desconectó deja de estar apartado.
    const still = ejectedRef.current.filter((t) => mounted.some((v) => v.uuid === volumeOf(t)));
    if (still.length !== ejectedRef.current.length) setEjected(still);
    const next = diskTrees(Saf.persistedTrees()).filter((t) => !still.includes(t));
    setTrees((prev) => (prev.join('|') === next.join('|') ? prev : next));
  }, []);

  useEffect(() => {
    const sub = Saf.addListener('onVolumesChanged', refreshDisks);
    return () => sub.remove();
  }, [refreshDisks]);

  // Al pasar a segundo plano, lo pendiente vuelve al disco (D2); al volver, se miran los discos.
  useEffect(() => {
    const sub = AppState.addEventListener('change', (state) => {
      if (state !== 'active') sessionRef.current?.flush();
      else refreshDisks();
    });
    return () => sub.remove();
  }, [refreshDisks]);

  const pickDisk = () =>
    run('Eligiendo disco', async () => {
      const tree = await Saf.pickDirectory();
      if (!tree) return;
      setTrees(diskTrees(Saf.persistedTrees()));
    });

  // Acceso en un toque a un disco conectado que aún no lo tiene. Si no tiene
  // nombre guardado (`lumik/.disk.json`), se pide uno.
  const grantVolume = (uuid: string) =>
    run('Dando acceso al disco', async () => {
      const tree = await Saf.requestVolumeAccess(uuid);
      refreshDisks();
      const volume = volumes.find((v) => v.uuid === uuid);
      if (tree && volume && !volume.readOnly && !readDiskInfo(tree).saved) setNaming(tree);
    });

  const saveDiskName = (tree: string, name: string) => {
    writeUserDiskName(tree, name);
    setDiskVersion((v) => v + 1);
    note(`Disco nombrado: ${name.trim()}`);
  };

  // Nombre guardado y tipo de cada disco con acceso; se leen al cambiar los discos.
  const diskInfo = useMemo(() => {
    const m = new Map<string, DiskInfo>();
    for (const t of [...trees, ...ejected]) m.set(t, readDiskInfo(t));
    return m;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [trees, ejected, volumes, diskVersion]);

  const diskLabel = (tree: string) => {
    const id = decodeURIComponent(tree.split('/tree/')[1] ?? tree);
    if (id.startsWith('primary:')) return `Memoria interna · ${id.slice(8)}`;
    const info = diskInfo.get(tree) ?? readDiskInfo(tree);
    const volume = volumes.find((v) => v.uuid === volumeOf(tree));
    const count = projects?.filter((p) => p.tree === tree).length;
    return `${info.saved?.name ?? genericName(info.kind, volume, count)} · ${volumeOf(tree)}`;
  };

  // Expulsar (como el desktop): lo abierto en ese disco se cierra, lo pendiente
  // vuelve a él y se suelta el candado. Android no deja a una app desmontarlo:
  // queda apartado y se avisa que ya se puede desconectar.
  const eject = (tree: string) => {
    if (importing?.tree === tree) {
      note('Hay un import en curso en ese disco: termina antes de expulsarlo', true);
      return;
    }
    if (sessionRef.current?.project.tree === tree) close();
    const next = [...ejectedRef.current, tree];
    ejectedRef.current = next;
    setEjected(next);
    setTrees((prev) => prev.filter((t) => t !== tree));
    note(`${diskLabel(tree)}: ya puedes desconectarlo`);
  };

  const unEject = (tree: string) => {
    ejectedRef.current = ejectedRef.current.filter((t) => t !== tree);
    setEjected(ejectedRef.current);
    refreshDisks();
  };

  // Motivo de un fallo de lote para el reporte de la papelera.
  const batchReason = (e: unknown) => (e instanceof ProjectLockedError ? lockMessage(e.lock) : String(e instanceof Error ? e.message : e));

  /** Agrupa las fotos por proyecto y corre `fn` con una sola copia de su base;
   *  si el proyecto lo tiene otro dispositivo, todas sus fotos fallan con ese motivo. */
  const perProject = async (
    items: TrashedPhotoItem[],
    failed: { name: string; reason: string }[],
    fn: (p: DiskProject, list: { id: string; name: string }[]) => Promise<{ name: string; reason: string }[]>,
  ) => {
    const groups = new Map<string, { project: DiskProject; list: { id: string; name: string }[] }>();
    for (const it of items) {
      const g = groups.get(it.project.uri) ?? { project: it.project, list: [] };
      g.list.push({ id: it.info.id, name: it.info.fileName });
      groups.set(it.project.uri, g);
    }
    for (const g of groups.values()) {
      try {
        failed.push(...(await fn(g.project, g.list)));
      } catch (e) {
        const reason = batchReason(e);
        for (const ph of g.list) failed.push({ name: ph.name, reason });
      }
    }
  };

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

  const moveToTrash = (i: number) =>
    run('Moviendo a la papelera', async () => {
      const s = sessionRef.current;
      if (!s) return;
      s.trashPhoto(s.photos[i]);
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
    refreshProjects(trees);
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
  const viewerList = session && viewer ? session.photos : [];

  const items: CullItem[] = useMemo(() => {
    if (!session || !viewer) return [];
    return viewer.visible.map((i) => {
      const p = viewerList[i];
      return {
        info: p.info,
        thumb: p.thumb,
        preview: p.previewReady ? `file://${p.previewPath}` : p.thumb,
        // La miniatura ya viene girada (como en el desktop); la vista previa, no.
        previewRotated: !p.previewReady,
        // Sin vista previa todavía: la miniatura, si ya está en el caché.
        histPath: p.previewReady ? p.previewPath : p.thumb.startsWith('file://') ? p.thumb.slice(7) : '',
      };
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session, viewer, version]);

  // El visor habla en posiciones de su lista; la sesión, en índices de todas.
  const backend: CullingBackend | undefined = useMemo(() => {
    if (!session || !viewer) return undefined;
    const list = session.photos;
    const at = (j: number) => list[viewer.visible[j]];
    return {
      rate: (j, stars, labels) => session.rate(at(j), stars, labels),
      approve: (j, approved) => session.approve(at(j), approved),
      rotate: (j, delta) => session.rotate(at(j), delta),
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
        backLabel="Fotos"
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
        filters={filters}
        onFilters={setFilters}
        viewMode={viewMode}
        onViewMode={setViewMode}
        onOpenPhoto={(visible, position) => setViewer({ visible, position })}
        onTrash={lock ? undefined : moveToTrash}
        onTrashMany={
          lock
            ? undefined
            : async (indices) => {
                const s = sessionRef.current;
                if (!s) return [];
                setBusy('Moviendo a la papelera');
                try {
                  return s.trashPhotos(indices.map((i) => s.photos[i]).filter(Boolean));
                } catch (e) {
                  note(`Mover a la papelera: ${String(e)}`, true);
                  return [];
                } finally {
                  setBusy(null);
                }
              }
        }
        onReveal={(i) => {
          const s = sessionRef.current;
          const p = s && s.photos[i];
          if (!s || !p) return;
          // La carpeta donde está el archivo (`_media/`, `_culled/` o `_trash/`).
          const dir = p.info.dngPath.split('/').slice(0, -1).join('/');
          if (!Saf.openFolder(Saf.childUri(s.project.uri, dir))) note('Ninguna app de archivos puede abrir la carpeta', true);
        }}
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
        projects={projects}
        busy={busy}
        labelOf={diskLabel}
        pendingVolumes={volumes.filter((v) => !v.readOnly && !Saf.persistedTrees().some((t) => volumeOf(t) === v.uuid))}
        // Sin acceso no se puede leer el contenido: solo "Disco" y su capacidad.
        pendingLabelOf={(v) => `${genericName('Disco', v)} · ${v.uuid}`}
        onGrant={grantVolume}
        nameSourceOf={(t) => {
          if (volumes.find((v) => v.uuid === volumeOf(t))?.readOnly) return 'readonly';
          const saved = diskInfo.get(t)?.saved;
          return saved ? (saved.fromLabel ? 'label' : 'user') : 'none';
        }}
        naming={
          naming
            ? { tree: naming, suggestion: diskLabel(naming).replace(/ · [^·]+$/, ''), current: diskInfo.get(naming)?.saved?.name }
            : null
        }
        onRename={setNaming}
        onSaveName={saveDiskName}
        onCloseNaming={() => setNaming(null)}
        ejected={ejected}
        onEject={eject}
        onUnEject={unEject}
        onStorageSettings={() => Saf.openStorageSettings()}
        actions={{
          archive: async (p, archived, force) => {
            await toggleArchived(p, archived, force);
            note(`${titleOf(p)}: ${archived ? 'archivado' : 'desarchivado'}`);
            refreshProjects(trees);
          },
          edit: async (p, form, force) => {
            const out = await editProject(p, form, force);
            note(`${form.name.trim()}: guardado${out.renamed ? ` · ${out.renamed} archivos renombrados` : ''}${out.moved ? ' · carpeta movida' : ''}`);
            refreshProjects(trees);
            return out;
          },
          openFolder: (p) => {
            if (!Saf.openFolder(p.uri)) note('Ninguna app de archivos puede abrir la carpeta', true);
          },
          trash: async (p, force) => {
            const timing = await trashProject(p, force);
            note(`${titleOf(p)}: en la papelera en ${timing}`);
            refreshProjects(trees);
          },
        }}
        trash={trash}
        reading={reading}
        trashActions={{
          restoreProject: async (item, newName) => {
            await restoreProject(item.project, newName);
            note(`${newName ?? item.card?.name ?? item.project.name}: restaurado`);
            refreshProjects(trees);
          },
          checkProject: (item, newName) => checkProjectRestore(item.project, newName),
          purgeProject: async (item) => {
            await purgeProject(item.project);
            note(`${item.card?.name ?? item.project.name}: eliminado definitivamente`);
            refreshProjects(trees);
          },
          restorePhoto: async (item, newStem, force) => {
            await restoreTrashedPhoto(item.project, item.info.id, newStem, force);
            note(`${item.info.fileName}: restaurada en «${item.projectName}»`);
            refreshProjects(trees);
          },
          checkPhoto: (item, newStem) => checkTrashedPhoto(item.project, item.info.id, newStem),
          purgePhoto: async (item, force) => {
            await purgeTrashedPhoto(item.project, item.info.id, force);
            note(`${item.info.fileName}: eliminada definitivamente`);
            refreshProjects(trees);
          },
          restoreMany: async (projectItems, photoItems) => {
            const failed: { name: string; reason: string }[] = [];
            for (const item of projectItems) {
              try {
                await restoreProject(item.project);
              } catch (e) {
                failed.push({ name: item.card?.name ?? item.project.name, reason: e instanceof RestoreCollisionError ? 'collision' : batchReason(e) });
              }
            }
            await perProject(photoItems, failed, (p, list) => restoreTrashedPhotos(p, list));
            note(`Restaurados ${projectItems.length + photoItems.length - failed.length}${failed.length ? ` · ${failed.length} siguen en la papelera` : ''}`, failed.length > 0);
            refreshProjects(trees);
            return failed;
          },
          purgeMany: async (projectItems, photoItems) => {
            const failed: { name: string; reason: string }[] = [];
            for (const item of projectItems) {
              try {
                await purgeProject(item.project);
              } catch (e) {
                failed.push({ name: item.card?.name ?? item.project.name, reason: batchReason(e) });
              }
            }
            await perProject(photoItems, failed, (p, list) => purgeTrashedPhotos(p, list));
            note(`Eliminados ${projectItems.length + photoItems.length - failed.length}${failed.length ? ` · ${failed.length} sin eliminar` : ''}`, failed.length > 0);
            refreshProjects(trees);
            return failed;
          },
          emptyDisk: async (disk) => {
            const failed: { name: string; reason: string }[] = [];
            const reason = (e: unknown) => (e instanceof ProjectLockedError ? lockMessage(e.lock) : String(e));
            for (const item of disk.projects) {
              try {
                await purgeProject(item.project);
              } catch (e) {
                failed.push({ name: item.card?.name ?? item.project.name, reason: reason(e) });
              }
            }
            // Una sola ida y vuelta de `project.db` por proyecto, como el lote de marcadas.
            await perProject(disk.photos, failed, (p, list) => purgeTrashedPhotos(p, list));
            note(`Papelera de ${diskLabel(disk.tree)} vaciada${failed.length ? ` · ${failed.length} sin eliminar` : ''}`, failed.length > 0);
            refreshProjects(trees);
            return failed;
          },
        }}
        onPickDisk={pickDisk}
        onOpen={open}
        cards={cards}
        onCreate={async (form: ProjectForm) => {
          const dirUri = await createProject(form);
          note(`Proyecto creado: ${form.name}`);
          // Se vuelve a leer y se abre el proyecto nuevo.
          const found = await discoverAll(trees, note);
          setProjects(found);
          const created = found.find((p) => p.uri === dirUri);
          if (created) {
            // Su nombre ya se sabe: no hace falta esperar a que lleguen los datos de la tarjeta.
            setCards((c) => new Map(c).set(created.uri, { title: form.name.trim(), cover: false }));
            open(created);
          }
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

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#131313' },
  center: { alignItems: 'center', justifyContent: 'center', gap: 12 },
  text: { color: '#fff', fontSize: 18 },
  // Encima del pie del inicio (archivados y orden).
  log: { position: 'absolute', left: 112, bottom: 76, right: 180, gap: 2 },
  logText: { color: '#777', fontSize: 12 },
  logError: { color: '#ffb4ab' },
});
