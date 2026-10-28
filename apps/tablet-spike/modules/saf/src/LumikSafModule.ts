// Acceso a carpetas por SAF (Android). Ver `LumikSafModule.kt`.
import { NativeModule, requireNativeModule } from 'expo';

import type { SafEntry } from './LumikSaf.types';

declare class LumikSafModule extends NativeModule<{}> {
  pickDirectory(): Promise<string | null>;
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
