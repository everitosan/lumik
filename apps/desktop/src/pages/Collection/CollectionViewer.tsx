import { useEffect, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import * as api from '../../lib/api';
import type { Photo } from '../../lib/types';
import { PhotoDetail } from '../Project/PhotoDetail';

/** Lo que el visor puede cambiar de una foto y la vista de abajo refleja al instante. */
export interface PhotoEdits {
  stars?: number;
  /** 1 rojo … 5 púrpura */
  colors?: number[];
  culled?: boolean;
}

/** `"1,3"` → `[1, 3]` */
function colorsOf(label: string | null): number[] {
  return (label ?? '').split(',').map((c) => Number(c.trim())).filter((c) => c >= 1 && c <= 5);
}

/** Una foto del conjunto que se recorre: resultados de búsqueda o elementos de un álbum. */
export interface CollectionRef {
  projectId: string;
  photoId: string;
}

const overlayStyles: CSSProperties = {
  position: 'fixed',
  inset: 0,
  zIndex: 50,
  background: 'var(--lumik-surface-container-lowest, #0e0e0e)',
};

interface Props {
  /** Fotos en el orden en que se ven (solo las que se pueden abrir: disco conectado). */
  refs: CollectionRef[];
  /** Foto con la que se abre. */
  startPhotoId: string;
  onClose: () => void;
  /**
   * Una foto cambió en el visor. La vista de abajo lo aplica en su tarjeta sin
   * esperar al índice, que se pone al día unos segundos después.
   */
  onPhotoEdited?: (photoId: string, edits: PhotoEdits) => void;
}

/**
 * Visor encima de la búsqueda o de un álbum (specs `photo-search` y
 * `photo-albums`): las flechas recorren ese conjunto aunque sus fotos sean de
 * proyectos distintos, y cerrar vuelve a donde se abrió sin desmontarlo.
 */
export function CollectionViewer({ refs: initialRefs, startPhotoId: initialStart, onClose, onPhotoEdited }: Props) {
  const { t } = useTranslation();
  // La lista queda fija desde que se abre: un refresco de la vista de abajo no
  // debe hacer saltar el visor a otra foto.
  const [refs] = useState(initialRefs);
  const [startPhotoId] = useState(initialStart);
  const [photos, setPhotos] = useState<Photo[] | null>(null);
  const [names, setNames] = useState<Record<string, string>>({});
  const [covers, setCovers] = useState<Record<string, string | null>>({});
  const [index, setIndex] = useState(0);

  // Carga las fotos de cada proyecto del conjunto, en el orden del conjunto.
  useEffect(() => {
    let active = true;
    const projectIds = [...new Set(refs.map((r) => r.projectId))];
    Promise.all([
      api.getProjectsDashboard(),
      Promise.all(projectIds.map((id) => api.getProjectPhotos(id).catch(() => [] as Photo[]))),
    ]).then(([dashboard, perProject]) => {
      if (!active) return;
      const byId = new Map<string, Photo>();
      for (const list of perProject) for (const p of list) byId.set(p.id, p);
      const ordered = refs.map((r) => byId.get(r.photoId)).filter((p): p is Photo => !!p && !p.deleted);
      setNames(Object.fromEntries(dashboard.map((p) => [p.id, p.name])));
      setCovers(Object.fromEntries(dashboard.map((p) => [p.id, p.cover_photo_path])));
      setIndex(Math.max(0, ordered.findIndex((p) => p.id === startPhotoId)));
      setPhotos(ordered);
    });
    return () => { active = false; };
  }, [refs, startPhotoId]);

  if (photos === null) {
    return <div style={overlayStyles} aria-busy="true" />;
  }
  const photo = photos[index];
  if (!photo) {
    return (
      <div style={{ ...overlayStyles, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--lumik-on-surface-variant, #c2c6d7)', fontFamily: 'var(--lumik-font-primary, Inter)' }}>
        <button type="button" onClick={onClose} style={{ background: 'transparent', border: 'none', color: 'inherit', cursor: 'pointer', font: 'inherit' }}>
          {t('collection.unavailable')}
        </button>
      </div>
    );
  }

  return (
    <div style={overlayStyles}>
      <PhotoDetail
        photo={photo}
        photos={photos}
        currentIndex={index}
        projectName={names[photo.project_id] ?? ''}
        coverPhotoPath={covers[photo.project_id] ?? null}
        onClose={onClose}
        onNavigate={setIndex}
        onPhotoChanged={(id, updates) => {
          setPhotos((list) => list && list.map((p) => (p.id === id ? { ...p, ...updates } : p)));
          const edits: PhotoEdits = {};
          if (updates.stars !== undefined) edits.stars = updates.stars;
          if (updates.color_label !== undefined) edits.colors = colorsOf(updates.color_label);
          if (updates.culled !== undefined) edits.culled = updates.culled;
          if (Object.keys(edits).length > 0) onPhotoEdited?.(id, edits);
        }}
        onCoverPhotoChange={(photoId) =>
          setCovers((c) => ({ ...c, [photo.project_id]: photoId ? `.thumbs/${photoId}.jpg` : null }))
        }
      />
    </div>
  );
}
