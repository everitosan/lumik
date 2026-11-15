// Lo que hay en la papelera de los discos conectados (spec trash-bin), como
// `list_trash` del desktop: los proyectos de `lumik/_trash/` y las fotos sueltas
// (`deleted = 1`) de los proyectos activos, agrupados por disco. Cada base se lee
// de una copia en la sandbox; nada se guarda.
import { copyFdToPath, openProjectTrash, type PhotoInfo, projectCard, type ProjectCard, projectTrashedAt, resetDir } from '@lumik/core-native';

import Saf from '../modules/saf/src/LumikSafModule';
import { FILES_DIR } from './paths';
import { discoverTrashed } from './projects';
import { type DiskProject, projectKey } from './session';

const DIR = `${FILES_DIR}/trash`;

export type TrashedProjectItem = {
  project: DiskProject;
  card?: ProjectCard;
  /** Portada en el disco (URI de SAF), si tiene. */
  cover?: string;
  /** Cuándo se mandó (`AAAA-MM-DDTHH:MM:SS`), de su ruta en la papelera. */
  trashedAt?: string;
};

export type TrashedPhotoItem = {
  project: DiskProject;
  projectName: string;
  info: PhotoInfo;
  /** Miniatura en el disco (URI de SAF). */
  thumb: string;
};

export type DiskTrash = { tree: string; projects: TrashedProjectItem[]; photos: TrashedPhotoItem[] };

export const trashCount = (disks: DiskTrash[] | null) => (disks ?? []).reduce((n, d) => n + d.projects.length + d.photos.length, 0);

/** Copia `project.db` del proyecto a la sandbox y devuelve la carpeta. */
function copyDb(p: DiskProject, kind: string): string | null {
  const db = p.files.find((f) => f.name === 'project.db') ?? (Saf.exists(Saf.childUri(p.uri, 'project.db')) ? { uri: Saf.childUri(p.uri, 'project.db') } : null);
  if (!db) return null;
  const dir = `${DIR}/${kind}/${projectKey(p)}`;
  resetDir(dir);
  copyFdToPath(Saf.openFd(db.uri, 'r'), `${dir}/project.db`);
  return dir;
}

/** La papelera de cada disco con algo en ella. `projects`: los activos ya descubiertos. */
export async function loadTrash(trees: string[], projects: DiskProject[], note: (text: string, error?: boolean) => void): Promise<DiskTrash[]> {
  const out: DiskTrash[] = [];
  for (const tree of trees) {
    const trashed: TrashedProjectItem[] = [];
    try {
      for (const p of await discoverTrashed(tree, note)) {
        let card: ProjectCard | undefined;
        try {
          const dir = copyDb(p, 'projects');
          card = dir ? (projectCard(`${dir}/project.db`) ?? undefined) : undefined;
        } catch (e) {
          note(`${p.name}: ${String(e)}`, true);
        }
        trashed.push({
          project: p,
          card,
          cover: card?.cover ? Saf.childUri(p.uri, card.cover) : undefined,
          trashedAt: projectTrashedAt(p.name) ?? undefined,
        });
      }
    } catch (e) {
      note(`Papelera: ${String(e)}`, true);
    }
    trashed.sort((a, b) => (b.trashedAt ?? '').localeCompare(a.trashedAt ?? ''));

    const photos: TrashedPhotoItem[] = [];
    // Solo los proyectos con carpeta `_trash/` pueden tener fotos en la papelera:
    // los demás no se copian (la lista de proyectos ya trae los hijos de cada carpeta).
    const withTrash = projects.filter((x) => x.tree === tree && x.files.some((f) => f.isDirectory && f.name === '_trash'));
    for (const p of withTrash) {
      // Cede el hilo de JS entre proyectos para no frenar la pantalla ni las portadas.
      await new Promise((r) => setTimeout(r, 0));
      try {
        const dir = copyDb(p, 'photos');
        if (!dir) continue;
        const infos = openProjectTrash(dir);
        if (infos.length === 0) continue;
        const projectName = projectCard(`${dir}/project.db`)?.name ?? p.name;
        for (const info of infos) photos.push({ project: p, projectName, info, thumb: Saf.childUri(p.uri, `.thumbs/${info.id}.jpg`) });
      } catch (e) {
        note(`${p.name}: ${String(e)}`, true);
      }
    }

    if (trashed.length > 0 || photos.length > 0) out.push({ tree, projects: trashed, photos });
  }
  return out;
}
