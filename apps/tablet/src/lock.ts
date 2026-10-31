// Candado del proyecto en el disco (`.lumik-lock`, spec `project-lock`), leído
// y escrito por SAF. El JSON lo arma y lo interpreta el núcleo, así que es el
// mismo que escribe el desktop.
import {
  type InstallInfo,
  installIdentity,
  setInstallName,
  lockView,
  type LockView,
  ownLockJson,
  readFdString,
} from '@lumik/core-native';
import { Platform } from 'react-native';

import Saf from '../modules/saf/src/LumikSafModule';
import { replaceFile } from './safWrite';
import { FILES_DIR } from './paths';

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

/** Cambia el nombre con que esta tablet aparece en el candado de los proyectos. */
export function renameInstall(name: string): InstallInfo {
  const model = (Platform.constants as { Model?: string }).Model ?? 'Android';
  install = setInstallName(`${FILES_DIR}/install-id`, `Tablet ${model}`, name);
  return install;
}

export function readLock(projectUri: string): LockView {
  const uri = Saf.childUri(projectUri, LOCK_FILE);
  const text = Saf.exists(uri) ? readFdString(Saf.openFd(uri, 'r')) : undefined;
  const view = lockView(text, thisInstall().id);
  // Uno ilegible cuenta como libre: que quede rastro para saber si pasa.
  if (text !== undefined && view.state === 'free') console.warn(`${LOCK_FILE} ilegible en ${projectUri}: ${JSON.stringify(text)}`);
  return view;
}

/** Escribe el candado propio. SAF no renombra sobre un archivo existente, así
 *  que no es atómico como en el desktop: hay un instante sin candado. */
export function writeOwnLock(projectUri: string, mirrorDir: string, pending: boolean) {
  replaceFile(projectUri, LOCK_FILE, ownLockJson(thisInstall(), mirrorDir, pending));
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
