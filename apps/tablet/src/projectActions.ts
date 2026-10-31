// El menú de proyectos del inicio (como el de las tarjetas del dashboard del
// desktop): editar, archivar o desarchivar y eliminar. Cada acción toma el
// candado mientras dura y lo suelta al terminar (como `write_session`); si el
// proyecto lo tiene otro dispositivo, falla con `ProjectLockedError` y la
// pantalla ofrece tomar el control antes (`force`).
import { editProjectDetails, type LockView, moveDir, setProjectArchived } from '@lumik/core-native';

import Saf from '../modules/saf/src/LumikSafModule';
import { safDiskFiles } from './disk';
import { readLock, removeLock, writeOwnLock } from './lock';
import { pullProjectDb, pushProjectDb } from './projectDb';
import { type DiskProject, FILES_DIR, projectKey } from './session';

const WORK = `${FILES_DIR}/menu`;

export class ProjectLockedError extends Error {
  constructor(readonly lock: LockView) {
    super(`Abierto en ${lock.installName}`);
  }
}

/** Trae la base, toma el candado, corre `fn` sobre la copia y la devuelve al disco. */
async function withProjectDb<T>(p: DiskProject, force: boolean, fn: (dir: string) => T): Promise<T> {
  const lock = readLock(p.uri);
  if (lock.state === 'foreign' && !force) throw new ProjectLockedError(lock);
  const dir = `${WORK}/${projectKey(p)}`;
  await pullProjectDb(p.uri, dir);
  writeOwnLock(p.uri, dir, false);
  try {
    const out = fn(dir);
    pushProjectDb(p.uri, dir);
    return out;
  } finally {
    removeLock(p.uri);
  }
}

export async function toggleArchived(p: DiskProject, archived: boolean, force = false) {
  await withProjectDb(p, force, (dir) => setProjectArchived(dir, archived));
}

export type EditForm = { name: string; date: string; description: string; renameFiles: boolean };

/** Editar proyecto, como el desktop: datos y, si se pide, nombres de las fotos;
 *  después la carpeta pasa a `lumik/{año}/{mes}/{día}_{nombre}`. Devuelve
 *  cuántas fotos se renombraron. */
export async function editProject(p: DiskProject, form: EditForm, force = false): Promise<{ renamed: number; moved: boolean }> {
  const lock = readLock(p.uri);
  if (lock.state === 'foreign' && !force) throw new ProjectLockedError(lock);
  const dir = `${WORK}/${projectKey(p)}`;
  await pullProjectDb(p.uri, dir);
  writeOwnLock(p.uri, dir, false);
  let uri = p.uri;
  try {
    const out = editProjectDetails(
      dir,
      form.name,
      form.date,
      form.description || undefined,
      form.renameFiles,
      safDiskFiles(p.uri),
    );
    // La base vuelve antes de mover la carpeta: si moverla falla, el proyecto
    // queda con sus datos nuevos en la carpeta de antes (se identifica por id).
    pushProjectDb(p.uri, dir);
    uri = await relocate(p, out.folder);
    if (uri !== p.uri) {
      const moved: DiskProject = { ...p, uri, name: out.folder };
      moveDir(`${FILES_DIR}/cache/${projectKey(p)}`, `${FILES_DIR}/cache/${projectKey(moved)}`);
    }
    return { renamed: out.renamedFiles, moved: uri !== p.uri };
  } finally {
    removeLock(uri);
  }
}

/** Eliminar proyecto, como el desktop: la carpeta y todo lo que tiene, para siempre. */
export function deleteProject(p: DiskProject, force = false) {
  const lock = readLock(p.uri);
  if (lock.state === 'foreign' && !force) throw new ProjectLockedError(lock);
  if (!Saf.delete(p.uri)) throw new Error('No se pudo borrar la carpeta del proyecto');
}

/** Mueve la carpeta del proyecto a `folder` bajo `lumik/` y quita las carpetas
 *  de año y mes que queden vacías. Un proyecto fuera de `lumik/` no se mueve. */
async function relocate(p: DiskProject, folder: string): Promise<string> {
  if (p.name === folder) return p.uri;
  const root = Saf.rootDocument(p.tree);
  const lumik = (await Saf.listChildren(root)).find((e) => e.isDirectory && e.name === 'lumik');
  if (!lumik || p.name === 'raíz') return p.uri;

  const target = folder.split('/');
  const name = target.pop()!;
  let parent = lumik.uri;
  for (const segment of target) {
    const child = Saf.childUri(parent, segment);
    parent = Saf.exists(child) ? child : Saf.createDirectory(parent, segment);
  }
  if (Saf.exists(Saf.childUri(parent, name))) throw new Error(`Ya existe una carpeta de proyecto en lumik/${folder}`);

  const current = p.name.split('/');
  const currentName = current.pop()!;
  const oldParent = Saf.childUri(lumik.uri, current.join('/'));
  let uri = p.uri;
  if (current.join('/') !== target.join('/')) uri = Saf.move(uri, oldParent, parent);
  if (currentName !== name) uri = Saf.rename(uri, name);

  // Como el desktop: las carpetas de antes que quedaron vacías se quitan, sin pasar de `lumik/`.
  for (let i = current.length; i > 0; i--) {
    const dirUri = Saf.childUri(lumik.uri, current.slice(0, i).join('/'));
    if ((await Saf.listChildren(dirUri)).length > 0) break;
    Saf.delete(dirUri);
  }
  return uri;
}
