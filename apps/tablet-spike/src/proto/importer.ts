// Import de la tarjeta en la tablet (3.1, etapa B):
//  1. Marcadores en la sombra con los nombres que ya hay en el disco.
//  2. `ImportPhotos` del núcleo en la sombra (tarjeta por fd → renombrar, XMP,
//     pares, registro, miniaturas y vistas previas).
//  3. Subir al disco por SAF lo nuevo y devolver `project.db`.
import {
  type CardImportResult,
  DiskError,
  type ImportStatus,
  makePlaceholders,
  startCardImport,
  startPathToFd,
} from '@lumik/core-native';

import Saf from '../../modules/saf/src/LumikSafModule';
import type { SafEntry } from '../../modules/saf/src/LumikSaf.types';
import type { ProjectSession } from './session';

const BINARY = 'application/octet-stream';
// Lo que el import del desktop acepta (fotos); los videos quedan para después.
const PHOTO = /\.(raf|cr2|cr3|nef|arw|dng|orf|rw2|pef|srw|jpe?g|heic)$/i;

export type ImportProgress =
  | { step: 'import'; status: ImportStatus }
  | { step: 'upload'; done: number; total: number; bytes: number; totalBytes: number };

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

  // 2. El import del núcleo, en la sombra.
  const started = Date.now();
  const result = await new Promise<CardImportResult>((resolve, reject) =>
    startCardImport(
      shadow,
      card.map((f) => ({ name: f.name, size: BigInt(Math.round(f.size)) })),
      {
        open: (index) => {
          try {
            return Saf.openFd(card[index].uri, 'r');
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

  // 3. Subir: cada archivo a su misma ruta en el disco.
  const uploadStarted = Date.now();
  let uploadedBytes = 0;
  const totalBytes = 0; // El tamaño de cada archivo se conoce al copiarlo.
  for (const [i, rel] of result.uploads.entries()) {
    const parts = rel.split('/');
    const name = parts.pop()!;
    const dirRel = parts.join('/');
    ensureDir(disk, dirRel);
    const target = Saf.childUri(disk, rel);
    if (Saf.exists(target)) Saf.delete(target); // Solo puede ser una miniatura o vista previa vieja.
    const dst = Saf.createFile(Saf.childUri(disk, dirRel), name, BINARY);
    const copied = await new Promise<number>((resolve, reject) =>
      startPathToFd(`${shadow}/${rel}`, Saf.openFd(dst, 'w'), {
        onProgress: (b) =>
          onProgress({ step: 'upload', done: i, total: result.uploads.length, bytes: uploadedBytes + Number(b), totalBytes }),
        onDone: (r) => resolve(Number(r.bytes)),
        onError: (m) => reject(new Error(`${rel}: ${m}`)),
      }),
    );
    uploadedBytes += copied;
    onProgress({ step: 'upload', done: i + 1, total: result.uploads.length, bytes: uploadedBytes, totalBytes });
  }
  session.giveBack();
  return { result, importMs, uploadMs: Date.now() - uploadStarted, uploadedBytes };
}

function ensureDir(base: string, rel: string) {
  let parent = '';
  for (const part of rel.split('/').filter(Boolean)) {
    const next = parent ? `${parent}/${part}` : part;
    if (!Saf.exists(Saf.childUri(base, next))) Saf.createDirectory(Saf.childUri(base, parent), part);
    parent = next;
  }
}
