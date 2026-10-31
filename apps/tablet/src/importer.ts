// Import de la tarjeta en la tablet (spec `tablet-import`):
//  1. Marcadores en la sombra con los nombres que ya hay en el disco.
//  2. `ImportPhotos` del núcleo en la sombra. La tarjeta se lee una vez y cada
//     archivo se escribe a la vez en la sandbox y en `.import-staging/` del
//     disco (renombrar, XMP, pares, registro, miniaturas y vistas previas
//     pasan en la sandbox).
//  3. En el disco: mover cada original del staging a su nombre final, copiar
//     los archivos chicos (XMP, miniaturas, vistas previas), borrar el
//     staging y devolver `project.db`.
import {
  type CardImportResult,
  DiskError,
  type ImportStatus,
  makePlaceholders,
  startCardImport,
  startPathToFd,
} from '@lumik/core-native';

import Saf from '../modules/saf/src/LumikSafModule';
import type { SafEntry } from '../modules/saf/src/LumikSaf.types';
import { importSettings } from './prefs';
import type { ProjectSession } from './session';

const BINARY = 'application/octet-stream';
// Lo que el import del desktop acepta (fotos); los videos quedan para después.
const PHOTO = /\.(raf|cr2|cr3|nef|arw|dng|orf|rw2|pef|srw|jpe?g|heic)$/i;

export type ImportProgress =
  | { step: 'import'; status: ImportStatus }
  | { step: 'upload'; done: number; total: number; bytes: number; totalBytes: number };

// Staging del import en el disco, dentro del proyecto (como el desktop).
const STAGING = '.import-staging/tablet';

export type ImportReport = {
  result: CardImportResult;
  importMs: number;
  uploadMs: number;
  uploadedBytes: number;
};

/** ¿Lo acepta el import? (por extensión, como `isAllowedRawFile` del desktop) */
export const isImportable = (name: string) => PHOTO.test(name);

export async function importCard(
  session: ProjectSession,
  card: SafEntry[],
  onProgress: (p: ImportProgress) => void,
): Promise<ImportReport> {
  const shadow = session.mirrorDir;
  const disk = session.project.uri;

  // 1. Nombres ocupados en el disco.
  for (const sub of ['_media', '_culled', '_exported']) {
    const dir = Saf.childUri(disk, sub);
    const names = Saf.exists(dir) ? (await Saf.listChildren(dir)).map((f) => f.name) : [];
    makePlaceholders(`${shadow}/${sub}`, names);
  }

  // Staging del disco, vacío (un import interrumpido pudo dejar algo).
  const stagingUri = Saf.childUri(disk, STAGING);
  if (Saf.exists(stagingUri)) Saf.delete(stagingUri);
  ensureDir(disk, STAGING);
  const staged = new Map<number, string>();

  // 2. El import del núcleo, en la sombra.
  const started = Date.now();
  const result = await new Promise<CardImportResult>((resolve, reject) =>
    startCardImport(
      shadow,
      card.map((f) => ({ name: f.name, size: BigInt(Math.round(f.size)) })),
      importSettings(),
      {
        open: (index) => {
          try {
            return Saf.openFd(card[index].uri, 'r');
          } catch (e) {
            throw new DiskError.Failed({ message: String(e) });
          }
        },
        openDiskStaging: (index, name) => {
          try {
            const uri = Saf.createFile(stagingUri, stagingName(index, name), BINARY);
            staged.set(index, uri);
            return Saf.openFd(uri, 'w');
          } catch (e) {
            throw new DiskError.Failed({ message: String(e) });
          }
        },
      },
      {
        onStatus: (status) => onProgress({ step: 'import', status }),
        onDone: resolve,
        onError: (m) => reject(new Error(m)),
      },
    ),
  );
  const importMs = Date.now() - started;

  // 3a. Originales: ya están en el staging del disco, solo cambian de lugar y de nombre.
  const uploadStarted = Date.now();
  const steps = result.moves.length + result.uploads.length;
  for (const [i, move] of result.moves.entries()) {
    const from = staged.get(move.cardIndex);
    if (!from) throw new Error(`Sin copia en el disco de ${card[move.cardIndex]?.name}`);
    const parts = move.to.split('/');
    const name = parts.pop()!;
    const dirRel = parts.join('/');
    ensureDir(disk, dirRel);
    const moved = Saf.move(from, stagingUri, Saf.childUri(disk, dirRel));
    if (stagingName(move.cardIndex, card[move.cardIndex].name) !== name) Saf.rename(moved, name);
    onProgress({ step: 'upload', done: i + 1, total: steps, bytes: 0, totalBytes: 0 });
  }

  // 3b. Lo chico (XMP, miniaturas, vistas previas): se copia desde la sandbox.
  let uploadedBytes = 0;
  const totalBytes = 0; // El tamaño de cada archivo se conoce al copiarlo.
  for (const [k, rel] of result.uploads.entries()) {
    const i = result.moves.length + k;
    const parts = rel.split('/');
    const name = parts.pop()!;
    const dirRel = parts.join('/');
    ensureDir(disk, dirRel);
    const target = Saf.childUri(disk, rel);
    if (Saf.exists(target)) Saf.delete(target); // Solo puede ser una miniatura o vista previa vieja.
    const dst = Saf.createFile(Saf.childUri(disk, dirRel), name, BINARY);
    const copied = await new Promise<number>((resolve, reject) =>
      startPathToFd(`${shadow}/${rel}`, Saf.openFd(dst, 'w'), {
        onProgress: (b) => onProgress({ step: 'upload', done: i, total: steps, bytes: uploadedBytes + Number(b), totalBytes }),
        onDone: (r) => resolve(Number(r.bytes)),
        onError: (m) => reject(new Error(`${rel}: ${m}`)),
      }),
    );
    uploadedBytes += copied;
    onProgress({ step: 'upload', done: i + 1, total: steps, bytes: uploadedBytes, totalBytes });
  }
  // Lo que quedó en el staging (archivos que fallaron después de copiarse) se va.
  if (Saf.exists(stagingUri)) Saf.delete(stagingUri);
  const stagingRoot = Saf.childUri(disk, '.import-staging');
  if (Saf.exists(stagingRoot) && Saf.listChildrenSync(stagingRoot).length === 0) Saf.delete(stagingRoot);
  session.giveBack();
  return { result, importMs, uploadMs: Date.now() - uploadStarted, uploadedBytes };
}

// En el staging, el número evita choques entre archivos de carpetas distintas de la tarjeta.
const stagingName = (index: number, name: string) => `${String(index).padStart(5, '0')}_${name}`;

function ensureDir(base: string, rel: string) {
  let parent = '';
  for (const part of rel.split('/').filter(Boolean)) {
    const next = parent ? `${parent}/${part}` : part;
    if (!Saf.exists(Saf.childUri(base, next))) Saf.createDirectory(Saf.childUri(base, parent), part);
    parent = next;
  }
}
