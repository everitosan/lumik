// `project.db` de un proyecto del disco en modo espejado (D2): se trae a una
// carpeta de la sandbox para editarlo y vuelve con `VACUUM INTO` + temporal,
// borrar el original y renombrar (una devolución interrumpida deja solo el
// temporal, que `discoverProjects` termina de renombrar).
import { copyFdToPath, copyPathToFd, resetDir, vacuumInto } from '@lumik/core-native';

import Saf from '../../modules/saf/src/LumikSafModule';

const BINARY = 'application/octet-stream';
export const TMP_NAME = 'project.db.lumik-tmp';

/** Trae `project.db` (y su `-wal`) de la carpeta `projectUri` a `dir`, que se vacía antes. */
export async function pullProjectDb(projectUri: string, dir: string) {
  resetDir(dir);
  const files = await Saf.listChildren(projectUri);
  for (const suffix of ['', '-wal']) {
    const f = files.find((e) => e.name === `project.db${suffix}`);
    if (f) copyFdToPath(Saf.openFd(f.uri, 'r'), `${dir}/project.db${suffix}`);
  }
  return files;
}

/** Devuelve `dir/project.db` a la carpeta `projectUri`. */
export function pushProjectDb(projectUri: string, dir: string) {
  const out = `${dir}/out.db`;
  vacuumInto(`${dir}/project.db`, out);
  const stale = Saf.childUri(projectUri, TMP_NAME);
  if (Saf.exists(stale)) Saf.delete(stale);
  const tmp = Saf.createFile(projectUri, TMP_NAME, BINARY);
  copyPathToFd(out, Saf.openFd(tmp, 'w'));
  for (const name of ['project.db', 'project.db-wal', 'project.db-shm']) {
    const f = Saf.childUri(projectUri, name);
    if (Saf.exists(f)) Saf.delete(f);
  }
  Saf.rename(tmp, 'project.db');
}
