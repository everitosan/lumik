// Proyectos de un disco por SAF, buscados como el desktop
// (`discover_projects_on_device`): bajo `lumik/` si existe, hasta 4 niveles, y
// sin bajar más al encontrar un `project.db`.
import Saf from '../../modules/saf/src/LumikSafModule';
import { readLock } from './lock';
import type { DiskProject } from './session';

const MAX_DEPTH = 4;
const TMP_NAME = 'project.db.lumik-tmp';

export async function discoverProjects(tree: string, note: (text: string) => void): Promise<DiskProject[]> {
  const root = Saf.rootDocument(tree);
  const top = await Saf.listChildren(root);
  const lumik = top.find((e) => e.isDirectory && e.name === 'lumik');
  const found: DiskProject[] = [];
  await walk(lumik ? lumik.uri : root, '', 0, found, note);
  return found.sort((a, b) => a.name.localeCompare(b.name));
}

async function walk(dirUri: string, path: string, depth: number, out: DiskProject[], note: (text: string) => void) {
  if (depth > MAX_DEPTH) return;
  let files = await Saf.listChildren(dirUri);
  // Una devolución interrumpida entre borrar el original y renombrar deja
  // solo el temporal: se termina aquí.
  const tmp = files.find((f) => f.name === TMP_NAME);
  if (tmp && !files.some((f) => f.name === 'project.db')) {
    Saf.rename(tmp.uri, 'project.db');
    note(`${path || 'raíz'}: devolución interrumpida recuperada`);
    files = await Saf.listChildren(dirUri);
  }
  if (files.some((f) => f.name === 'project.db')) {
    // Quién lo tiene, para marcar la tarjeta (listar no toma el candado).
    const lock = files.some((f) => f.name === '.lumik-lock') ? readLock(dirUri) : undefined;
    out.push({ name: path || 'raíz', uri: dirUri, files, lock });
    return;
  }
  for (const dir of files.filter((f) => f.isDirectory && !f.name.startsWith('.'))) {
    await walk(dir.uri, path ? `${path}/${dir.name}` : dir.name, depth + 1, out, note);
  }
}
