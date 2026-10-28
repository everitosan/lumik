// Fotos del experimento 2.4: un proyecto real de Lumik (`project.db` con sus
// `.thumbs/` y `.previews/`) leído con el núcleo, y repetido hasta 2 000 en
// archivos con nombres distintos para que el caché de imágenes no los reutilice.
// El proyecto se copia a la sandbox con `adb shell run-as` (build de debug).
import { openProject, prepareGrid, type PhotoInfo } from '@lumik/core-native';

const FILES_DIR = '/data/user/0/com.lumik.tabletspike/files';
const PROJECT_DIR = `${FILES_DIR}/project`;
const GRID_DIR = `${FILES_DIR}/grid`;

export const PHOTO_COUNT = 2000;

export type Photo = {
  id: number;
  info: PhotoInfo;
  thumb: string;
  preview: string;
  /** Ruta sin `file://`, para el histograma en Rust. */
  previewPath: string;
};

export function loadPhotos(): Photo[] {
  const infos = openProject(PROJECT_DIR);
  prepareGrid(
    infos.map((p) => p.thumbPath),
    infos.map((p) => p.previewPath),
    GRID_DIR,
    PHOTO_COUNT,
  );
  return Array.from({ length: PHOTO_COUNT }, (_, id) => ({
    id,
    info: infos[id % infos.length],
    thumb: `file://${GRID_DIR}/thumbs/${id}.jpg`,
    preview: `file://${GRID_DIR}/previews/${id}.jpg`,
    previewPath: `${GRID_DIR}/previews/${id}.jpg`,
  }));
}
