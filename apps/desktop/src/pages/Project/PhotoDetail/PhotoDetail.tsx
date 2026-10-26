import { useState, useEffect, useRef, useCallback, useMemo, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { FormatBadge, Icon, photoFormat } from '@lumik/ui';
import type { ColorLabel } from '@lumik/ui';
import type { Photo } from '../../../lib/types';
import { usePhotoPreview, useContextKeybindings, matchesKey, usePlatform } from '../../../lib/hooks';
import * as api from '../../../lib/api';
import { PhotoViewer, type PhotoViewerHandle, type HistogramBins } from './PhotoViewer';
import { PhotoSidebar, type TagManagerHandle } from './PhotoSidebar';
import { AddToAlbumDialog, type AlbumTarget } from '../../Albums/AddToAlbumDialog';

export interface PhotoDetailProps {
  photo: Photo;
  photos: Photo[];
  currentIndex: number;
  projectName: string;
  coverPhotoPath: string | null;
  sidebarOpen?: boolean;
  onSidebarToggle?: (open: boolean) => void;
  onClose: () => void;
  onNavigate: (index: number) => void;
  onThumbnailChanged?: (photoIds: string[]) => void;
  onPhotoChanged?: (photoId: string, updates: Partial<Pick<Photo, 'stars' | 'color_label' | 'tags' | 'culled' | 'rotation'>>) => void;
  onCoverPhotoChange?: (photoId: string | null) => void;
  /** Proyecto en otra instalación (spec project-lock): se ve todo, no se cambia nada. */
  readOnly?: boolean;
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
  return path.replace(/\\/g, '/').split('/').pop() ?? path;
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
  sidebarOpen = true,
  onSidebarToggle,
  onClose,
  onNavigate,
  onThumbnailChanged,
  onPhotoChanged,
  onCoverPhotoChange,
  readOnly = false,
}: PhotoDetailProps) {
  const { t } = useTranslation();
  const platform = usePlatform();
  const isMobile = platform === 'android' || platform === 'ios';
  const [infoPanelCollapsed, setInfoPanelCollapsed] = useState(!sidebarOpen);
  const infoCollapsed = isMobile && infoPanelCollapsed;

  const kb = useContextKeybindings('photo_detail');
  const viewerRef = useRef<PhotoViewerHandle>(null);
  const tagManagerRef = useRef<TagManagerHandle>(null);
  const [saveState, setSaveState] = useState<SaveState>('idle');
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const thumbnailDirtyRef = useRef(new Set<string>());
  const onThumbnailChangedRef = useRef(onThumbnailChanged);
  onThumbnailChangedRef.current = onThumbnailChanged;

  const [histogramBins, setHistogramBins] = useState<HistogramBins | null>(null);

  const [localStars, setLocalStars] = useState(photo.stars);
  const [localColorLabels, setLocalColorLabels] = useState<ColorLabel[]>(() => parseColorLabels(photo.color_label));
  const [localTags, setLocalTags] = useState<string[]>(() => parseTags(photo.tags));
  const [localCulled, setLocalCulled] = useState(photo.culled);
  const [albumTarget, setAlbumTarget] = useState<AlbumTarget | null>(null);
  const openAddToAlbum = useCallback(
    () =>
      setAlbumTarget({
        items: [{ kind: 'photo', project_id: photo.project_id, ref_id: photo.id, file_name: basename(photo.dng_path), project_name: projectName }],
      }),
    [photo.project_id, photo.id, photo.dng_path, projectName],
  );

  // Sync panel state when the project preference is loaded from DB
  useEffect(() => {
    setInfoPanelCollapsed(!sidebarOpen);
  }, [sidebarOpen]); // eslint-disable-line react-hooks/exhaustive-deps

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
      if (thumbnailDirtyRef.current.size > 0) onThumbnailChangedRef.current?.([...thumbnailDirtyRef.current]);
    };
  }, []);

  const hasPrev = currentIndex > 0;
  const hasNext = currentIndex < photos.length - 1;
  const filename = basename(photo.dng_path);

  // All tags already used across the project, normalized to lowercase + deduped,
  // sorted alphabetically. Feeds the tag autocomplete to avoid near-duplicates.
  const tagSuggestions = useMemo(() => {
    const set = new Set<string>();
    for (const p of photos) {
      for (const tag of parseTags(p.tags)) set.add(tag.toLowerCase());
    }
    return [...set].sort();
  }, [photos]);

  // Un par RAW+JPG se abre en su JPG; el indicador cambia al RAW y, al pasar a
  // otra foto, se vuelve al JPG.
  const format = photoFormat(photo.dng_path, photo.camera_jpg_path);
  const [showRaw, setShowRaw] = useState(false);
  useEffect(() => setShowRaw(false), [photo.id]);
  const { data: preview, loading: previewLoading } = usePhotoPreview(
    photo.id,
    photo.project_id,
    format === 'RAW+JPG' && showRaw,
  );
  const fullImageUrl = preview?.url ?? null;
  const initialRotation = photo.rotation;

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
          thumbnailDirtyRef.current.add(photo.id);
          onPhotoChanged?.(photo.id, { rotation });
        })
        .catch(() => setSaveState('error'));
    },
    [photo.id, photo.project_id, onPhotoChanged],
  );

  const saveRating = (stars: number, colorLabels: ColorLabel[], tags: string[]) => {
    if (readOnly) return;
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
    if (readOnly) return;
    setLocalStars(stars);
    saveRating(stars, localColorLabels, localTags);
  };

  const handleColorChange = (colorLabels: ColorLabel[]) => {
    if (readOnly) return;
    setLocalColorLabels(colorLabels);
    saveRating(localStars, colorLabels, localTags);
  };

  const handleTagsChange = (tags: string[]) => {
    if (readOnly) return;
    setLocalTags(tags);
    saveRating(localStars, localColorLabels, tags);
  };

  const isCover = coverPhotoPath === `.thumbs/${photo.id}.jpg`;

  const handleCoverToggle = useCallback(() => {
    if (readOnly) return;
    const newCoverId = isCover ? null : photo.id;
    api.setProjectCoverPhoto(photo.project_id, newCoverId)
      .then(() => {
        onCoverPhotoChange?.(newCoverId);
        // Refleja la nueva portada en el catálogo/dashboard web (fire-and-forget).
        api.syncProjectCover(photo.project_id).catch(() => undefined);
      })
      .catch(() => setSaveState('error'));
  }, [readOnly, isCover, photo.id, photo.project_id, onCoverPhotoChange]);

  const handleCulledChange = useCallback((culled: boolean) => {
    // Una foto en la papelera no se aprueba (el backend también lo rechaza): sin
    // actualización optimista, para no mostrar un cambio que no ocurrirá.
    if (photo.deleted || readOnly) return;
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
  }, [photo.id, photo.project_id, photo.deleted, readOnly, onPhotoChanged]);

  // Global keyboard shortcuts
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      // Con el diálogo de álbum abierto, las teclas son suyas (Esc lo cierra).
      if (albumTarget) return;
      if (matchesKey(e, kb['photo_detail.close']))         { onClose(); return; }
      if (matchesKey(e, kb['photo_detail.prev']))          { goPrev(); return; }
      if (matchesKey(e, kb['photo_detail.next']))          { goNext(); return; }
      if (matchesKey(e, kb['photo_detail.zoom_in']) || (kb['photo_detail.zoom_in'] === '+' && e.key === '=' && !e.ctrlKey)) { viewerRef.current?.zoomIn(); return; }
      if (matchesKey(e, kb['photo_detail.zoom_out']))      { viewerRef.current?.zoomOut(); return; }
      if (matchesKey(e, kb['photo_detail.fit']))           { viewerRef.current?.fitToScreen(); return; }
      if (matchesKey(e, kb['photo_detail.rotate_left']))   { viewerRef.current?.rotateLeft(); return; }
      if (matchesKey(e, kb['photo_detail.rotate_right']))  { viewerRef.current?.rotateRight(); return; }
      if (matchesKey(e, kb['photo_detail.cull']))          { handleCulledChange(!localCulled); return; }
      if (matchesKey(e, kb['photo_detail.add_tag']))       { e.preventDefault(); if (!readOnly) tagManagerRef.current?.startAdding(); return; }
      if (matchesKey(e, kb['photo_detail.toggle_raw']))    { if (format === 'RAW+JPG') setShowRaw((v) => !v); return; }
      if (matchesKey(e, kb['photo_detail.add_to_album']))  { e.preventDefault(); openAddToAlbum(); return; }
      if (matchesKey(e, kb['photo_detail.stars_0']))       { handleStarsChange(0); return; }
      if (matchesKey(e, kb['photo_detail.stars_1']))       { handleStarsChange(1); return; }
      if (matchesKey(e, kb['photo_detail.stars_2']))       { handleStarsChange(2); return; }
      if (matchesKey(e, kb['photo_detail.stars_3']))       { handleStarsChange(3); return; }
      if (matchesKey(e, kb['photo_detail.stars_4']))       { handleStarsChange(4); return; }
      if (matchesKey(e, kb['photo_detail.stars_5']))       { handleStarsChange(5); return; }
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [kb, onClose, goPrev, goNext, localCulled, handleCulledChange, localColorLabels, localTags, format, readOnly, albumTarget, openAddToAlbum]);

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
        {readOnly ? (
          <span style={{ fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '12px', color: 'var(--lumik-secondary, #e9c349)', flexShrink: 0 }}>
            {t('projectLock.readOnly')}
          </span>
        ) : (
        <button
          onClick={handleCoverToggle}
          title={isCover ? t('photo.detail.removeCover') : t('photo.detail.setCover')}
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
        )}
        <button
          onClick={openAddToAlbum}
          title={t('albums.add.button')}
          aria-label={t('albums.add.button')}
          style={{
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            width: '32px', height: '32px', padding: 0, flexShrink: 0,
            background: 'transparent', border: 'none',
            borderRadius: 'var(--lumik-radius, 4px)', cursor: 'pointer',
            color: 'var(--lumik-outline, #8c90a0)',
          }}
        >
          <Icon name="album" size={18} />
        </button>
        <span style={{ fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)', fontSize: '12px', color: 'var(--lumik-outline, #8c90a0)', flexShrink: 0 }}>
          {currentIndex + 1} / {photos.length}
        </span>
        {saveState !== 'idle' && (
          <span style={{
            fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
            fontSize: '11px',
            color: saveState === 'error' ? 'var(--lumik-error, #ffb4ab)' : saveState === 'saved' ? '#27AE60' : saveState === 'saving' ? 'var(--lumik-secondary, #e9c349)' : 'var(--lumik-outline, #8c90a0)',
            flexShrink: 0,
          }}>
            {saveState === 'saving' && t('photo.saving')}
            {saveState === 'saved'  && t('photo.saved')}
            {saveState === 'error'  && t('photo.errorSaving')}
          </span>
        )}
      </div>

      {/* Body */}
      <div style={{ flex: 1, display: 'flex', overflow: 'hidden', position: 'relative' }}>
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
          onRotationChange={readOnly ? undefined : handleRotationChange}
          onHistogramReady={setHistogramBins}
          cornerOverlay={
            format && (
              <FormatBadge
                format={format}
                showing={format === 'RAW+JPG' ? (showRaw ? 'RAW' : 'JPG') : undefined}
                onToggle={format === 'RAW+JPG' ? () => setShowRaw((v) => !v) : undefined}
              />
            )
          }
        />

        {/* Tab to open info panel when collapsed */}
        {infoCollapsed && (
          <button
            onClick={() => { setInfoPanelCollapsed(false); onSidebarToggle?.(true); }}
            aria-label={t('photo.detail.showInfo')}
            style={{
              position: 'absolute',
              right: 0,
              top: '8px',
              zIndex: 10,
              width: '20px',
              height: '56px',
              border: '1px solid var(--lumik-outline-variant, #424654)',
              borderRight: 'none',
              borderRadius: '10px 0 0 10px',
              background: 'var(--lumik-surface-container-low, #1c1b1b)',
              color: 'var(--lumik-on-surface, #e3e2e9)',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: '14px',
              padding: 0,
            }}
          >
            ‹
          </button>
        )}

        {!infoCollapsed && (
          <PhotoSidebar
            photo={photo}
            histogramBins={histogramBins}
            localStars={localStars}
            localColorLabels={localColorLabels}
            localTags={localTags}
            localCulled={localCulled}
            tagSuggestions={tagSuggestions}
            tagManagerRef={tagManagerRef}
            onStarsChange={handleStarsChange}
            onColorChange={handleColorChange}
            onTagsChange={handleTagsChange}
            onCulledChange={handleCulledChange}
            onCollapse={isMobile ? () => { setInfoPanelCollapsed(true); onSidebarToggle?.(false); } : undefined}
          />
        )}
      </div>
      <AddToAlbumDialog target={albumTarget} onClose={() => setAlbumTarget(null)} />
    </div>
  );
}
