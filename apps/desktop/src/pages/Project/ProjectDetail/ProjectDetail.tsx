import { useState, useMemo, useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { CSSProperties } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { PhotoCard, PHOTO_CARD_HEADER_HEIGHT, PHOTO_CARD_FOOTER_HEIGHT, type ColorLabel } from '@lumik/ui';
import type { Photo } from '../../../lib/types';
import { useProjectPhotos, useProjectThumbnails, useContextKeybindings, matchesKey } from '../../../lib/hooks';
import * as api from '../../../lib/api';
import { ProjectDetailHeader, type SortOption } from './ProjectDetailHeader';
import { ProjectDetailFooter } from './ProjectDetailFooter';
import { ImportPage } from '../../ImportPage';
import { PhotoDetail } from '../PhotoDetail';

export interface ProjectDetailProps {
  projectId: string;
  projectName: string;
  deviceUuid: string;
  coverPhotoPath: string | null;
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
}

function LazyPhotoCard({ projectId, photo, hasThumbnail, cache, onClick }: LazyPhotoCardProps) {
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
      onClick={onClick}
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

export function ProjectDetail({ projectId, projectName, deviceUuid, coverPhotoPath, onBack, onCoverPhotoChange }: ProjectDetailProps) {
  const { t, i18n } = useTranslation();
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
    api.updateProjectSettings(projectId, { sidebar_open: open, show_culled: showCulledOnly });
  }, [projectId, showCulledOnly]);

  const handleShowCulledChange = useCallback((value: boolean) => {
    setShowCulledOnly(value);
    api.updateProjectSettings(projectId, {
      sidebar_open: sidebarOpen,
      show_culled: value,
      min_stars: minStars,
      selected_tags: Array.from(selectedTags).join(','),
      selected_colors: Array.from(selectedColors).join(','),
    });
  }, [projectId, sidebarOpen, minStars, selectedTags, selectedColors]);

  const handleMinStarsChange = useCallback((stars: number | null) => {
    setMinStars(stars);
    api.updateProjectSettings(projectId, {
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
    api.updateProjectSettings(projectId, {
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
    api.updateProjectSettings(projectId, {
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
    api.updateProjectSettings(projectId, {
      sidebar_open: sidebarOpen,
      show_culled: showCulledOnly,
      min_stars: minStars,
      selected_tags: Array.from(tags).join(','),
      selected_colors: Array.from(selectedColors).join(','),
    });
  }, [projectId, sidebarOpen, showCulledOnly, minStars, selectedColors]);

  const handleSelectedColorsChange = useCallback((colors: Set<number>) => {
    setSelectedColors(colors);
    api.updateProjectSettings(projectId, {
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
          api.updateProjectSettings(projectId, { sidebar_open: sidebarOpen, show_culled: next });
          return next;
        });
      }
      if (matchesKey(e, kb['project.back'])) {
        e.preventDefault();
        onBack?.();
      }
      if (matchesKey(e, kb['project.import'])) {
        e.preventDefault();
        setShowImport(true);
      }
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [selectedPhotoId, showImport, kb, onBack, projectId, sidebarOpen]);

  const { data: photos, loading, error, refetch: refetchPhotos } = useProjectPhotos(projectId);
  const { thumbnailIds, refetch: refetchThumbnails } = useProjectThumbnails(projectId);
  const thumbnailCache = useRef<Map<string, string>>(new Map());
  const autoRedirectedToImport = useRef(false);

  // Auto-show import when project has no photos (only once per mount)
  useEffect(() => {
    if (!loading && !error && photos && photos.length === 0 && !autoRedirectedToImport.current) {
      autoRedirectedToImport.current = true;
      setShowImport(true);
    }
  }, [loading, error, photos]);

  // Optimistic overrides: merged on top of the DB-loaded photos array so that
  // edits made in PhotoDetailView are reflected immediately when navigating back.
  const [photoOverrides, setPhotoOverrides] = useState<Record<string, Partial<Photo>>>({});

  const handlePhotoChanged = (photoId: string, updates: Partial<Photo>) => {
    setPhotoOverrides((prev) => ({
      ...prev,
      [photoId]: { ...(prev[photoId] ?? {}), ...updates },
    }));
  };

  const photosWithOverrides = useMemo(() => {
    if (!photos) return photos;
    if (Object.keys(photoOverrides).length === 0) return photos;
    return photos.map((p) =>
      photoOverrides[p.id] ? { ...p, ...photoOverrides[p.id] } : p,
    );
  }, [photos, photoOverrides]);

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
        onImport={() => setShowImport(true)}
      />

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
        totalPhotos={photos?.length ?? 0}
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
      />
    </div>
  );
}
