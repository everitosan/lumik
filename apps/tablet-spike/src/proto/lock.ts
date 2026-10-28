// Candado del proyecto en el disco (`.lumik-lock`, spec `project-lock`), leído
// y escrito por SAF. El JSON lo arma y lo interpreta el núcleo, así que es el
// mismo que escribe el desktop.
import {
  type InstallInfo,
  installIdentity,
  lockView,
  type LockView,
  ownLockJson,
  readFdString,
  writeFdString,
} from '@lumik/core-native';
import { Platform } from 'react-native';

import Saf from '../../modules/saf/src/LumikSafModule';

const FILES_DIR = '/data/user/0/com.lumik.tabletspike/files';
const LOCK_FILE = '.lumik-lock';

let install: InstallInfo | null = null;

/** Identidad de esta tablet; el nombre por omisión lleva el modelo. */
export function thisInstall(): InstallInfo {
  if (!install) {
    const model = (Platform.constants as { Model?: string }).Model ?? 'Android';
    install = installIdentity(`${FILES_DIR}/install-id`, `Tablet ${model}`);
  }
  return install;
}

export function readLock(projectUri: string): LockView {
  const uri = Saf.childUri(projectUri, LOCK_FILE);
  const text = Saf.exists(uri) ? readFdString(Saf.openFd(uri, 'r')) : undefined;
  return lockView(text, thisInstall().id);
}

/** Escribe el candado propio. Se sobrescribe en el lugar: SAF no renombra
 *  sobre un archivo existente, así que no es atómico como en el desktop. */
export function writeOwnLock(projectUri: string, mirrorDir: string, pending: boolean) {
  const json = ownLockJson(thisInstall(), mirrorDir, pending);
  const uri = Saf.childUri(projectUri, LOCK_FILE);
  // Con otro tipo, Android le agrega la extensión (`.lumik-lock.json`) y el
  // desktop no lo vería: como binario, el nombre queda tal cual.
  const target = Saf.exists(uri) ? uri : Saf.createFile(projectUri, LOCK_FILE, 'application/octet-stream');
  writeFdString(Saf.openFd(target, 'w'), json);
}

export function removeLock(projectUri: string) {
  const uri = Saf.childUri(projectUri, LOCK_FILE);
  if (Saf.exists(uri)) Saf.delete(uri);
}

/** "Abierto en Tablet de Ana desde las 10:02" */
export function lockMessage(lock: LockView): string {
  const since = new Date(lock.since);
  const time = Number.isNaN(since.getTime())
    ? lock.since
    : since.toLocaleTimeString('es-MX', { hour: '2-digit', minute: '2-digit' });
  return `Abierto en ${lock.installName} desde las ${time}`;
}
