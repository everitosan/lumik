// El menú de proyectos del inicio (como el de las tarjetas del dashboard del
// desktop): editar, archivar o desarchivar y mandar a la papelera; y lo que se
// hace desde la sección "Papelera": restaurar y eliminar definitivamente. Cada acción toma el
// candado mientras dura y lo suelta al terminar (como `write_session`); si el
// proyecto lo tiene otro dispositivo, falla con `ProjectLockedError` y la
// pantalla ofrece tomar el control antes (`force`).
import {
  checkPhotoRestore,
  editProjectDetails,
  type LockView,
  moveDir,
  projectCard,
  projectRestoreRel,
  projectTrashRel,
  purgePhoto,
  restorePhotoAs,
  setProjectArchived,
} from '@lumik/core-native';

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

/** El lugar al que vuelve un elemento de la papelera ya está ocupado (spec trash-bin). */
export class RestoreCollisionError extends Error {
  constructor(readonly target: string) {
    super(`restore_collision:${target}`);
  }
}

/** `restore_collision:{archivo}` en el mensaje de un error del núcleo, si lo es. */
function collisionOf(e: unknown): string | null {
  const m = /restore_collision:(.+)$/.exec(String(e instanceof Error ? e.message : e));
  return m ? m[1].trim() : null;
}

/** Hora local `AAAA-MM-DD HH:MM:SS`, para la carpeta de la fecha en la papelera. */
function nowLocal(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

const basename = (rel: string) => rel.split('/').pop() ?? rel;

async function lumikDir(tree: string): Promise<string | undefined> {
  const root = Saf.rootDocument(tree);
  return (await Saf.listChildren(root)).find((e) => e.isDirectory && e.name === 'lumik')?.uri;
}

/** Mandar a la papelera, como el desktop: la carpeta completa pasa a
 *  `lumik/_trash/{fecha}/{ruta}` en el mismo disco, en un solo movimiento (el
 *  nombre de la carpeta no cambia). No se borra nada. */
/** Tiempos de cada paso de mover una carpeta, para mostrarlos en el registro de la pantalla. */
let lastMoveTimings: string[] = [];

/** Devuelve, además, cuánto tardó cada paso (se muestra en el registro de la pantalla). */
export async function trashProject(p: DiskProject, force = false): Promise<string> {
  const started = Date.now();
  const lock = readLock(p.uri);
  if (lock.state === 'foreign' && !force) throw new ProjectLockedError(lock);
  if (p.name === 'raíz' || !(await lumikDir(p.tree))) throw new Error('El proyecto no está bajo lumik/: no se puede mandar a la papelera');
  const uri = await moveFolder(p, projectTrashRel(p.name, nowLocal()), 0);
  // Un candado que haya viajado no sirve en la papelera.
  removeLock(uri);
  const summary = `${(Date.now() - started) / 1000} s (${lastMoveTimings.join(', ')})`;
  console.log(`[papelera] ${p.name}: a la papelera en ${summary}`);
  return summary;
}

/** ¿Está libre el lugar al que volvería el proyecto (con `newName` o con su nombre)? */
export async function checkProjectRestore(p: DiskProject, newName?: string): Promise<{ free: boolean; target: string }> {
  const lumik = await lumikDir(p.tree);
  const rel = projectRestoreRel(p.name, newName);
  return { free: !!lumik && !Saf.exists(Saf.childUri(lumik, rel)), target: basename(rel) };
}

/** Devuelve un proyecto de la papelera a su lugar. Con `newName` vuelve con ese
 *  nombre y sus fotos se renombran como al editarlo. Si el lugar está ocupado,
 *  lanza `RestoreCollisionError` sin mover nada. */
export async function restoreProject(p: DiskProject, newName?: string) {
  const check = await checkProjectRestore(p, newName);
  if (!check.free) throw new RestoreCollisionError(check.target);
  const rel = projectRestoreRel(p.name, newName);
  const uri = await moveFolder(p, rel, 1);
  removeLock(uri);
  if (!newName) return;

  const restored: DiskProject = { ...p, uri, name: rel };
  const dir = `${WORK}/${projectKey(restored)}`;
  await pullProjectDb(uri, dir);
  writeOwnLock(uri, dir, false);
  try {
    const card = projectCard(`${dir}/project.db`);
    if (!card) throw new Error('No se pudo leer el proyecto restaurado');
    // Como `restore_project` del desktop: cambia el nombre y las fotos, sin mover la carpeta.
    editProjectDetails(
      dir,
      newName,
      (card.sessionDate ?? card.createdAt).slice(0, 10),
      card.description ?? undefined,
      true,
      safDiskFiles(uri),
    );
    pushProjectDb(uri, dir);
  } finally {
    removeLock(uri);
  }
}

/** Borra para siempre un proyecto de la papelera (su carpeta completa). Solo
 *  bajo `lumik/_trash/`: nunca un proyecto activo. No toca el respaldo. */
export async function purgeProject(p: DiskProject) {
  const lumik = await lumikDir(p.tree);
  if (!lumik || !p.name.startsWith('_trash/')) throw new Error('No está en la papelera');
  if (Saf.exists(p.uri) && !Saf.delete(p.uri)) throw new Error('No se pudo borrar la carpeta del proyecto');
  await pruneEmpty(lumik, p.name.split('/').slice(0, -1), 1);
}

/** Restaura una foto suelta de la papelera (con `newStem`, con ese nombre base). */
export async function restoreTrashedPhoto(p: DiskProject, photoId: string, newStem: string | undefined, force = false) {
  try {
    await withProjectDb(p, force, (dir) => restorePhotoAs(dir, photoId, safDiskFiles(p.uri), newStem));
  } catch (e) {
    const target = collisionOf(e);
    throw target !== null ? new RestoreCollisionError(target) : e;
  }
}

/** El archivo que ocuparía el lugar al restaurar la foto, o `null` si está libre. Solo lee. */
export async function checkTrashedPhoto(p: DiskProject, photoId: string, newStem?: string): Promise<{ free: boolean; target: string }> {
  const dir = `${WORK}/check/${projectKey(p)}`;
  await pullProjectDb(p.uri, dir);
  const occupied = checkPhotoRestore(dir, photoId, safDiskFiles(p.uri), newStem);
  return { free: occupied === undefined || occupied === null, target: occupied ?? '' };
}

/** Elimina definitivamente una foto de la papelera: archivos y registro. */
export async function purgeTrashedPhoto(p: DiskProject, photoId: string, force = false) {
  await withProjectDb(p, force, (dir) => purgePhoto(dir, photoId, safDiskFiles(p.uri)));
}

export type BatchFailure = { id: string; name: string; reason: string };

/** Motivo de un fallo de lote: `collision`, el candado o el mensaje. */
function batchReason(e: unknown): string {
  return collisionOf(e) !== null ? 'collision' : String(e instanceof Error ? e.message : e);
}

/** Restaura varias fotos de un proyecto con una sola copia de `project.db`
 *  (spec trash-bin). Las que chocan se quedan en la papelera (`collision`). */
export async function restoreTrashedPhotos(p: DiskProject, photos: { id: string; name: string }[], force = false): Promise<BatchFailure[]> {
  const failed: BatchFailure[] = [];
  await withProjectDb(p, force, (dir) => {
    const disk = safDiskFiles(p.uri);
    for (const ph of photos) {
      try {
        restorePhotoAs(dir, ph.id, disk, undefined);
      } catch (e) {
        failed.push({ id: ph.id, name: ph.name, reason: batchReason(e) });
      }
    }
  });
  return failed;
}

/** Elimina definitivamente varias fotos de un proyecto con una sola copia de `project.db`. */
export async function purgeTrashedPhotos(p: DiskProject, photos: { id: string; name: string }[], force = false): Promise<BatchFailure[]> {
  const failed: BatchFailure[] = [];
  await withProjectDb(p, force, (dir) => {
    const disk = safDiskFiles(p.uri);
    for (const ph of photos) {
      try {
        purgePhoto(dir, ph.id, disk);
      } catch (e) {
        failed.push({ id: ph.id, name: ph.name, reason: batchReason(e) });
      }
    }
  });
  return failed;
}

/** Quita las carpetas vacías de `segments` (relativas a `lumik/`), de la más
 *  honda hacia arriba, sin quitar las primeras `keep` (p. ej. `_trash`). */
async function pruneEmpty(lumik: string, segments: string[], keep: number) {
  for (let i = segments.length; i > keep; i--) {
    const dirUri = Saf.childUri(lumik, segments.slice(0, i).join('/'));
    if (!Saf.exists(dirUri) || (await Saf.listChildren(dirUri)).length > 0) break;
    Saf.delete(dirUri);
  }
}

/** Mueve la carpeta del proyecto a `folder` bajo `lumik/` y quita las carpetas
 *  de año y mes que queden vacías. Un proyecto fuera de `lumik/` no se mueve. */
async function relocate(p: DiskProject, folder: string): Promise<string> {
  if (p.name === folder) return p.uri;
  if (p.name === 'raíz' || !(await lumikDir(p.tree))) return p.uri;
  return moveFolder(p, folder, 0);
}

/** Mueve la carpeta del proyecto de `p.name` a `folder` (las dos relativas a
 *  `lumik/`), creando las carpetas que falten, y quita las que quedan vacías sin
 *  tocar las primeras `keep` del origen (1 para no quitar `_trash/`). Falla si
 *  el destino ya existe. */
async function moveFolder(p: DiskProject, folder: string, keep: number): Promise<string> {
  const lumik = await lumikDir(p.tree);
  if (!lumik) throw new Error('El disco no tiene carpeta lumik/');

  const target = folder.split('/');
  const name = target.pop()!;
  let parent = lumik;
  for (const segment of target) {
    const child = Saf.childUri(parent, segment);
    parent = Saf.exists(child) ? child : Saf.createDirectory(parent, segment);
  }
  if (Saf.exists(Saf.childUri(parent, name))) throw new Error(`Ya existe una carpeta de proyecto en lumik/${folder}`);

  const current = p.name.split('/');
  const currentName = current.pop()!;
  const oldParent = Saf.childUri(lumik, current.join('/'));
  let uri = p.uri;
  // Tiempos de cada paso en el log (`[papelera]`): en un disco USB, Android
  // actualiza su índice de medios por cada archivo de la carpeta que se mueve.
  let t = Date.now();
  lastMoveTimings = [];
  const step = (what: string) => {
    lastMoveTimings.push(`${what} ${((Date.now() - t) / 1000).toFixed(1)} s`);
    t = Date.now();
  };
  if (current.join('/') !== target.join('/')) {
    uri = Saf.move(uri, oldParent, parent);
    step('mover');
  }
  if (currentName !== name) {
    uri = Saf.rename(uri, name);
    step('renombrar');
  }

  // Como el desktop: las carpetas de antes que quedaron vacías se quitan, sin pasar de `lumik/`.
  await pruneEmpty(lumik, current, keep);
  step('limpiar carpetas vacías');
  return uri;
}
