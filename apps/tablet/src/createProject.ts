// Crear un proyecto en el disco desde la tablet, como `create_project` del
// desktop: `lumik/{año}/{mes}/{día}_{nombre}/` con su `project.db` y `_exported/`.
// La base se crea en la sandbox y se sube por SAF.
import { copyPathToFd, createProjectDb } from '@lumik/core-native';

import Saf from '../modules/saf/src/LumikSafModule';
import { defaultDelivery } from './delivery';
import { thisInstall } from './lock';
import { FILES_DIR } from './paths';

const WORK_DIR = `${FILES_DIR}/new-project`;
const BINARY = 'application/octet-stream';

/** `disk`: el permiso (árbol SAF) del disco donde va el proyecto. */
export type ProjectForm = { name: string; date: string; description: string; disk: string | null };

/** Crea el proyecto en `form.disk` y devuelve la URI de su carpeta. */
export async function createProject(form: ProjectForm): Promise<string> {
  const tree = form.disk;
  if (!tree) throw new Error('Elige el disco del proyecto');
  const created = createProjectDb(
    WORK_DIR,
    form.name,
    form.description.trim() || undefined,
    form.date.trim() || undefined,
    thisInstall().id,
    defaultDelivery(),
  );
  // Si se eligió la propia carpeta `lumik/` como disco, la ruta empieza debajo.
  const root = Saf.rootDocument(tree);
  const rootIsLumik = /(^|[:/])lumik$/.test(decodeURIComponent(root.split('/document/')[1] ?? ''));
  const rel = rootIsLumik ? created.dir.replace(/^lumik\//, '') : created.dir;

  const dirUri = Saf.childUri(root, rel);
  if (Saf.exists(Saf.childUri(dirUri, 'project.db'))) throw new Error(`Ya hay un proyecto en ${rel}`);
  let parent = '';
  for (const part of rel.split('/')) {
    const next = parent ? `${parent}/${part}` : part;
    if (!Saf.exists(Saf.childUri(root, next))) Saf.createDirectory(Saf.childUri(root, parent), part);
    parent = next;
  }
  if (!Saf.exists(Saf.childUri(dirUri, '_exported'))) Saf.createDirectory(dirUri, '_exported');
  const db = Saf.createFile(dirUri, 'project.db', BINARY);
  copyPathToFd(`${WORK_DIR}/project.db`, Saf.openFd(db, 'w'));
  return dirUri;
}

export const today = () => new Date().toISOString().slice(0, 10);
