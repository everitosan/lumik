// Escribir un archivo pequeño del disco por SAF sin sobrescribirlo: en la
// Huawei, abrir uno existente en "wt" y escribirlo lo dejaba vacío al
// desmontar el disco (se vio con `lumik/.disk.json`). Se escribe un temporal
// nuevo, se borra el anterior y se renombra el temporal.
import { copyPathToFd, writeFdString } from '@lumik/core-native';

import Saf from '../modules/saf/src/LumikSafModule';

/** Escribe `text` en `name` dentro de `dirUri`. Entre borrar el anterior y
 *  renombrar hay un instante sin archivo. Como binario: con otro tipo, Android
 *  le agrega una extensión al nombre. */
export function replaceFile(dirUri: string, name: string, text: string) {
  replaceWith(dirUri, name, (fd) => writeFdString(fd, text));
}

/** Como `replaceFile`, con el contenido de un archivo de la sandbox (p. ej. una miniatura). */
export function replaceFileFromPath(dirUri: string, name: string, path: string) {
  replaceWith(dirUri, name, (fd) => copyPathToFd(path, fd));
}

function replaceWith(dirUri: string, name: string, write: (fd: number) => void) {
  const tmpName = `${name}.tmp`;
  const tmpUri = Saf.childUri(dirUri, tmpName);
  if (Saf.exists(tmpUri)) Saf.delete(tmpUri);
  const tmp = Saf.createFile(dirUri, tmpName, 'application/octet-stream');
  write(Saf.openFd(tmp, 'w'));
  const uri = Saf.childUri(dirUri, name);
  if (Saf.exists(uri)) Saf.delete(uri);
  Saf.rename(tmp, name);
}
