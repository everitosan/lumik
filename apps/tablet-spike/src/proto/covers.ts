// Portadas de las tarjetas del inicio: la elegida en el desktop
// (`project.cover_photo_path`) o, si no hay, la primera miniatura. Se copian al
// caché de la sandbox en segundo plano, una a la vez.
import { copyFdToPath, missingFiles, projectCover, startFdToPath } from '@lumik/core-native';

import Saf from '../../modules/saf/src/LumikSafModule';
import type { DiskProject } from './session';

const DIR = '/data/user/0/com.lumik.tabletspike/files/covers';
const safeName = (s: string) => s.replace(/[^\w.-]+/g, '_');

export const coverPath = (p: DiskProject) => `${DIR}/${safeName(p.name)}.jpg`;

/** Trae las portadas que faltan; avisa cada una lista con `onReady`. */
export async function fetchCovers(projects: DiskProject[], onReady: (p: DiskProject) => void) {
  const missing = new Set(missingFiles(DIR, projects.map((p) => `${safeName(p.name)}.jpg`)));
  for (const p of projects) {
    if (!missing.has(`${safeName(p.name)}.jpg`)) {
      onReady(p);
      continue;
    }
    try {
      const db = p.files.find((f) => f.name === 'project.db');
      let rel: string | undefined;
      if (db) {
        const copy = `${DIR}/${safeName(p.name)}.db`;
        copyFdToPath(Saf.openFd(db.uri, 'r'), copy);
        rel = projectCover(copy);
      }
      if (!rel) {
        const thumbs = p.files.find((f) => f.name === '.thumbs' && f.isDirectory);
        const first = thumbs && (await Saf.listChildren(thumbs.uri)).find((f) => f.name.endsWith('.jpg'));
        rel = first ? `.thumbs/${first.name}` : undefined;
      }
      if (!rel) continue;
      const uri = Saf.childUri(p.uri, rel);
      if (!Saf.exists(uri)) continue;
      await new Promise<void>((resolve) =>
        startFdToPath(Saf.openFd(uri, 'r'), coverPath(p), {
          onProgress: () => {},
          onDone: () => resolve(),
          onError: () => resolve(),
        }),
      );
      onReady(p);
    } catch (e) {
      console.warn('portada de', p.name, e);
    }
  }
}
