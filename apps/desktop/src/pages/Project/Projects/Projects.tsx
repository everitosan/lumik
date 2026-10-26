import { useState, useMemo, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { ProjectCard, Button, Icon, FilterBar, FilterBarDivider } from '@lumik/ui';
import { useProjectsDashboard, useActivePhotographer, useConnectedDevices, useCoverThumbnails, useContextKeybindings, useDashboardShowArchived, useDashboardSort, matchesKey, usePlatform } from '../../../lib/hooks';
import { createProject, ensureProjectSynced, setProjectArchived, relocateBackup, deleteProject, openProjectFolder, openExternalUrl, getBackupFolderLink, takeProjectControl, closeProjectSession } from '../../../lib/api';
import { TakeControlDialog } from '../../../components/ProjectLockNotice';
import { SearchBox, looksLikeOperators } from '../../Search/SearchBox';
import { PhotoSearchResults } from '../../Search/PhotoSearchResults';
import { searchPhotos } from '../../../lib/api';
import { CreateProjectModal, type ProjectFormData } from '../../../components/CreateProjectModal';
import { EditProjectModal } from './EditProjectModal';
import { DeleteProjectModal } from './DeleteProjectModal';
import type { DashboardSort, ProjectDashboard, SearchSnapshot, SortDirection } from '../../../lib/types';

const containerStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  height: '100%',
  overflow: 'hidden',
};

// Cabecera + cuadrícula; la barra de filtros queda debajo, a todo el ancho.
const contentStyles: React.CSSProperties = {
  flex: 1,
  minHeight: 0,
  display: 'flex',
  flexDirection: 'column',
  padding: '24px 32px',
  gap: '24px',
  overflow: 'hidden',
};

const headerStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  flexShrink: 0,
};

const headerSectionStyles: React.CSSProperties = {
  flex: 1,
  display: 'flex',
  alignItems: 'center',
};

const headerLeftStyles: React.CSSProperties = {
  ...headerSectionStyles,
  flexDirection: 'column',
  alignItems: 'flex-start',
  gap: '4px',
};

const headerCenterStyles: React.CSSProperties = {
  ...headerSectionStyles,
  justifyContent: 'center',
};

const headerRightStyles: React.CSSProperties = {
  ...headerSectionStyles,
  justifyContent: 'flex-end',
  gap: '12px',
};

// Sort menu (mismo lenguaje visual que los filtros del detalle de proyecto)
const sortMenuWrapperStyles: React.CSSProperties = {
  position: 'relative',
};

const sortTriggerStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  padding: '8px 12px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  background: 'transparent',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  cursor: 'pointer',
  whiteSpace: 'nowrap',
};

const sortPanelStyles: React.CSSProperties = {
  position: 'absolute',
  // Vive en la barra inferior: abre hacia arriba.
  bottom: '100%',
  right: 0,
  marginBottom: '4px',
  padding: '8px',
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  minWidth: '240px',
  backgroundColor: 'var(--lumik-surface-container, #2d2d2d)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  boxShadow: '0 4px 12px rgba(0, 0, 0, 0.4)',
  zIndex: 1000,
};

const sortGroupLabelStyles: React.CSSProperties = {
  padding: '4px 8px 0',
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '11px',
  fontWeight: 500,
  color: 'var(--lumik-outline, #8c90a0)',
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
};

const sortOptionStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '6px 8px',
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  borderRadius: '4px',
  cursor: 'pointer',
};

const sortOptionSelectedStyles: React.CSSProperties = {
  ...sortOptionStyles,
  backgroundColor: 'var(--lumik-surface-bright, #3a3a3a)',
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const sortRadioStyles: React.CSSProperties = {
  margin: 0,
  accentColor: 'var(--lumik-primary, #b0c6ff)',
  cursor: 'pointer',
};

const archivedToggleActiveStyles: React.CSSProperties = {
  ...sortTriggerStyles,
  color: 'var(--lumik-tertiary, #ffb690)',
  borderColor: 'var(--lumik-tertiary, #ffb690)',
};

/** Compara dos claves de fecha ISO en la dirección indicada. */
const compareIn = (direction: SortDirection, a: string, b: string) =>
  direction === 'asc' ? a.localeCompare(b) : b.localeCompare(a);

interface SortMenuProps {
  sort: DashboardSort;
  onChange: (sort: DashboardSort) => void;
}

/** Botón "Ordenar" con el orden de los años y de los proyectos de cada año. */
function SortMenu({ sort, onChange }: SortMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleKey);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKey);
    };
  }, [open]);

  const groups: { key: keyof DashboardSort; label: string }[] = [
    { key: 'years', label: t('dashboard.sort.years') },
    { key: 'projects', label: t('dashboard.sort.projects') },
  ];
  const options: { value: SortDirection; label: string }[] = [
    { value: 'desc', label: t('dashboard.sort.newestFirst') },
    { value: 'asc', label: t('dashboard.sort.oldestFirst') },
  ];

  return (
    <div ref={ref} style={sortMenuWrapperStyles}>
      <button
        type="button"
        style={sortTriggerStyles}
        onClick={() => setOpen((v) => !v)}
        aria-haspopup="true"
        aria-expanded={open}
      >
        <Icon name="sort" size="sm" />
        {t('dashboard.sort.label')}
        <Icon name="chevron-down" size="sm" />
      </button>
      {open && (
        <div style={sortPanelStyles} role="group" aria-label={t('dashboard.sort.label')}>
          {groups.map((group) => (
            <div key={group.key} role="radiogroup" aria-label={group.label}>
              <div style={sortGroupLabelStyles}>{group.label}</div>
              {options.map((option) => {
                const selected = sort[group.key] === option.value;
                return (
                  <label key={option.value} style={selected ? sortOptionSelectedStyles : sortOptionStyles}>
                    <input
                      type="radio"
                      name={`dashboard-sort-${group.key}`}
                      checked={selected}
                      onChange={() => onChange({ ...sort, [group.key]: option.value })}
                      style={sortRadioStyles}
                    />
                    {option.label}
                  </label>
                );
              })}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const titleStyles: React.CSSProperties = {
  fontSize: '28px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  margin: 0,
};

const statsStyles: React.CSSProperties = {
  display: 'flex',
  gap: '24px',
  fontSize: '14px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const statItemStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
};

const statValueStyles: React.CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontWeight: 500,
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const gridContainerStyles: React.CSSProperties = {
  flex: 1,
  overflow: 'auto',
  paddingRight: '8px',
};

const gridStyles: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
  gap: '24px',
  paddingBottom: '24px',
};

const yearSeparatorStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'flex-end',
  paddingBottom: '20px',
  borderBottom: '1px solid rgba(255, 255, 255, 0.2)',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '18px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  marginBottom: '24px',
};

const yearSectionStyles: React.CSSProperties = {
  marginBottom: '40px',
};

const loadingStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  height: '200px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const emptyStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  height: '200px',
  gap: '16px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

interface ProjectsProps {
  onProjectClick?: (project: ProjectDashboard) => void;
  /** Búsqueda de fotos en curso (vive en App para volver a ella desde el visor). */
  photoSearch?: SearchSnapshot | null;
  onPhotoSearch?: (search: SearchSnapshot | null) => void;
}

export function Projects({ onProjectClick, photoSearch = null, onPhotoSearch }: ProjectsProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState(photoSearch?.query ?? '');
  // Búsqueda de fotos (spec photo-search): Enter, operadores o el panel avanzado.
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  // Visor de resultados abierto: sus teclas no son del dashboard.
  const [viewerOpen, setViewerOpen] = useState(false);
  const photoMode = photoSearch !== null || searching || searchError !== null;

  const runPhotoSearch = async (query: string) => {
    setSearching(true);
    setSearchError(null);
    try {
      const result = await searchPhotos(query);
      onPhotoSearch?.({ query, result, scrollTop: 0 });
    } catch (err) {
      setSearchError(String(err));
    } finally {
      setSearching(false);
    }
  };

  const handleSearchChange = (value: string) => {
    setSearchQuery(value);
    if (!value.trim()) closePhotoSearch();
  };

  const closePhotoSearch = () => {
    setSearchError(null);
    onPhotoSearch?.(null);
  };
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const [editingProject, setEditingProject] = useState<ProjectDashboard | null>(null);
  const [deletingProject, setDeletingProject] = useState<ProjectDashboard | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  // Acción pedida sobre un proyecto que tiene otra instalación: espera a que
  // se confirme tomar el control (spec project-lock).
  const [lockedAction, setLockedAction] = useState<{ project: ProjectDashboard; run: () => void } | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const platform = usePlatform();
  // Opening a folder in a file manager isn't available on Android.
  const canOpenFolder = platform !== 'android';

  const kb = useContextKeybindings('projects');
  const { sort, setSort } = useDashboardSort();
  const { showArchived, setShowArchived } = useDashboardShowArchived();

  useEffect(() => {
    if (showCreateModal || viewerOpen) return;
    function onKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (matchesKey(e, kb['projects.new_project'])) {
        e.preventDefault();
        setShowCreateModal(true);
      }
      if (matchesKey(e, kb['projects.focus_search'])) {
        e.preventDefault();
        searchRef.current?.focus();
      }
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [showCreateModal, viewerOpen, kb]);

  const { data: projects, loading, error, refetch } = useProjectsDashboard();
  const { data: photographer } = useActivePhotographer();
  const { data: devices } = useConnectedDevices();

  // Al conectar o expulsar un disco, la búsqueda abierta se repite en silencio
  // para que cada resultado sepa si su disco está (se conserva la posición).
  const devicesKey = useMemo(() => (devices ? devices.map((d) => d.uuid).sort().join(',') : null), [devices]);
  const lastDevicesKey = useRef<string | null>(null);
  useEffect(() => {
    if (devicesKey === null) return;
    const changed = lastDevicesKey.current !== null && lastDevicesKey.current !== devicesKey;
    lastDevicesKey.current = devicesKey;
    if (!changed || !photoSearch) return;
    searchPhotos(photoSearch.query)
      .then((result) => onPhotoSearch?.({ ...photoSearch, result }))
      .catch(() => undefined);
  }, [devicesKey]); // eslint-disable-line react-hooks/exhaustive-deps
  const coverThumbnails = useCoverThumbnails(projects);

  // Refetch projects whenever the set of connected device UUIDs changes — a
  // device appearing (new projects to show) or disappearing (e.g. the user
  // ejected it, so its projects must vanish from the grid). scan_connected_devices
  // already called refresh_open_projects on the Rust side, so we just re-query
  // get_projects_dashboard to mirror the current open_projects map.
  const knownDeviceUuids = useRef<Set<string>>(new Set());
  useEffect(() => {
    if (!devices) return;
    const current = new Set(devices.map((d) => d.uuid));
    const prev = knownDeviceUuids.current;
    const changed =
      current.size !== prev.size ||
      [...current].some((uuid) => !prev.has(uuid));
    knownDeviceUuids.current = current;
    if (changed) refetch();
  }, [devices, refetch]);

  // Archivados ocultos salvo que se elija mostrarlos.
  const shownProjects = useMemo(
    () => (projects ?? []).filter((project) => showArchived || !project.archived),
    [projects, showArchived]
  );

  const archivedCount = useMemo(
    () => (projects ?? []).filter((project) => project.archived).length,
    [projects]
  );

  // Al escribir se filtran proyectos por nombre; con operadores no, porque
  // esa consulta es de fotos y se resuelve con Enter.
  const filteredProjects = useMemo(
    () =>
      looksLikeOperators(searchQuery)
        ? shownProjects
        : shownProjects.filter((project) =>
            project.name.toLowerCase().includes(searchQuery.toLowerCase())
          ),
    [shownProjects, searchQuery]
  );

  const projectsByYear = useMemo(() => {
    const groups = new Map<string, ProjectDashboard[]>();
    for (const project of filteredProjects) {
      const raw = project.session_date ?? project.created_at;
      const year = raw ? raw.slice(0, 4) : 'sin-fecha';
      const bucket = groups.get(year);
      if (bucket) bucket.push(project);
      else groups.set(year, [project]);
    }
    return [...groups.entries()]
      .sort(([a], [b]) => {
        if (a === 'sin-fecha') return 1;
        if (b === 'sin-fecha') return -1;
        return compareIn(sort.years, a, b);
      })
      .map(([year, list]) => [
        year,
        [...list].sort((a, b) => {
          const da = a.session_date ?? a.created_at;
          const db = b.session_date ?? b.created_at;
          // Mismo día: desempata por fecha de creación en la misma dirección.
          return compareIn(sort.projects, da, db) || compareIn(sort.projects, a.created_at, b.created_at);
        }),
      ] as [string, typeof list]);
  }, [filteredProjects, sort]);

  const totalPhotos = useMemo(
    () => shownProjects.reduce((sum, p) => sum + p.photo_count, 0),
    [shownProjects]
  );

  const handleToggleArchive = (project: ProjectDashboard) => {
    void setProjectArchived(project.id, !project.archived)
      .then(() => refetch())
      .catch((err) => console.error('setProjectArchived failed:', err));
  };

  /** Corre `run` sobre el proyecto, o pide antes tomar el control si es de otro. */
  const withControl = (project: ProjectDashboard, run: () => void) => {
    if (project.locked_by) setLockedAction({ project, run });
    else run();
  };

  // Desde el dashboard no hay vista abierta: se toma el control y se suelta
  // enseguida, así el proyecto queda libre y la acción sigue con normalidad.
  const handleTakeControl = async () => {
    if (!lockedAction) return;
    const { project, run } = lockedAction;
    await takeProjectControl(project.id);
    await closeProjectSession(project.id);
    setLockedAction(null);
    refetch();
    run();
  };

  const handleProjectClick = (project: ProjectDashboard) => {
    onProjectClick?.(project);
  };

  const handleOpenFolder = async (project: ProjectDashboard) => {
    try {
      await openProjectFolder(project.id);
    } catch (err) {
      console.error('open folder failed', err);
      alert(t('project.openFolderError', { error: String(err) }));
    }
  };

  const handleEdited = (projectId: string) => {
    refetch();
    // Best-effort en segundo plano, sin bloquear ni fallar la edición local:
    // mueve la carpeta del backup en la nube y actualiza el catálogo del server
    // (nombre y fecha que ve el portal web).
    void relocateBackup(projectId).catch(() => {});
    void ensureProjectSynced(projectId).catch(() => {});
  };

  const handleDelete = async () => {
    if (!deletingProject) return;
    setIsDeleting(true);
    setDeleteError(null);
    try {
      await deleteProject(deletingProject.id);
      setDeletingProject(null);
      refetch();
    } catch (err) {
      setDeleteError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsDeleting(false);
    }
  };

  const handleCreateProject = async (data: ProjectFormData) => {
    setIsCreating(true);
    setCreateError(null);

    try {
      await createProject({
        name: data.name,
        description: data.description || undefined,
        session_date: data.sessionDate || undefined,
        creator_id: photographer!.id,
        device_uuid: data.deviceUuid,
      });

      setShowCreateModal(false);
      refetch();
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : t('import.errors.createProject'));
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <div style={containerStyles}>
      <div style={contentStyles}>
        <div style={headerStyles}>
          <div style={headerLeftStyles}>
            <h1 style={titleStyles}>{t('dashboard.title')}</h1>
            <div style={statsStyles}>
              <span style={statItemStyles}>
                <span style={statValueStyles}>{shownProjects.length}</span> {t('dashboard.statistics.projects')}
              </span>
              <span style={statItemStyles}>
                <span style={statValueStyles}>{totalPhotos.toLocaleString()}</span> {t('dashboard.statistics.photos')}
              </span>
            </div>
          </div>
          <div style={headerCenterStyles}>
            <SearchBox
              inputRef={searchRef}
              value={searchQuery}
              onChange={handleSearchChange}
              onSearch={(q) => void runPhotoSearch(q)}
            />
          </div>
          <div style={headerRightStyles}>
            <Button
              variant="primary"
              leftIcon={<Icon name="plus" size="sm" />}
              onClick={() => setShowCreateModal(true)}
            >
              {t('dashboard.newProject')}
            </Button>
          </div>
        </div>

        {photoMode ? (
          <PhotoSearchResults
            key={photoSearch?.query ?? ''}
            query={photoSearch?.query ?? searchQuery}
            result={photoSearch?.result ?? null}
            searching={searching}
            error={searchError}
            initialScrollTop={photoSearch?.scrollTop ?? 0}
            onScrollSave={(scrollTop) => { if (photoSearch) onPhotoSearch?.({ ...photoSearch, scrollTop }); }}
            onViewerChange={setViewerOpen}
            onClose={() => { setSearchQuery(''); closePhotoSearch(); }}
          />
        ) : (
        <div style={gridContainerStyles}>
          {loading && (
            <div style={loadingStyles}>{t('dashboard.loading')}</div>
          )}

          {error && (
            <div style={loadingStyles}>{t('dashboard.error')}: {error}</div>
          )}

          {!loading && !error && filteredProjects.length === 0 && (
            <div style={emptyStyles}>
              <span>{t('dashboard.empty')}</span>
              <span style={{ fontSize: '14px' }}>
                {t('dashboard.emptyHint')}
              </span>
            </div>
          )}

          {!loading && !error && projectsByYear.map(([year, yearProjects]) => (
            <div key={year} style={yearSectionStyles}>
              <div style={yearSeparatorStyles}>
                {year === 'sin-fecha' ? t('dashboard.noDate') : year}
              </div>
              <div style={gridStyles}>
                {yearProjects.map((project) => (
                  <ProjectCard
                    key={project.id}
                    name={project.name}
                    photoCount={project.photo_count}
                    date={project.session_date ?? project.created_at}
                    status={project.workflow_status}
                    driveName={devices?.find((d) => d.uuid === project.device_uuid)?.name ?? project.device_uuid}
                    thumbnailUrl={coverThumbnails[project.id]}
                    providers={(project.backup_providers ?? '').split(',').map((s) => s.trim()).filter(Boolean)}
                    onProviderOpen={(url, provider) => {
                      // Enlace expirable a la carpeta si el proveedor lo soporta;
                      // si no, abre la web del proveedor.
                      void getBackupFolderLink(project.id, provider)
                        .then((link) => openExternalUrl(link ?? url))
                        .catch(() => openExternalUrl(url));
                    }}
                    onClick={() => handleProjectClick(project)}
                    onOpenFolder={canOpenFolder ? () => handleOpenFolder(project) : undefined}
                    onEdit={() => withControl(project, () => setEditingProject(project))}
                    onToggleArchive={() => withControl(project, () => handleToggleArchive(project))}
                    onDelete={() => withControl(project, () => setDeletingProject(project))}
                    lockedBy={project.locked_by?.name}
                    archived={project.archived}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>
        )}
      </div>

      <FilterBar
        end={
          <>
            {archivedCount > 0 && (
              <>
                <button
                  type="button"
                  style={showArchived ? archivedToggleActiveStyles : sortTriggerStyles}
                  onClick={() => setShowArchived(!showArchived)}
                  aria-pressed={showArchived}
                  title={showArchived ? t('dashboard.archived.hide') : t('dashboard.archived.show')}
                >
                  <Icon name="archive" size="sm" />
                  {t('dashboard.archived.label', { count: archivedCount })}
                </button>
                <FilterBarDivider />
              </>
            )}
            <SortMenu sort={sort} onChange={setSort} />
          </>
        }
      />

      <CreateProjectModal
        open={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        onSubmit={handleCreateProject}
        devices={devices ?? []}
        loading={isCreating}
        error={createError}
      />

      <EditProjectModal
        project={editingProject}
        onClose={() => setEditingProject(null)}
        onSaved={(result) => handleEdited(result.project.id)}
      />

      <TakeControlDialog
        holder={lockedAction?.project.locked_by ?? null}
        onClose={() => setLockedAction(null)}
        onTakeControl={handleTakeControl}
      />

      <DeleteProjectModal
        open={deletingProject !== null}
        projectName={deletingProject?.name ?? ''}
        onClose={() => { setDeletingProject(null); setDeleteError(null); }}
        onConfirm={handleDelete}
        loading={isDeleting}
        error={deleteError}
      />
    </div>
  );
}
