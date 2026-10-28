// Acceso a carpetas por SAF (Android). Ver `LumikSafModule.kt`.
import { NativeModule, requireNativeModule } from 'expo';

import type { SafEntry } from './LumikSaf.types';

declare class LumikSafModule extends NativeModule<{}> {
  pickDirectory(): Promise<string | null>;
  persistedTrees(): string[];
  rootDocument(treeUri: string): string;
  listChildren(documentUri: string): Promise<SafEntry[]>;
  createFile(parentUri: string, name: string, mime: string): string;
  createDirectory(parentUri: string, name: string): string;
  /** fd despegado: quien lo recibe (Rust) lo cierra. */
  openFd(uri: string, mode: 'r' | 'w' | 'rw'): number;
  rename(uri: string, name: string): string;
  delete(uri: string): boolean;
}

export default requireNativeModule<LumikSafModule>('LumikSaf');
