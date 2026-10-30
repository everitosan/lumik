// Nombre del disco (spec `device-detection`): Android no da la etiqueta del
// sistema de archivos, así que se usa `lumik/.disk.json`, que el desktop llena
// con la etiqueta o que se pone aquí a mano. El JSON lo arma y lo interpreta el
// núcleo. Sin nombre, una descripción genérica: tipo, capacidad y proyectos.
import { diskNameView, type DiskNameView, readFdString, userDiskNameJson } from '@lumik/core-native';

import Saf, { type SafVolume } from '../../modules/saf/src/LumikSafModule';
import { replaceFile } from './safWrite';

const FILE = '.disk.json';

export type DiskInfo = {
  /** Nombre guardado en el disco, si hay. */
  saved?: DiskNameView;
  kind: 'Disco' | 'Tarjeta de cámara';
};

/** "content://…/tree/FB5F-B3EF%3Alumik" → "FB5F-B3EF" (el uuid del volumen). */
export const volumeOf = (tree: string) => decodeURIComponent(tree.split('/tree/')[1] ?? tree).split(':')[0];

// Si se eligió la propia carpeta `lumik/` como disco, el archivo está en la raíz del permiso.
function rootIsLumik(root: string) {
  return /(^|[:/])lumik$/.test(decodeURIComponent(root.split('/document/')[1] ?? ''));
}

/** Nombre guardado y tipo del disco de un permiso. Lo que no se pueda leer no cuenta. */
export function readDiskInfo(tree: string): DiskInfo {
  try {
    const root = Saf.rootDocument(tree);
    const isLumik = rootIsLumik(root);
    const names = isLumik ? [] : Saf.listChildrenSync(root).map((e) => e.name);
    const kind = !isLumik && !names.includes('lumik') && names.includes('DCIM') ? 'Tarjeta de cámara' : 'Disco';
    const uri = Saf.childUri(isLumik ? root : Saf.childUri(root, 'lumik'), FILE);
    const text = Saf.exists(uri) ? readFdString(Saf.openFd(uri, 'r')) : undefined;
    const saved = diskNameView(text);
    if (text !== undefined && !saved) console.warn(`${FILE} ilegible en ${volumeOf(tree)}: ${JSON.stringify(text)}`);
    return { saved, kind };
  } catch {
    return { kind: 'Disco' };
  }
}

/** Guarda un nombre puesto a mano; crea `lumik/` si hace falta. Lanza si no
 *  queda guardado. */
export function writeUserDiskName(tree: string, name: string) {
  const root = Saf.rootDocument(tree);
  let dir = rootIsLumik(root) ? root : Saf.childUri(root, 'lumik');
  if (!Saf.exists(dir)) dir = Saf.createDirectory(root, 'lumik');
  replaceFile(dir, FILE, userDiskNameJson(name.trim()));
  const uri = Saf.childUri(dir, FILE);
  const saved = diskNameView(readFdString(Saf.openFd(uri, 'r')));
  if (saved?.name !== name.trim()) throw new Error('El nombre no quedó guardado en el disco');
}

/** "1.8 TB" */
export function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let i = 0;
  while (value >= 1000 && i < units.length - 1) {
    value /= 1000;
    i++;
  }
  return `${value >= 10 || i === 0 ? Math.round(value) : value.toFixed(1)} ${units[i]}`;
}

/** "Disco · 1.8 TB (1.2 TB libres) · 12 proyectos": para un disco sin nombre. */
export function genericName(kind: DiskInfo['kind'], volume: SafVolume | undefined, projects?: number): string {
  const parts: string[] = [kind];
  if (volume?.totalBytes) {
    const free = volume.freeBytes != null ? ` (${formatBytes(volume.freeBytes)} libres)` : '';
    parts.push(`${formatBytes(volume.totalBytes)}${free}`);
  }
  if (kind === 'Disco' && projects != null && projects > 0) parts.push(`${projects} ${projects === 1 ? 'proyecto' : 'proyectos'}`);
  return parts.join(' · ');
}
