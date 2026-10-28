// Lo que muestran las tarjetas del inicio: el id y el nombre del proyecto (de su
// `project.db`, no el de la carpeta) y la portada (la elegida en el desktop o,
// si no hay, la primera miniatura). Se traen en segundo plano, una a la vez.
import { copyFdToPath, missingFiles, projectSummary, startFdToPath } from '@lumik/core-native';

import Saf from '../../modules/saf/src/LumikSafModule';
import { type DiskProject, projectKey } from './session';

const DIR = '/data/user/0/com.lumik.tabletspike/files/covers';
export const coverPath = (p: DiskProject) => `${DIR}/${projectKey(p)}.jpg`;

/** `id`: el `project.id`, para listar una sola vez un proyecto que está en dos discos. */
export type CardInfo = { id?: string; title?: string; cover: boolean };

/** Trae nombre y portada de cada proyecto; avisa cada uno con `onReady`. */
export async function fetchCovers(projects: DiskProject[], onReady: (p: DiskProject, info: CardInfo) => void) {
  const missing = new Set(missingFiles(DIR, projects.map((p) => `${projectKey(p)}.jpg`)));
  for (const p of projects) {
    try {
      const db = p.files.find((f) => f.name === 'project.db');
      let rel: string | undefined;
      let id: string | undefined;
      let title: string | undefined;
      if (db) {
        const copy = `${DIR}/${projectKey(p)}.db`;
        copyFdToPath(Saf.openFd(db.uri, 'r'), copy);
        const summary = projectSummary(copy);
        id = summary.id;
        title = summary.name;
        rel = summary.cover;
      }
      if (!missing.has(`${projectKey(p)}.jpg`)) {
        onReady(p, { id, title, cover: true });
        continue;
      }
      if (!rel) {
        const thumbs = p.files.find((f) => f.name === '.thumbs' && f.isDirectory);
        const first = thumbs && (await Saf.listChildren(thumbs.uri)).find((f) => f.name.endsWith('.jpg'));
        rel = first ? `.thumbs/${first.name}` : undefined;
      }
      const uri = rel ? Saf.childUri(p.uri, rel) : undefined;
      if (!uri || !Saf.exists(uri)) {
        onReady(p, { id, title, cover: false });
        continue;
      }
      await new Promise<void>((resolve) =>
        startFdToPath(Saf.openFd(uri, 'r'), coverPath(p), {
          onProgress: () => {},
          onDone: () => resolve(),
          onError: () => resolve(),
        }),
      );
      onReady(p, { id, title, cover: true });
    } catch (e) {
      console.warn('portada de', p.name, e);
    }
  }
}
