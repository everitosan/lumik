// Acceso a carpetas por SAF (Android). Ver `LumikSafModule.kt`.
import { NativeModule, requireNativeModule } from 'expo';

import type { SafEntry } from './LumikSaf.types';

/** Disco externo montado (USB o tarjeta). Capacidad y espacio libre, si Android los da. */
export type SafVolume = {
  uuid: string;
  label: string;
  readOnly: boolean;
  totalBytes: number | null;
  freeBytes: number | null;
};

declare class LumikSafModule extends NativeModule<{ onVolumesChanged: (e: { action: string }) => void }> {
  /** Discos externos montados; la memoria interna no cuenta. */
  volumes(): SafVolume[];
  /** Selector de carpeta abierto en la raíz del disco `uuid`; null si se cancela. */
  requestVolumeAccess(uuid: string): Promise<string | null>;
  /** Abre la carpeta en la app de Archivos del sistema; false si ninguna app la abre. */
  openFolder(documentUri: string): boolean;
  /** Pantalla de almacenamiento de Android (donde se expulsa un disco). */
  openStorageSettings(): void;
  pickDirectory(): Promise<string | null>;
  /** Selector de archivos con selección múltiple; lista vacía si se cancela. */
  pickFiles(): Promise<SafEntry[]>;
  persistedTrees(): string[];
  rootDocument(treeUri: string): string;
  listChildren(documentUri: string): Promise<SafEntry[]>;
  listChildrenSync(documentUri: string): SafEntry[];
  /** Documento en `rel` bajo `baseUri`, exista o no. */
  childUri(baseUri: string, rel: string): string;
  exists(uri: string): boolean;
  createFile(parentUri: string, name: string, mime: string): string;
  createDirectory(parentUri: string, name: string): string;
  /** fd despegado: quien lo recibe (Rust) lo cierra. */
  openFd(uri: string, mode: 'r' | 'w' | 'rw'): number;
  rename(uri: string, name: string): string;
  move(uri: string, fromParentUri: string, toParentUri: string): string;
  delete(uri: string): boolean;
}

export default requireNativeModule<LumikSafModule>('LumikSaf');
