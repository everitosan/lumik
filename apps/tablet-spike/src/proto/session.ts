// Un proyecto del disco abierto en la tablet, en modo espejado (D2):
// `project.db` se trae a la sandbox, se edita ahí y vuelve al disco con
// `VACUUM INTO` + renombrado justo después de mover archivos y, con debounce,
// tras calificar o etiquetar. Las miniaturas se copian al caché de la sandbox
// al abrir; las vistas previas, cuando el visor las va a necesitar.
import {
  copyFdToPath,
  copyPathToFd,
  cullPhoto,
  missingFiles,
  openProject,
  type PhotoInfo,
  ratePhoto,
  resetDir,
  startCopyToDir,
  startFdToPath,
  vacuumInto,
} from '@lumik/core-native';

import Saf from '../../modules/saf/src/LumikSafModule';
import type { SafEntry } from '../../modules/saf/src/LumikSaf.types';
import { safDiskFiles } from './disk';

const FILES_DIR = '/data/user/0/com.lumik.tabletspike/files';
const BINARY = 'application/octet-stream';
const TMP_NAME = 'project.db.lumik-tmp';
// Tras calificar o etiquetar, cuánto esperar otro cambio antes de devolver.
const GIVE_BACK_DEBOUNCE_MS = 3000;
// fds abiertos a la vez al traer miniaturas.
const THUMB_BATCH = 100;

export type DiskProject = { name: string; uri: string; files: SafEntry[] };

export type SessionPhoto = {
  info: PhotoInfo;
  thumb: string;
  /** Ruta en el caché; existe cuando `previewReady`. */
  previewPath: string;
  previewReady: boolean;
};

const safeName = (s: string) => s.replace(/[^\w.-]+/g, '_');

export class ProjectSession {
  readonly photos: SessionPhoto[];
  private giveBackTimer: ReturnType<typeof setTimeout> | null = null;
  private dirty = false;
  private fetching = new Set<number>();

  private constructor(
    readonly project: DiskProject,
    readonly mirrorDir: string,
    readonly cacheDir: string,
    infos: PhotoInfo[],
    private readonly onChange: () => void,
    private readonly log: (text: string, error?: boolean) => void,
  ) {
    this.photos = infos.map((info) => ({
      info,
      thumb: `file://${cacheDir}/thumbs/${info.id}.jpg`,
      previewPath: `${cacheDir}/previews/${info.id}.jpg`,
      previewReady: false,
    }));
    // Las vistas previas que ya estaban en el caché de una apertura anterior.
    const names = infos.map((i) => `${i.id}.jpg`);
    const missing = new Set(missingFiles(`${cacheDir}/previews`, names));
    this.photos.forEach((p, i) => (p.previewReady = !missing.has(names[i])));
  }

  /** Trae `project.db` (y su `-wal`) a la sandbox y abre la copia de trabajo. */
  static async open(
    project: DiskProject,
    onChange: () => void,
    log: (text: string, error?: boolean) => void,
  ): Promise<ProjectSession> {
    const started = Date.now();
    const key = safeName(project.name);
    const mirrorDir = `${FILES_DIR}/mirror/${key}`;
    resetDir(mirrorDir);
    const files = await Saf.listChildren(project.uri);
    for (const suffix of ['', '-wal']) {
      const f = files.find((e) => e.name === `project.db${suffix}`);
      if (f) copyFdToPath(Saf.openFd(f.uri, 'r'), `${mirrorDir}/project.db${suffix}`);
    }
    const infos = openProject(mirrorDir);
    log(`${project.name}: ${infos.length} fotos (base en ${Date.now() - started} ms)`);
    return new ProjectSession({ ...project, files }, mirrorDir, `${FILES_DIR}/cache/${key}`, infos, onChange, log);
  }

  /** Copia al caché las miniaturas que falten, avisando el avance. */
  async fetchThumbs(onProgress: (done: number, total: number) => void): Promise<void> {
    const started = Date.now();
    const dir = `${this.cacheDir}/thumbs`;
    const wanted = this.photos.map((p) => `${p.info.id}.jpg`);
    const missing = new Set(missingFiles(dir, wanted));
    if (!missing.size) return;
    const thumbsDir = this.project.files.find((f) => f.name === '.thumbs' && f.isDirectory);
    if (!thumbsDir) {
      this.log('El proyecto no tiene .thumbs/ en el disco', true);
      return;
    }
    const onDisk = (await Saf.listChildren(thumbsDir.uri)).filter((f) => missing.has(f.name));
    let done = 0;
    for (let i = 0; i < onDisk.length; i += THUMB_BATCH) {
      const batch = onDisk.slice(i, i + THUMB_BATCH);
      await new Promise<void>((resolve, reject) =>
        startCopyToDir(
          batch.map((f) => Saf.openFd(f.uri, 'r')),
          batch.map((f) => f.name),
          dir,
          {
            onProgress: (n) => onProgress(done + Number(n), onDisk.length),
            onDone: () => resolve(),
            onError: (m) => reject(new Error(m)),
          },
        ),
      );
      done += batch.length;
    }
    this.log(`${onDisk.length} miniaturas al caché en ${((Date.now() - started) / 1000).toFixed(1)} s`);
  }

  /** Trae del disco la vista previa de la foto `i` si aún no está. */
  ensurePreview(i: number) {
    const p = this.photos[i];
    if (!p || p.previewReady || this.fetching.has(i)) return;
    this.fetching.add(i);
    const uri = Saf.childUri(this.project.uri, `.previews/${p.info.id}.jpg`);
    if (!Saf.exists(uri)) return; // Sin vista previa en el disco: se queda la miniatura.
    startFdToPath(Saf.openFd(uri, 'r'), p.previewPath, {
      onProgress: () => {},
      onDone: () => {
        p.previewReady = true;
        this.fetching.delete(i);
        this.onChange();
      },
      onError: (m) => {
        this.fetching.delete(i);
        this.log(`Vista previa de ${p.info.fileName}: ${m}`, true);
      },
    });
  }

  rate(i: number, stars: number, labels: number[]) {
    const p = this.photos[i];
    const colorLabel = labels.length ? labels.join(',') : undefined;
    ratePhoto(this.mirrorDir, p.info.id, stars, colorLabel);
    p.info = { ...p.info, stars, colorLabel };
    this.scheduleGiveBack();
  }

  /** Aprueba o quita la aprobación con el `CullPhoto` del núcleo, sobre el disco. */
  approve(i: number, approved: boolean) {
    const p = this.photos[i];
    const started = Date.now();
    cullPhoto(this.mirrorDir, p.info.id, approved, safDiskFiles(this.project.uri));
    // El núcleo movió el archivo: la ruta nueva está en la copia de trabajo.
    const fresh = openProject(this.mirrorDir).find((x) => x.id === p.info.id);
    if (fresh) p.info = fresh;
    this.log(`${approved ? 'Aprobada' : 'Aprobación quitada'}: ${p.info.dngPath} (${Date.now() - started} ms)`);
    // Se movieron archivos: la base vuelve al disco ya, para que coincidan.
    this.giveBack();
  }

  private scheduleGiveBack() {
    this.dirty = true;
    if (this.giveBackTimer) clearTimeout(this.giveBackTimer);
    this.giveBackTimer = setTimeout(() => this.giveBack(), GIVE_BACK_DEBOUNCE_MS);
  }

  /** Devuelve `project.db` al disco: temporal, borrar el original y renombrar. */
  giveBack() {
    if (this.giveBackTimer) clearTimeout(this.giveBackTimer);
    this.giveBackTimer = null;
    this.dirty = false;
    const started = Date.now();
    try {
      const out = `${this.mirrorDir}/out.db`;
      vacuumInto(`${this.mirrorDir}/project.db`, out);
      const stale = Saf.childUri(this.project.uri, TMP_NAME);
      if (Saf.exists(stale)) Saf.delete(stale);
      const tmp = Saf.createFile(this.project.uri, TMP_NAME, BINARY);
      copyPathToFd(out, Saf.openFd(tmp, 'w'));
      for (const name of ['project.db', 'project.db-wal', 'project.db-shm']) {
        const f = Saf.childUri(this.project.uri, name);
        if (Saf.exists(f)) Saf.delete(f);
      }
      Saf.rename(tmp, 'project.db');
      this.log(`project.db devuelto al disco (${Date.now() - started} ms)`);
    } catch (e) {
      this.dirty = true;
      this.log(`No se pudo devolver project.db: ${String(e)}`, true);
    }
  }

  /** Al salir o pasar a segundo plano: devuelve si quedó algo pendiente. */
  flush() {
    if (this.dirty) this.giveBack();
  }
}
