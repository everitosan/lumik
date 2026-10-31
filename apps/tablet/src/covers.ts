// Lo que muestran las tarjetas del inicio: los datos del proyecto de su
// `project.db` (nombre, fecha, fotos, estado, archivado; como `ProjectDashboard`
// del desktop) y la portada (la elegida o, si no hay, la primera foto por fecha
// de captura, como el desktop). Se traen en segundo plano, una a la vez.
import { copyFdToPath, missingFiles, projectCard, type ProjectCard, resetDir, startFdToPath } from '@lumik/core-native';

import Saf from '../modules/saf/src/LumikSafModule';
import { type DiskProject, projectKey } from './session';
import { FILES_DIR } from './paths';

const DIR = `${FILES_DIR}/covers`;
export const coverPath = (p: DiskProject) => `${DIR}/${projectKey(p)}.jpg`;

/** `id`: el `project.id`, para listar una sola vez un proyecto que está en dos discos. */
export type CardInfo = { id?: string; title?: string; card?: ProjectCard; cover: boolean };

/** Trae nombre y portada de cada proyecto; avisa cada uno con `onReady`. */
export async function fetchCovers(projects: DiskProject[], onReady: (p: DiskProject, info: CardInfo) => void) {
  const missing = new Set(missingFiles(DIR, projects.map((p) => `${projectKey(p)}.jpg`)));
  for (const p of projects) {
    try {
      const db = p.files.find((f) => f.name === 'project.db');
      let rel: string | undefined;
      let id: string | undefined;
      let title: string | undefined;
      let card: ProjectCard | undefined;
      if (db) {
        // Una carpeta por proyecto, vaciada antes: un `-wal` viejo no se mezcla con la copia nueva.
        const dir = `${DIR}/db/${projectKey(p)}`;
        resetDir(dir);
        const copy = `${dir}/project.db`;
        copyFdToPath(Saf.openFd(db.uri, 'r'), copy);
        card = projectCard(copy) ?? undefined;
        id = card?.id;
        title = card?.name;
        rel = card?.cover;
      }
      if (!missing.has(`${projectKey(p)}.jpg`)) {
        onReady(p, { id, title, card, cover: true });
        continue;
      }
      const uri = rel ? Saf.childUri(p.uri, rel) : undefined;
      if (!uri || !Saf.exists(uri)) {
        onReady(p, { id, title, card, cover: false });
        continue;
      }
      await new Promise<void>((resolve) =>
        startFdToPath(Saf.openFd(uri, 'r'), coverPath(p), {
          onProgress: () => {},
          onDone: () => resolve(),
          onError: () => resolve(),
        }),
      );
      onReady(p, { id, title, card, cover: true });
    } catch (e) {
      console.warn('portada de', p.name, e);
    }
  }
}
