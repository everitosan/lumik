import { useState, useEffect, useRef, useCallback, type CSSProperties } from 'react';
import { Icon } from '@lumik/ui';
import type { ColorLabel } from '@lumik/ui';
import type { Photo } from '../../../lib/types';
import { usePhotoPreview, useContextKeybindings, matchesKey } from '../../../lib/hooks';
import * as api from '../../../lib/api';
import { PhotoViewer, type PhotoViewerHandle, type HistogramBins } from './PhotoViewer';
import { PhotoSidebar } from './PhotoSidebar';

export interface PhotoDetailProps {
  photo: Photo;
  photos: Photo[];
  currentIndex: number;
  projectName: string;
  coverPhotoPath: string | null;
  onClose: () => void;
  onNavigate: (index: number) => void;
  onThumbnailChanged?: () => void;
  onPhotoChanged?: (photoId: string, updates: Partial<Pick<Photo, 'stars' | 'color_label' | 'tags' | 'culled'>>) => void;
  onCoverPhotoChange?: (photoId: string | null) => void;
}

type SaveState = 'idle' | 'saving' | 'saved' | 'error';

function parseColorLabels(raw: string | null): ColorLabel[] {
  if (!raw) return [];
  return raw
    .split(',')
    .map((n) => parseInt(n.trim(), 10))
    .filter((n) => n >= 1 && n <= 5) as ColorLabel[];
}

function parseTags(raw: string | null): string[] {
  if (!raw) return [];
  return raw.split(',').map((t) => t.trim()).filter(Boolean);
}

function basename(path: string): string {
  return path.split('/').pop() ?? path;
}

const rootStyle: CSSProperties = {
  width: '100%',
  height: '100%',
  display: 'flex',
  flexDirection: 'column',
  background: 'var(--lumik-surface-container-lowest, #0e0e0e)',
};

const headerStyle: CSSProperties = {
  height: '52px',
  flexShrink: 0,
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  padding: '0 20px',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
  background: 'var(--lumik-surface-container-low, #1c1b1b)',
};

const backBtnStyle: CSSProperties = {
  display: 'flex', alignItems: 'center', gap: '4px',
  padding: '4px 10px',
  background: 'transparent',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius, 4px)',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  cursor: 'pointer',
  flexShrink: 0,
};

export function PhotoDetail({
  photo,
  photos,
  currentIndex,
  projectName,
  coverPhotoPath,
  onClose,
  onNavigate,
  onThumbnailChanged,
  onPhotoChanged,
  onCoverPhotoChange,
}: PhotoDetailProps) {
  const kb = useContextKeybindings('photo_detail');
  const viewerRef = useRef<PhotoViewerHandle>(null);
  const [saveState, setSaveState] = useState<SaveState>('idle');
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const thumbnailDirtyRef = useRef(false);
  const onThumbnailChangedRef = useRef(onThumbnailChanged);
  onThumbnailChangedRef.current = onThumbnailChanged;

  const [histogramBins, setHistogramBins] = useState<HistogramBins | null>(null);

  const [localStars, setLocalStars] = useState(photo.stars);
  const [localColorLabels, setLocalColorLabels] = useState<ColorLabel[]>(() => parseColorLabels(photo.color_label));
  const [localTags, setLocalTags] = useState<string[]>(() => parseTags(photo.tags));
  const [localCulled, setLocalCulled] = useState(photo.culled);

  // Reset editable state and histogram when navigating to a different photo
  useEffect(() => {
    setLocalStars(photo.stars);
    setLocalColorLabels(parseColorLabels(photo.color_label));
    setLocalTags(parseTags(photo.tags));
    setLocalCulled(photo.culled);
    setHistogramBins(null);
  }, [photo.id]); // eslint-disable-line react-hooks/exhaustive-deps

  // Flush thumbnail refetch when leaving detail view
  useEffect(() => {
    return () => {
      if (thumbnailDirtyRef.current) onThumbnailChangedRef.current?.();
    };
  }, []);

  const hasPrev = currentIndex > 0;
  const hasNext = currentIndex < photos.length - 1;
  const filename = basename(photo.dng_path);

  const { data: preview, loading: previewLoading } = usePhotoPreview(photo.id, photo.project_id);
  const fullImageUrl = preview?.url ?? null;
  const initialRotation = preview?.rotation ?? 0;

  const goPrev = useCallback(() => {
    if (hasPrev) onNavigate(currentIndex - 1);
  }, [hasPrev, currentIndex, onNavigate]);

  const goNext = useCallback(() => {
    if (hasNext) onNavigate(currentIndex + 1);
  }, [hasNext, currentIndex, onNavigate]);

  const handleRotationChange = useCallback(
    (rotation: number) => {
      setSaveState('saving');
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      api.savePhotoRotation(photo.id, photo.project_id, rotation)
        .then(() => {
          setSaveState('saved');
          saveTimerRef.current = setTimeout(() => setSaveState('idle'), 2000);
          thumbnailDirtyRef.current = true;
        })
        .catch(() => setSaveState('error'));
    },
    [photo.id, photo.project_id],
  );

  const saveRating = (stars: number, colorLabels: ColorLabel[], tags: string[]) => {
    setSaveState('saving');
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    const color_label = colorLabels.length > 0 ? colorLabels.join(',') : null;
    const tagsStr = tags.length > 0 ? tags.join(',') : null;
    api.savePhotoRating(photo.id, photo.project_id, stars, color_label, tagsStr)
      .then(() => {
        setSaveState('saved');
        saveTimerRef.current = setTimeout(() => setSaveState('idle'), 2000);
        onPhotoChanged?.(photo.id, { stars, color_label, tags: tagsStr });
      })
      .catch(() => setSaveState('error'));
  };

  const handleStarsChange = (stars: number) => {
    setLocalStars(stars);
    saveRating(stars, localColorLabels, localTags);
  };

  const handleColorChange = (colorLabels: ColorLabel[]) => {
    setLocalColorLabels(colorLabels);
    saveRating(localStars, colorLabels, localTags);
  };

  const handleTagsChange = (tags: string[]) => {
    setLocalTags(tags);
    saveRating(localStars, localColorLabels, tags);
  };

  const isCover = coverPhotoPath === `.thumbs/${photo.id}.jpg`;

  const handleCoverToggle = useCallback(() => {
    const newCoverId = isCover ? null : photo.id;
    api.setProjectCoverPhoto(photo.project_id, newCoverId)
      .then(() => onCoverPhotoChange?.(newCoverId))
      .catch(() => setSaveState('error'));
  }, [isCover, photo.id, photo.project_id, onCoverPhotoChange]);

  const handleCulledChange = useCallback((culled: boolean) => {
    setLocalCulled(culled);
    setSaveState('saving');
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    api.savePhotoCulled(photo.id, photo.project_id, culled)
      .then(() => {
        setSaveState('saved');
        saveTimerRef.current = setTimeout(() => setSaveState('idle'), 2000);
        onPhotoChanged?.(photo.id, { culled });
      })
      .catch(() => {
        setLocalCulled(!culled);
        setSaveState('error');
      });
  }, [photo.id, photo.project_id, onPhotoChanged]);

  // Global keyboard shortcuts
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (matchesKey(e, kb['photo_detail.close']))         { onClose(); return; }
      if (matchesKey(e, kb['photo_detail.prev']))          { goPrev(); return; }
      if (matchesKey(e, kb['photo_detail.next']))          { goNext(); return; }
      if (matchesKey(e, kb['photo_detail.zoom_in']) || (kb['photo_detail.zoom_in'] === '+' && e.key === '=' && !e.ctrlKey)) { viewerRef.current?.zoomIn(); return; }
      if (matchesKey(e, kb['photo_detail.zoom_out']))      { viewerRef.current?.zoomOut(); return; }
      if (matchesKey(e, kb['photo_detail.fit']))           { viewerRef.current?.fitToScreen(); return; }
      if (matchesKey(e, kb['photo_detail.rotate_left']))   { viewerRef.current?.rotateLeft(); return; }
      if (matchesKey(e, kb['photo_detail.rotate_right']))  { viewerRef.current?.rotateRight(); return; }
      if (matchesKey(e, kb['photo_detail.cull']))          { handleCulledChange(!localCulled); return; }
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [kb, onClose, goPrev, goNext, localCulled, handleCulledChange]);

  return (
    <div style={rootStyle}>
      {/* Header */}
      <div style={headerStyle}>
        <button onClick={onClose} style={backBtnStyle}>
          <Icon name="chevron-left" size={16} />
          Volver
        </button>
        <div style={{ flex: 1, minWidth: 0, display: 'flex', alignItems: 'center', gap: '4px', overflow: 'hidden' }}>
          <span style={{ fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '13px', color: 'var(--lumik-outline, #8c90a0)', whiteSpace: 'nowrap', flexShrink: 0 }}>
            {projectName}
          </span>
          <span style={{ color: 'var(--lumik-outline-variant, #424654)', flexShrink: 0 }}>
            <Icon name="chevron-right" size={12} />
          </span>
          <span
            style={{ fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)', fontSize: '13px', fontWeight: 500, color: 'var(--lumik-on-surface, #e5e2e1)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
            title={filename}
          >
            {filename}
          </span>
        </div>
        <button
          onClick={handleCoverToggle}
          title={isCover ? 'Quitar como portada del proyecto' : 'Usar como portada del proyecto'}
          style={{
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            width: '32px', height: '32px', padding: 0, flexShrink: 0,
            background: 'transparent', border: 'none',
            borderRadius: 'var(--lumik-radius, 4px)', cursor: 'pointer',
            color: isCover ? 'var(--lumik-secondary, #e9c349)' : 'var(--lumik-outline, #8c90a0)',
          }}
        >
          <Icon name={isCover ? 'star-filled' : 'star'} size={18} />
        </button>
        <span style={{ fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)', fontSize: '12px', color: 'var(--lumik-outline, #8c90a0)', flexShrink: 0 }}>
          {currentIndex + 1} / {photos.length}
        </span>
        {saveState !== 'idle' && (
          <span style={{
            fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
            fontSize: '11px',
            color: saveState === 'error' ? 'var(--lumik-error, #ffb4ab)' : saveState === 'saved' ? '#27AE60' : 'var(--lumik-outline, #8c90a0)',
            flexShrink: 0,
          }}>
            {saveState === 'saving' && '↻ Guardando…'}
            {saveState === 'saved'  && '✓ Guardado'}
            {saveState === 'error'  && '✕ Error al guardar'}
          </span>
        )}
      </div>

      {/* Body */}
      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        <PhotoViewer
          ref={viewerRef}
          photoId={photo.id}
          fullImageUrl={fullImageUrl}
          fullImageLoading={previewLoading}
          initialRotation={initialRotation}
          hasPrev={hasPrev}
          hasNext={hasNext}
          onPrev={goPrev}
          onNext={goNext}
          onRotationChange={handleRotationChange}
          onHistogramReady={setHistogramBins}
        />
        <PhotoSidebar
          photo={photo}
          histogramBins={histogramBins}
          localStars={localStars}
          localColorLabels={localColorLabels}
          localTags={localTags}
          localCulled={localCulled}
          onStarsChange={handleStarsChange}
          onColorChange={handleColorChange}
          onTagsChange={handleTagsChange}
          onCulledChange={handleCulledChange}
        />
      </div>
    </div>
  );
}
