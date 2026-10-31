// `DiskFiles` sobre SAF: lo que el núcleo (`CullPhoto`) necesita del disco,
// con rutas relativas a la carpeta del proyecto. Es síncrono porque Rust lo
// llama dentro de su caso de uso.
import { copyFdToFd, DiskError, type DiskFiles, fingerprintFd } from '@lumik/core-native';

import Saf from '../modules/saf/src/LumikSafModule';

const BINARY = 'application/octet-stream';

const dirname = (rel: string) => rel.split('/').slice(0, -1).join('/');
const basename = (rel: string) => rel.split('/').pop() ?? rel;

function fail(what: string, e: unknown): never {
  throw new DiskError.Failed({ message: `${what}: ${String(e)}` });
}

export function safDiskFiles(projectUri: string): DiskFiles {
  const at = (rel: string) => Saf.childUri(projectUri, rel);

  const ensureDir = (rel: string) => {
    let parent = '';
    for (const part of rel.split('/').filter(Boolean)) {
      const next = parent ? `${parent}/${part}` : part;
      if (!Saf.exists(at(next))) Saf.createDirectory(at(parent), part);
      parent = next;
    }
  };

  return {
    createDirAll(rel) {
      try {
        ensureDir(rel);
      } catch (e) {
        fail(`crear ${rel}`, e);
      }
    },
    rename(from, to) {
      try {
        let uri = at(from);
        const fromDir = dirname(from);
        const toDir = dirname(to);
        if (fromDir !== toDir) {
          ensureDir(toDir);
          uri = Saf.move(uri, at(fromDir), at(toDir));
        }
        if (basename(from) !== basename(to)) Saf.rename(uri, basename(to));
      } catch (e) {
        fail(`mover ${from} → ${to}`, e);
      }
    },
    exists(rel) {
      return Saf.exists(at(rel));
    },
    copy(from, to) {
      try {
        ensureDir(dirname(to));
        const dst = Saf.createFile(at(dirname(to)), basename(to), BINARY);
        copyFdToFd(Saf.openFd(at(from), 'r'), Saf.openFd(dst, 'w'));
      } catch (e) {
        fail(`copiar ${from} → ${to}`, e);
      }
    },
    removeFile(rel) {
      try {
        Saf.delete(at(rel));
      } catch (e) {
        fail(`borrar ${rel}`, e);
      }
    },
    fingerprint(rel) {
      try {
        return fingerprintFd(Saf.openFd(at(rel), 'r'));
      } catch (e) {
        fail(`huella de ${rel}`, e);
      }
    },
    listFiles(rel) {
      try {
        const dir = at(rel);
        if (!Saf.exists(dir)) return [];
        return Saf.listChildrenSync(dir)
          .filter((f) => !f.isDirectory)
          .map((f) => ({ name: f.name, size: BigInt(Math.round(f.size)), modified: 0n }));
      } catch (e) {
        fail(`listar ${rel}`, e);
      }
    },
  };
}
