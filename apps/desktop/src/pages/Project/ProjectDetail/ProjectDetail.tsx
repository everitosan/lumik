import { useState, useMemo, useEffect, useRef, useCallback } from 'react';
import type { CSSProperties } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { PhotoCard, PHOTO_CARD_HEADER_HEIGHT, PHOTO_CARD_FOOTER_HEIGHT, type ColorLabel } from '@lumik/ui';
import type { Photo } from '../../../lib/types';
import { useProjectPhotos, useProjectThumbnails, useContextKeybindings, matchesKey } from '../../../lib/hooks';
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

function formatDayLabel(day: string): string {
  if (day === 'sin-fecha') return 'Sin fecha';
  try {
    const d = new Date(day + 'T00:00:00');
    if (isNaN(d.getTime())) return day;
    return d.toLocaleDateString('es-MX', {
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
  const [sortBy, setSortBy] = useState<SortOption>('date');
  const [showImport, setShowImport] = useState(false);
  const [selectedPhotoId, setSelectedPhotoId] = useState<string | null>(null);
  const [showCulledOnly, setShowCulledOnly] = useState(false);
  const kb = useContextKeybindings('project');

  // Grid-level keyboard shortcuts (only active when grid is visible)
  useEffect(() => {
    if (selectedPhotoId !== null || showImport) return;
    function onKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (matchesKey(e, kb['project.show_culled'])) {
        e.preventDefault();
        setShowCulledOnly((v) => !v);
      }
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [selectedPhotoId, showImport, kb]);

  const { data: photos, loading, error, refetch: refetchPhotos } = useProjectPhotos(projectId);
  const { thumbnails, refetch: refetchThumbnails } = useProjectThumbnails(projectId);

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
    const filtered = showCulledOnly
      ? photosWithOverrides.filter((p) => p.culled)
      : photosWithOverrides;
    return sortPhotos(filtered, sortBy);
  }, [photosWithOverrides, sortBy, showCulledOnly]);

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
      items.push({ type: 'separator', label: formatDayLabel(key) });
      const dayPhotos = groups.get(key)!;
      for (let i = 0; i < dayPhotos.length; i += colCount) {
        items.push({ type: 'row', photos: dayPhotos.slice(i, i + colCount) });
      }
    }
    return items;
  }, [sortedPhotos, colCount]);

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
        onClose={() => setSelectedPhotoId(null)}
        onNavigate={(idx) => setSelectedPhotoId(sortedPhotos[idx].id)}
        onThumbnailChanged={refetchThumbnails}
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
        sortBy={sortBy}
        onSortChange={setSortBy}
        showCulledOnly={showCulledOnly}
        onShowCulledOnlyChange={setShowCulledOnly}
      />

      <div ref={setScrollEl} style={gridScrollStyles}>
        {loading && <div style={feedbackStyles}>Cargando fotos…</div>}

        {!loading && error && (
          <div style={feedbackStyles}>No se pudieron cargar las fotos: {error}</div>
        )}

        {!loading && !error && sortedPhotos.length === 0 && (
          <div style={feedbackStyles}>Este proyecto no tiene fotos aún.</div>
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
                      <PhotoCard
                        key={photo.id}
                        filename={photo.dng_path}
                        thumbnailUrl={thumbnails[photo.id]}
                        stars={photo.stars}
                        culled={photo.culled}
                        captureDate={photo.capture_date ?? undefined}
                        colorLabels={parseColorLabels(photo.color_label)}
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
        onImport={() => setShowImport(true)}
      />
    </div>
  );
}
