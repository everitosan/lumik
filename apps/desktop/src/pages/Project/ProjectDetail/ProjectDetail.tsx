import { useState, useMemo, useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { CSSProperties } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { PhotoCard, PHOTO_CARD_HEADER_HEIGHT, PHOTO_CARD_FOOTER_HEIGHT, photoFormat, type ColorLabel } from '@lumik/ui';
import type { Photo, ProjectAccess, ProjectSettings } from '../../../lib/types';
import { useProjectPhotos, useProjectTrashedPhotos, useProjectThumbnails, useContextKeybindings, matchesKey, usePlatform } from '../../../lib/hooks';
import * as api from '../../../lib/api';
import { ProjectDetailHeader } from './ProjectDetailHeader';
import { ProjectDetailFooter } from './ProjectDetailFooter';
import { useBackup } from '../../../lib/BackupContext';
import { TrashPhotoModal } from './TrashPhotoModal';
import { ImportPage } from '../../ImportPage';
import { PhotoDetail } from '../PhotoDetail';
import { ProjectLockBanner } from '../../../components/ProjectLockNotice';

export interface ProjectDetailProps {
  projectId: string;
  projectName: string;
  deviceUuid: string;
  coverPhotoPath: string | null;
  /** Foto que se abre en el visor al entrar (al venir de la búsqueda). */
  initialPhotoId?: string | null;
  onBack?: () => void;
  onCoverPhotoChange?: (photoId: string | null) => void;
}

function parseColorLabels(raw: string | null): ColorLabel[] {
  if (!raw) return [];
  return raw
    .split(',')
    .map((n) => parseInt(n.trim(), 10))
    .filter((n) => n >= 1 && n <= 5) as ColorLabel[];
}

function fileNameOf(path: string): string {
  return path.split('/').pop() ?? path;
}

type SortOption = 'date' | 'name' | 'stars';

function sortPhotos(photos: Photo[], sortBy: SortOption): Photo[] {
  return [...photos].sort((a, b) => {
    switch (sortBy) {
      case 'date':
        return (a.capture_date ?? '').localeCompare(b.capture_date ?? '');
      case 'name':
        return a.dng_path.localeCompare(b.dng_path);
      case 'stars':
        return b.stars - a.stars;
    }
  });
}

const containerStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  height: '100%',
  overflow: 'hidden',
};

const gridScrollStyles: CSSProperties = {
  flex: 1,
  overflowY: 'auto',
  padding: '24px 32px',
};

interface LazyPhotoCardProps {
  projectId: string;
  photo: Photo;
  hasThumbnail: boolean;
  cache: { current: Map<string, string> };
  onClick: () => void;
  onRevealInFolder?: () => void;
  onTrash?: () => void;
  onRestore?: () => void;
}

function LazyPhotoCard({ projectId, photo, hasThumbnail, cache, onClick, onRevealInFolder, onTrash, onRestore }: LazyPhotoCardProps) {
  const cached = cache.current.get(photo.id) ?? null;
  const [url, setUrl] = useState<string | null>(cached);

  useEffect(() => {
    if (!hasThumbnail || cached) return;
    let cancelled = false;
    api.getThumbnail(projectId, photo.id).then((data: string | null) => {
      if (!cancelled && data) {
        cache.current.set(photo.id, data);
        setUrl(data);
      }
    });
    return () => { cancelled = true; };
  }, [projectId, photo.id, hasThumbnail, cached, cache]);

  return (
    <PhotoCard
      filename={photo.dng_path}
      thumbnailUrl={url ?? undefined}
      stars={photo.stars}
      culled={photo.culled}
      captureDate={photo.capture_date ?? undefined}
      colorLabels={parseColorLabels(photo.color_label)}
      format={photoFormat(photo.dng_path, photo.camera_jpg_path)}
      onClick={onClick}
      onRevealInFolder={onRevealInFolder}
      onTrash={onTrash}
      onRestore={onRestore}
    />
  );
}

// Layout constants — must match PhotoCard dimensions
const CARD_MIN_WIDTH = 200;
const COL_GAP = 16;
const ROW_GAP = 16;
const CARD_H = PHOTO_CARD_HEADER_HEIGHT + PHOTO_CARD_FOOTER_HEIGHT; // fixed parts (header + footer)
const SEPARATOR_H = 90;

type FlatItem =
  | { type: 'separator'; label: string }
  | { type: 'row'; photos: Photo[] }

function captureDateToDay(raw: string): string {
  // EXIF format "2024:11:07 13:59:07" → "2024-11-07"
  // ISO format  "2024-11-07T13:59:07" → "2024-11-07"
  return raw.slice(0, 10).replace(/:/g, '-');
}

function formatDayLabel(day: string, t: (key: string) => string, locale: string): string {
  if (day === 'sin-fecha') return t('projectDetail.noDate');
  try {
    const d = new Date(day + 'T00:00:00');
    if (isNaN(d.getTime())) return day;
    const dateLocale = locale === 'es' ? 'es-MX' : 'en-US';
    return d.toLocaleDateString(dateLocale, {
      weekday: 'long', year: 'numeric', month: 'long', day: 'numeric',
    });
  } catch {
    return day;
  }
}

const feedbackStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  height: '200px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

export function ProjectDetail({ projectId, projectName, deviceUuid, coverPhotoPath, initialPhotoId, onBack, onCoverPhotoChange }: ProjectDetailProps) {
  const { t, i18n } = useTranslation();

  // Al abrir el proyecto: (1) asegura que exista en el catálogo (Fase 5) y
  // (2) enlaza los archivos de _exported/ con sus fotos y publica (Fase 6).
  // Fire-and-forget: degrada a solo-local sin sesión/red.
  useEffect(() => {
    api
      .ensureProjectSynced(projectId)
      .then(() => api.syncExported(projectId))
      .catch((e) => console.debug('backup sync skipped:', e));
  }, [projectId]);

  // Candado del proyecto (spec project-lock): la vista lo toma al abrir y lo
  // suelta al cerrar. Si lo tiene otra instalación, se abre en solo lectura.
  const [access, setAccess] = useState<ProjectAccess | null>(null);
  const readOnly = access?.access === 'read_only';
  const readOnlyRef = useRef(false);
  readOnlyRef.current = readOnly;
  useEffect(() => {
    let active = true;
    api
      .openProjectSession(projectId)
      .then((a) => { if (active) setAccess(a); })
      .catch((e) => console.error('open project session failed', e));
    return () => {
      active = false;
      api.closeProjectSession(projectId).catch(() => undefined);
    };
  }, [projectId]);
  // Una escritura chocó con el candado de otro (tomó el control mientras
  // estábamos aquí): se vuelve a mirar y la vista pasa a solo lectura.
  useEffect(() => {
    const recheck = () => {
      api.openProjectSession(projectId).then(setAccess).catch(() => undefined);
    };
    window.addEventListener(api.PROJECT_LOCKED_EVENT, recheck);
    return () => window.removeEventListener(api.PROJECT_LOCKED_EVENT, recheck);
  }, [projectId]);
  const handleTakeControl = useCallback(async () => {
    await api.takeProjectControl(projectId);
    setAccess({ access: 'open' });
    api.syncExported(projectId).catch(() => undefined);
  }, [projectId]);

  // En solo lectura los ajustes de vista (filtros, panel) se quedan en memoria.
  const saveViewSettings = (settings: Partial<ProjectSettings>) => {
    if (readOnlyRef.current) return;
    api.updateProjectSettings(projectId, settings as ProjectSettings).catch(() => undefined);
  };

  const [sortBy, setSortBy] = useState<SortOption>('date');
  const [viewMode, setViewMode] = useState<'grid' | 'by-date'>('by-date');
  const [showImport, setShowImport] = useState(false);
  const [selectedPhotoId, setSelectedPhotoId] = useState<string | null>(null);
  const [showCulledOnly, setShowCulledOnly] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [minStars, setMinStars] = useState<number | null>(null);
  const [starsFilterMode, setStarsFilterMode] = useState<'exact' | 'inclusive'>('inclusive');
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const [selectedColors, setSelectedColors] = useState<Set<number>>(new Set());
  const kb = useContextKeybindings('project');
  const platform = usePlatform();
  // Revealing a file in a file manager isn't available on Android.
  const canRevealInFolder = platform !== 'android';

  const handleRevealInFolder = useCallback(async (photoId: string) => {
    try {
      await api.revealPhoto(projectId, photoId);
    } catch (err) {
      console.error('reveal photo failed', err);
      alert(t('projectDetail.revealError', { error: String(err) }));
    }
  }, [projectId, t]);

  // Load project settings from DB on mount
  useEffect(() => {
    api.getProjectSettings(projectId).then((s) => {
      setSidebarOpen(s.sidebar_open);
      setShowCulledOnly(s.show_culled);
      setMinStars(s.min_stars ?? null);
      setStarsFilterMode(s.stars_filter_mode ?? 'inclusive');
      setViewMode((s.view_mode as 'grid' | 'by-date') ?? 'by-date');
      setSelectedTags(s.selected_tags ? new Set(s.selected_tags.split(',')) : new Set());
      setSelectedColors(s.selected_colors ? new Set(s.selected_colors.split(',').map(Number)) : new Set());
    });
  }, [projectId]);

  const handleSidebarToggle = useCallback((open: boolean) => {
    setSidebarOpen(open);
    saveViewSettings({ sidebar_open: open, show_culled: showCulledOnly });
  }, [projectId, showCulledOnly]);

  const handleShowCulledChange = useCallback((value: boolean) => {
    setShowCulledOnly(value);
    saveViewSettings({
      sidebar_open: sidebarOpen,
      show_culled: value,
      min_stars: minStars,
      selected_tags: Array.from(selectedTags).join(','),
      selected_colors: Array.from(selectedColors).join(','),
    });
  }, [projectId, sidebarOpen, minStars, selectedTags, selectedColors]);

  const handleMinStarsChange = useCallback((stars: number | null) => {
    setMinStars(stars);
    saveViewSettings({
      sidebar_open: sidebarOpen,
      show_culled: showCulledOnly,
      min_stars: stars,
      selected_tags: Array.from(selectedTags).join(','),
      selected_colors: Array.from(selectedColors).join(','),
      stars_filter_mode: starsFilterMode,
    });
  }, [projectId, sidebarOpen, showCulledOnly, selectedTags, selectedColors, starsFilterMode]);

  const handleStarsFilterModeChange = useCallback((mode: 'exact' | 'inclusive') => {
    setStarsFilterMode(mode);
    saveViewSettings({
      sidebar_open: sidebarOpen,
      show_culled: showCulledOnly,
      min_stars: minStars,
      selected_tags: Array.from(selectedTags).join(','),
      selected_colors: Array.from(selectedColors).join(','),
      stars_filter_mode: mode,
      view_mode: viewMode,
    });
  }, [projectId, sidebarOpen, showCulledOnly, minStars, selectedTags, selectedColors, viewMode]);

  const handleViewModeChange = useCallback((mode: 'grid' | 'by-date') => {
    setViewMode(mode);
    saveViewSettings({
      sidebar_open: sidebarOpen,
      show_culled: showCulledOnly,
      min_stars: minStars,
      selected_tags: Array.from(selectedTags).join(','),
      selected_colors: Array.from(selectedColors).join(','),
      stars_filter_mode: starsFilterMode,
      view_mode: mode,
    });
  }, [projectId, sidebarOpen, showCulledOnly, minStars, selectedTags, selectedColors, starsFilterMode]);

  const handleSelectedTagsChange = useCallback((tags: Set<string>) => {
    setSelectedTags(tags);
    saveViewSettings({
      sidebar_open: sidebarOpen,
      show_culled: showCulledOnly,
      min_stars: minStars,
      selected_tags: Array.from(tags).join(','),
      selected_colors: Array.from(selectedColors).join(','),
    });
  }, [projectId, sidebarOpen, showCulledOnly, minStars, selectedColors]);

  const handleSelectedColorsChange = useCallback((colors: Set<number>) => {
    setSelectedColors(colors);
    saveViewSettings({
      sidebar_open: sidebarOpen,
      show_culled: showCulledOnly,
      min_stars: minStars,
      selected_tags: Array.from(selectedTags).join(','),
      selected_colors: Array.from(colors).join(','),
    });
  }, [projectId, sidebarOpen, showCulledOnly, minStars, selectedTags]);

  // Grid-level keyboard shortcuts (only active when grid is visible)
  useEffect(() => {
    if (selectedPhotoId !== null || showImport) return;
    function onKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (matchesKey(e, kb['project.show_culled'])) {
        e.preventDefault();
        setShowCulledOnly((v) => {
          const next = !v;
          saveViewSettings({ sidebar_open: sidebarOpen, show_culled: next });
          return next;
        });
      }
      if (matchesKey(e, kb['project.back'])) {
        e.preventDefault();
        onBack?.();
      }
      if (matchesKey(e, kb['project.import'])) {
        e.preventDefault();
        if (!readOnlyRef.current) setShowImport(true);
      }
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [selectedPhotoId, showImport, kb, onBack, projectId, sidebarOpen]);

  const photosState = useProjectPhotos(projectId);
  const { data: photos, refetch: refetchPhotos } = photosState;
  // Fotos en la papelera (borrado lógico): fuente del grid con el filtro "Papelera".
  // El filtro no se guarda en project_settings: al abrir el proyecto empieza apagado.
  const trashState = useProjectTrashedPhotos(projectId);
  const { data: trashedPhotos, refetch: refetchTrash } = trashState;
  const [showTrash, setShowTrash] = useState(false);
  const { loading, error } = showTrash ? trashState : photosState;
  const { thumbnailIds, refetch: refetchThumbnails } = useProjectThumbnails(projectId);
  const thumbnailCache = useRef<Map<string, string>>(new Map());
  const autoRedirectedToImport = useRef(false);

  // Respaldo del proyecto (Fase 7): el modal que lista _exported/ y sube lo
  // faltante se monta a nivel de app (BackupModalHost), para abrirlo también
  // desde la barra lateral.
  const { openBackupDetail } = useBackup();

  // Auto-show import when project has no photos (only once per mount)
  useEffect(() => {
    if (access?.access === 'open' && !photosState.loading && !photosState.error && photos && photos.length === 0 && !autoRedirectedToImport.current) {
      autoRedirectedToImport.current = true;
      setShowImport(true);
    }
  }, [access, photosState.loading, photosState.error, photos]);

  // Optimistic overrides: merged on top of the DB-loaded photos array so that
  // edits made in PhotoDetailView are reflected immediately when navigating back.
  const [photoOverrides, setPhotoOverrides] = useState<Record<string, Partial<Photo>>>({});

  const handlePhotoChanged = (photoId: string, updates: Partial<Photo>) => {
    setPhotoOverrides((prev) => ({
      ...prev,
      [photoId]: { ...(prev[photoId] ?? {}), ...updates },
    }));
  };

  const applyOverrides = useCallback((list: Photo[] | null) => {
    if (!list) return list;
    if (Object.keys(photoOverrides).length === 0) return list;
    return list.map((p) =>
      photoOverrides[p.id] ? { ...p, ...photoOverrides[p.id] } : p,
    );
  }, [photoOverrides]);

  // Una foto movida a la papelera o restaurada en esta sesión lleva `deleted` en
  // sus overrides; se filtra al instante sin recargar (y sin perder el scroll).
  const activePhotos = useMemo(
    () => applyOverrides(photos)?.filter((p) => !p.deleted) ?? null,
    [photos, applyOverrides],
  );
  const trashWithOverrides = useMemo(
    () => applyOverrides(trashedPhotos)?.filter((p) => p.deleted) ?? null,
    [trashedPhotos, applyOverrides],
  );
  const photosWithOverrides = showTrash ? trashWithOverrides : activePhotos;

  // ── Papelera / restaurar ────────────────────────────────────────────────────
  const [pendingTrash, setPendingTrash] = useState<Photo | null>(null);
  const [trashing, setTrashing] = useState(false);
  const [trashError, setTrashError] = useState<string | null>(null);

  const handleConfirmTrash = async () => {
    if (!pendingTrash) return;
    const photoId = pendingTrash.id;
    setTrashing(true);
    setTrashError(null);
    try {
      await api.trashPhoto(projectId, photoId);
      handlePhotoChanged(photoId, { deleted: true });
      if (selectedPhotoId === photoId) setSelectedPhotoId(null);
      if (coverPhotoPath === `.thumbs/${photoId}.jpg`) onCoverPhotoChange?.(null);
      setPendingTrash(null);
      refetchTrash();
    } catch (err) {
      console.error('trash photo failed', err);
      setTrashError(t('projectDetail.trashPhoto.error', { error: String(err) }));
    } finally {
      setTrashing(false);
    }
  };

  const handleRestore = async (photoId: string) => {
    try {
      await api.restorePhoto(projectId, photoId);
      handlePhotoChanged(photoId, { deleted: false });
      refetchPhotos();
    } catch (err) {
      console.error('restore photo failed', err);
      alert(t('projectDetail.restoreError', { error: String(err) }));
    }
  };

  const prevShowImport = useRef(false);
  useEffect(() => {
    if (prevShowImport.current && !showImport) {
      refetchPhotos();
      refetchThumbnails();
    }
    prevShowImport.current = showImport;
  }, [showImport, refetchPhotos, refetchThumbnails]);

  const sortedPhotos = useMemo(() => {
    if (!photosWithOverrides) return [];
    let filtered = showCulledOnly
      ? photosWithOverrides.filter((p) => p.culled)
      : photosWithOverrides;

    if (minStars !== null) {
      if (starsFilterMode === 'exact') {
        filtered = filtered.filter((p) => p.stars === minStars);
      } else {
        filtered = filtered.filter((p) => p.stars >= minStars);
      }
    }

    if (selectedTags.size > 0) {
      filtered = filtered.filter((p) => {
        if (!p.tags) return false;
        const photoTags = p.tags.split(',').map((t) => t.trim());
        return Array.from(selectedTags).some((t) => photoTags.includes(t));
      });
    }

    if (selectedColors.size > 0) {
      filtered = filtered.filter((p) => {
        if (!p.color_label) return false;
        const photoColors = p.color_label.split(',').map((c) => parseInt(c.trim(), 10));
        return Array.from(selectedColors).some((c) => photoColors.includes(c));
      });
    }

    return sortPhotos(filtered, sortBy);
  }, [photosWithOverrides, sortBy, showCulledOnly, minStars, starsFilterMode, selectedTags, selectedColors]);

  const allAvailableTags = useMemo(() => {
    const tags = new Set<string>();
    if (photosWithOverrides) {
      for (const photo of photosWithOverrides) {
        if (photo.tags) {
          const photoTags = photo.tags.split(',').map((t) => t.trim());
          photoTags.forEach((t) => tags.add(t));
        }
      }
    }
    return tags;
  }, [photosWithOverrides]);

  // Al venir de la búsqueda se abre esa foto. Si los filtros guardados del
  // proyecto la esconden, se quitan (solo en memoria) para que se vea.
  const openedInitialPhoto = useRef(false);
  useEffect(() => {
    if (!initialPhotoId || openedInitialPhoto.current || !activePhotos) return;
    if (!activePhotos.some((p) => p.id === initialPhotoId)) return;
    openedInitialPhoto.current = true;
    if (!sortedPhotos.some((p) => p.id === initialPhotoId)) {
      setShowCulledOnly(false);
      setMinStars(null);
      setSelectedTags(new Set());
      setSelectedColors(new Set());
    }
    setSelectedPhotoId(initialPhotoId);
  }, [initialPhotoId, activePhotos, sortedPhotos]);

  const selectedPhotoIndex = useMemo(
    () => (selectedPhotoId ? sortedPhotos.findIndex((p) => p.id === selectedPhotoId) : -1),
    [selectedPhotoId, sortedPhotos],
  );

  const culledCount = useMemo(
    () => sortedPhotos.filter((p) => p.culled).length,
    [sortedPhotos],
  );

  // ── Virtual grid ────────────────────────────────────────────────────────────
  // Callback ref so the ResizeObserver re-attaches whenever the scroll div
  // mounts/unmounts (e.g. when navigating to/from PhotoDetailView).
  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null);
  const [containerWidth, setContainerWidth] = useState(0);

  useEffect(() => {
    if (!scrollEl) return;
    const ro = new ResizeObserver(([entry]) => {
      setContainerWidth(entry.contentRect.width);
    });
    ro.observe(scrollEl);
    return () => ro.disconnect();
  }, [scrollEl]);

  // Mirror the original auto-fill minmax(200px, 1fr) behaviour
  const colCount = containerWidth > 0
    ? Math.max(1, Math.floor((containerWidth + COL_GAP) / (CARD_MIN_WIDTH + COL_GAP)))
    : 4;

  const cardWidth = containerWidth > 0
    ? (containerWidth - (colCount - 1) * COL_GAP) / colCount
    : CARD_MIN_WIDTH;

  // 4:3 image + footer + row gap below each row
  const rowHeight = cardWidth * 0.75 + CARD_H + ROW_GAP;

  // Group sorted photos by day and build the flat virtualizer list
  const flatList = useMemo((): FlatItem[] => {
    if (sortedPhotos.length === 0 || colCount === 0) return [];

    // Grid view: just rows without separators
    if (viewMode === 'grid') {
      const items: FlatItem[] = [];
      for (let i = 0; i < sortedPhotos.length; i += colCount) {
        items.push({ type: 'row', photos: sortedPhotos.slice(i, i + colCount) });
      }
      return items;
    }

    // By-date view: group by day with separators
    const groups = new Map<string, Photo[]>();
    for (const photo of sortedPhotos) {
      const day = photo.capture_date ? captureDateToDay(photo.capture_date) : 'sin-fecha';
      const bucket = groups.get(day);
      if (bucket) bucket.push(photo);
      else groups.set(day, [photo]);
    }

    const sortedKeys = [...groups.keys()].sort((a, b) => {
      if (a === 'sin-fecha') return 1;
      if (b === 'sin-fecha') return -1;
      return a.localeCompare(b); // oldest day first
    });

    const items: FlatItem[] = [];
    for (const key of sortedKeys) {
      items.push({ type: 'separator', label: formatDayLabel(key, t, i18n.language) });
      const dayPhotos = groups.get(key)!;
      for (let i = 0; i < dayPhotos.length; i += colCount) {
        items.push({ type: 'row', photos: dayPhotos.slice(i, i + colCount) });
      }
    }
    return items;
  }, [sortedPhotos, colCount, viewMode, t, i18n.language]);

  const rowVirtualizer = useVirtualizer({
    count: flatList.length,
    getScrollElement: () => scrollEl,
    estimateSize: (i) => flatList[i]?.type === 'separator' ? SEPARATOR_H : rowHeight,
    overscan: 3,
  });

  useEffect(() => {
    rowVirtualizer.measure();
  }, [rowHeight, flatList.length]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Early exits ─────────────────────────────────────────────────────────────

  if (showImport) {
    return (
      <ImportPage
        initialProjectId={projectId}
        initialProjectName={projectName}
        initialDeviceId={deviceUuid}
        onClose={() => setShowImport(false)}
      />
    );
  }

  if (selectedPhotoId !== null && selectedPhotoIndex !== -1) {
    return (
      <PhotoDetail
        photo={sortedPhotos[selectedPhotoIndex]}
        photos={sortedPhotos}
        currentIndex={selectedPhotoIndex}
        projectName={projectName}
        coverPhotoPath={coverPhotoPath}
        sidebarOpen={sidebarOpen}
        onSidebarToggle={handleSidebarToggle}
        onClose={() => setSelectedPhotoId(null)}
        onNavigate={(idx) => setSelectedPhotoId(sortedPhotos[idx].id)}
        onThumbnailChanged={(photoIds) => {
            for (const id of photoIds) thumbnailCache.current.delete(id);
            refetchThumbnails();
          }}
        onPhotoChanged={handlePhotoChanged}
        onCoverPhotoChange={onCoverPhotoChange}
        readOnly={readOnly}
      />
    );
  }

  return (
    <div style={containerStyles}>
      <ProjectDetailHeader
        projectName={projectName}
        onBack={onBack}
        viewMode={viewMode}
        onViewModeChange={handleViewModeChange}
        onImport={readOnly ? undefined : () => setShowImport(true)}
        onBackup={readOnly ? undefined : () => openBackupDetail(projectId, projectName)}
      />

      {access?.access === 'read_only' && (
        <ProjectLockBanner holder={access.holder} onTakeControl={handleTakeControl} />
      )}

      <div ref={setScrollEl} style={gridScrollStyles}>
        {loading && <div style={feedbackStyles}>{t('projectDetail.loading')}</div>}

        {!loading && error && (
          <div style={feedbackStyles}>{t('projectDetail.loadError', { error })}</div>
        )}

        {!loading && !error && sortedPhotos.length === 0 && (
          <div style={feedbackStyles}>{t('projectDetail.empty')}</div>
        )}

        {!loading && !error && flatList.length > 0 && (
          <div style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}>
            {rowVirtualizer.getVirtualItems().map((vRow) => {
              const item = flatList[vRow.index];
              if (!item) return null;

              const baseStyle: CSSProperties = {
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                transform: `translateY(${vRow.start}px)`,
              };

              if (item.type === 'separator') {
                return (
                  <div key={vRow.key} style={{ ...baseStyle, height: SEPARATOR_H, display: 'flex', flexDirection: 'column' }}>
                    <div style={{
                      flex: 1,
                      display: 'flex',
                      alignItems: 'flex-end',
                      paddingBottom: '20px',
                      borderBottom: '1px solid rgba(255, 255, 255, 0.2)',
                      fontFamily: 'var(--lumik-font-primary, Inter)',
                      fontSize: '18px',
                      fontWeight: 600,
                      color: 'var(--lumik-on-surface-variant, #c2c6d7)',
                      textTransform: 'capitalize',
                    }}>
                      {item.label}
                    </div>
                    <div style={{ height: '40px', flexShrink: 0 }} />
                  </div>
                );
              }

              return (
                <div key={vRow.key} style={{
                  ...baseStyle,
                  display: 'grid',
                  gridTemplateColumns: `repeat(${colCount}, 1fr)`,
                  gap: `${COL_GAP}px`,
                  paddingBottom: `${ROW_GAP}px`,
                }}>
                  {Array.from({ length: colCount }, (_, col) => {
                    const photo = item.photos[col];
                    if (!photo) return <div key={col} />;
                    return (
                      <LazyPhotoCard
                        key={photo.id}
                        projectId={projectId}
                        photo={photo}
                        hasThumbnail={thumbnailIds.has(photo.id)}
                        cache={thumbnailCache}
                        onClick={() => setSelectedPhotoId(photo.id)}
                        onRevealInFolder={canRevealInFolder ? () => handleRevealInFolder(photo.id) : undefined}
                        onTrash={showTrash || readOnly ? undefined : () => { setTrashError(null); setPendingTrash(photo); }}
                        onRestore={showTrash && !readOnly ? () => handleRestore(photo.id) : undefined}
                      />
                    );
                  })}
                </div>
              );
            })}
          </div>
        )}
      </div>

      <ProjectDetailFooter
        totalPhotos={activePhotos?.length ?? 0}
        culledCount={culledCount}
        showCulledOnly={showCulledOnly}
        onShowCulledOnlyChange={handleShowCulledChange}
        minStars={minStars}
        onMinStarsChange={handleMinStarsChange}
        starsFilterMode={starsFilterMode}
        onStarsFilterModeChange={handleStarsFilterModeChange}
        allTags={allAvailableTags}
        selectedTags={selectedTags}
        onSelectedTagsChange={handleSelectedTagsChange}
        selectedColors={selectedColors}
        onSelectedColorsChange={handleSelectedColorsChange}
        trashCount={trashWithOverrides?.length ?? 0}
        showTrash={showTrash}
        onShowTrashChange={setShowTrash}
      />

      <TrashPhotoModal
        open={pendingTrash !== null}
        fileName={pendingTrash ? fileNameOf(pendingTrash.dng_path) : ''}
        onClose={() => setPendingTrash(null)}
        onConfirm={handleConfirmTrash}
        loading={trashing}
        error={trashError}
      />
    </div>
  );
}
