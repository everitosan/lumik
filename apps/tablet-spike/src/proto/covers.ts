// Lo que muestran las tarjetas del inicio: el nombre del proyecto (de su
// `project.db`, no el de la carpeta) y la portada (la elegida en el desktop o,
// si no hay, la primera miniatura). Se traen en segundo plano, una a la vez.
import { copyFdToPath, missingFiles, projectSummary, startFdToPath } from '@lumik/core-native';

import Saf from '../../modules/saf/src/LumikSafModule';
import type { DiskProject } from './session';

const DIR = '/data/user/0/com.lumik.tabletspike/files/covers';
const safeName = (s: string) => s.replace(/[^\w.-]+/g, '_');

export const coverPath = (p: DiskProject) => `${DIR}/${safeName(p.name)}.jpg`;

export type CardInfo = { title?: string; cover: boolean };

/** Trae nombre y portada de cada proyecto; avisa cada uno con `onReady`. */
export async function fetchCovers(projects: DiskProject[], onReady: (p: DiskProject, info: CardInfo) => void) {
  const missing = new Set(missingFiles(DIR, projects.map((p) => `${safeName(p.name)}.jpg`)));
  for (const p of projects) {
    try {
      const db = p.files.find((f) => f.name === 'project.db');
      let rel: string | undefined;
      let title: string | undefined;
      if (db) {
        const copy = `${DIR}/${safeName(p.name)}.db`;
        copyFdToPath(Saf.openFd(db.uri, 'r'), copy);
        const summary = projectSummary(copy);
        title = summary.name;
        rel = summary.cover;
      }
      if (!missing.has(`${safeName(p.name)}.jpg`)) {
        onReady(p, { title, cover: true });
        continue;
      }
      if (!rel) {
        const thumbs = p.files.find((f) => f.name === '.thumbs' && f.isDirectory);
        const first = thumbs && (await Saf.listChildren(thumbs.uri)).find((f) => f.name.endsWith('.jpg'));
        rel = first ? `.thumbs/${first.name}` : undefined;
      }
      const uri = rel ? Saf.childUri(p.uri, rel) : undefined;
      if (!uri || !Saf.exists(uri)) {
        onReady(p, { title, cover: false });
        continue;
      }
      await new Promise<void>((resolve) =>
        startFdToPath(Saf.openFd(uri, 'r'), coverPath(p), {
          onProgress: () => {},
          onDone: () => resolve(),
          onError: () => resolve(),
        }),
      );
      onReady(p, { title, cover: true });
    } catch (e) {
      console.warn('portada de', p.name, e);
    }
  }
}
