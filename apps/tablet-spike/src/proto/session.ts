// Un proyecto del disco abierto en la tablet, en modo espejado (D2):
// `project.db` se trae a la sandbox, se edita ahí y vuelve al disco con
// `VACUUM INTO` + renombrado justo después de mover archivos y, con debounce,
// tras calificar o etiquetar. Las miniaturas se muestran del disco mientras se
// copian al caché de la sandbox en segundo plano (la primera vez de un proyecto
// de 1 000 fotos son unos 30 s); las vistas previas, cuando el visor las pide.
//
// Candado (`project-lock`): al abrir se toma si está libre o es propio; si es
// de otro dispositivo, la sesión queda en solo lectura. Mientras haya cambios
// sin devolver, el candado dice `pending`. Antes de escribir en el disco se
// relee: si otro lo tomó, la tablet deja de escribir.
import {
  type LockView,
  copyFdToPath,
  copyPathToFd,
  cullPhoto,
  getViewSettings,
  missingFiles,
  openProject,
  openProjectTrash,
  type PhotoInfo,
  ratePhoto,
  resetDir,
  restorePhoto,
  saveViewSettings,
  startCopyToDir,
  startFdToPath,
  trashPhoto,
  vacuumInto,
  type ViewSettings,
} from '@lumik/core-native';

import Saf from '../../modules/saf/src/LumikSafModule';
import type { SafEntry } from '../../modules/saf/src/LumikSaf.types';
import { safDiskFiles } from './disk';
import { readLock, removeLock, writeOwnLock } from './lock';

const FILES_DIR = '/data/user/0/com.lumik.tabletspike/files';
const BINARY = 'application/octet-stream';
const TMP_NAME = 'project.db.lumik-tmp';
// Tras calificar o etiquetar, cuánto esperar otro cambio antes de devolver.
const GIVE_BACK_DEBOUNCE_MS = 3000;
// fds abiertos a la vez al traer miniaturas.
const THUMB_BATCH = 100;

export type DiskProject = { name: string; uri: string; files: SafEntry[]; lock?: LockView };

export type SessionPhoto = {
  info: PhotoInfo;
  /** La del caché (`file://`) o, mientras llega, la del disco (`content://`). */
  thumb: string;
  /** Ruta en el caché; existe cuando `previewReady`. */
  previewPath: string;
  previewReady: boolean;
};

const safeName = (s: string) => s.replace(/[^\w.-]+/g, '_');

export class ProjectSession {
  photos: SessionPhoto[];
  /** Fotos en la papelera (`_trash/`), para el filtro "Papelera". */
  trash: SessionPhoto[];
  /** Filtros y modo de vista guardados en el proyecto (`project_settings`), como los deja el desktop. */
  readonly viewSettings: ViewSettings;
  private giveBackTimer: ReturnType<typeof setTimeout> | null = null;
  private dirty = false;
  private fetching = new Set<string>();
  /** Candado de otro dispositivo: no se escribe nada. */
  readOnly: LockView | null;
  private pendingInLock = false;

  private constructor(
    readonly project: DiskProject,
    readonly mirrorDir: string,
    readonly cacheDir: string,
    infos: PhotoInfo[],
    trashed: PhotoInfo[],
    private readonly onChange: () => void,
    private readonly log: (text: string, error?: boolean) => void,
    lock: LockView,
  ) {
    this.readOnly = lock.state === 'foreign' ? lock : null;
    this.photos = this.wrap(infos);
    this.trash = this.wrap(trashed);
    this.viewSettings = getViewSettings(mirrorDir);
  }

  /** Con lo que ya estaba en el caché de una apertura anterior. */
  private wrap(infos: PhotoInfo[]): SessionPhoto[] {
    const names = infos.map((i) => `${i.id}.jpg`);
    const noThumb = new Set(missingFiles(`${this.cacheDir}/thumbs`, names));
    const noPreview = new Set(missingFiles(`${this.cacheDir}/previews`, names));
    return infos.map((info, i) => ({
      info,
      thumb: noThumb.has(names[i])
        ? Saf.childUri(this.project.uri, `.thumbs/${names[i]}`)
        : `file://${this.cacheDir}/thumbs/${names[i]}`,
      previewPath: `${this.cacheDir}/previews/${names[i]}`,
      previewReady: !noPreview.has(names[i]),
    }));
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
    const trashed = openProjectTrash(mirrorDir);
    const lock = readLock(project.uri);
    if (lock.state !== 'foreign') writeOwnLock(project.uri, mirrorDir, false);
    log(`${project.name}: ${infos.length} fotos (base en ${Date.now() - started} ms, candado ${lock.state})`);
    return new ProjectSession({ ...project, files }, mirrorDir, `${FILES_DIR}/cache/${key}`, infos, trashed, onChange, log, lock);
  }

  /** Copia al caché las miniaturas que falten, avisando el avance. */
  async fetchThumbs(onProgress: (done: number, total: number) => void): Promise<void> {
    const started = Date.now();
    const dir = `${this.cacheDir}/thumbs`;
    const all = [...this.photos, ...this.trash];
    const wanted = all.map((p) => `${p.info.id}.jpg`);
    const missing = new Set(missingFiles(dir, wanted));
    if (!missing.size) return;
    const thumbsDir = this.project.files.find((f) => f.name === '.thumbs' && f.isDirectory);
    if (!thumbsDir) {
      this.log('El proyecto no tiene .thumbs/ en el disco', true);
      return;
    }
    const onDisk = (await Saf.listChildren(thumbsDir.uri)).filter((f) => missing.has(f.name));
    const byName = new Map(all.map((p) => [`${p.info.id}.jpg`, p]));
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
      // Esta tanda ya está en el caché: el grid la toma de ahí.
      for (const f of batch) {
        const p = byName.get(f.name);
        if (p) p.thumb = `file://${dir}/${f.name}`;
      }
      this.onChange();
    }
    this.log(`${onDisk.length} miniaturas al caché en ${((Date.now() - started) / 1000).toFixed(1)} s`);
  }

  /** Trae del disco la vista previa de `p` si aún no está. */
  ensurePreview(p: SessionPhoto | undefined) {
    if (!p || p.previewReady || this.fetching.has(p.info.id)) return;
    const id = p.info.id;
    this.fetching.add(id);
    const uri = Saf.childUri(this.project.uri, `.previews/${p.info.id}.jpg`);
    if (!Saf.exists(uri)) return; // Sin vista previa en el disco: se queda la miniatura.
    startFdToPath(Saf.openFd(uri, 'r'), p.previewPath, {
      onProgress: () => {},
      onDone: () => {
        p.previewReady = true;
        this.fetching.delete(id);
        this.onChange();
      },
      onError: (m) => {
        this.fetching.delete(id);
        this.log(`Vista previa de ${p.info.fileName}: ${m}`, true);
      },
    });
  }

  /** Antes de tocar el disco: el candado sigue siendo nuestro, o se pasa a solo lectura. */
  private ensureOwned() {
    if (this.readOnly) throw new Error(`Solo lectura: lo tiene ${this.readOnly.installName}`);
    const lock = readLock(this.project.uri);
    if (lock.state === 'foreign') {
      this.readOnly = lock;
      this.onChange();
      throw new Error(`${lock.installName} tomó el control del proyecto; no se guardó el cambio.`);
    }
  }

  rate(p: SessionPhoto, stars: number, labels: number[]) {
    if (this.readOnly) throw new Error(`Solo lectura: lo tiene ${this.readOnly.installName}`);
    const colorLabel = labels.length ? labels.join(',') : undefined;
    ratePhoto(this.mirrorDir, p.info.id, stars, colorLabel);
    p.info = { ...p.info, stars, colorLabel };
    this.onChange();
    this.markPending();
  }

  /** Guarda los filtros y el modo de vista en el proyecto, como el desktop.
   *  En solo lectura se quedan en memoria. */
  saveView(settings: ViewSettings) {
    if (this.readOnly) return;
    saveViewSettings(this.mirrorDir, settings);
    this.markPending();
  }

  /** El disco ya no tiene todo: que lo sepa quien vea el candado; vuelve con debounce. */
  private markPending() {
    if (!this.pendingInLock) {
      this.ensureOwned();
      writeOwnLock(this.project.uri, this.mirrorDir, true);
      this.pendingInLock = true;
    }
    this.scheduleGiveBack();
  }

  /** Mueve la foto a la papelera con el `TrashPhoto` del núcleo (sus archivos van a `_trash/`). */
  trashPhoto(p: SessionPhoto) {
    this.ensureOwned();
    trashPhoto(this.mirrorDir, p.info.id, safDiskFiles(this.project.uri));
    this.reload();
    this.log(`A la papelera: ${p.info.fileName}`);
    this.giveBack();
  }

  /** Restaura la foto de la papelera con el `RestorePhoto` del núcleo. */
  restorePhoto(p: SessionPhoto) {
    this.ensureOwned();
    restorePhoto(this.mirrorDir, p.info.id, safDiskFiles(this.project.uri));
    this.reload();
    this.log(`Restaurada: ${p.info.fileName}`);
    this.giveBack();
  }

  /** Vuelve a leer las dos listas de la copia de trabajo, conservando lo ya
   *  traído al caché de cada foto. */
  private reload() {
    const known = new Map([...this.photos, ...this.trash].map((p) => [p.info.id, p]));
    const rewrap = (infos: PhotoInfo[]) => {
      const fresh = new Map(this.wrap(infos.filter((i) => !known.has(i.id))).map((p) => [p.info.id, p]));
      return infos.map((info) => {
        const p = known.get(info.id);
        if (!p) return fresh.get(info.id)!;
        p.info = info;
        return p;
      });
    };
    this.photos = rewrap(openProject(this.mirrorDir));
    this.trash = rewrap(openProjectTrash(this.mirrorDir));
    this.onChange();
  }

  /** Aprueba o quita la aprobación con el `CullPhoto` del núcleo, sobre el disco. */
  approve(p: SessionPhoto, approved: boolean) {
    const started = Date.now();
    this.ensureOwned();
    cullPhoto(this.mirrorDir, p.info.id, approved, safDiskFiles(this.project.uri));
    // El núcleo movió el archivo: la ruta nueva está en la copia de trabajo.
    const fresh = openProject(this.mirrorDir).find((x) => x.id === p.info.id);
    if (fresh) p.info = fresh;
    this.onChange();
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
      this.ensureOwned();
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
      writeOwnLock(this.project.uri, this.mirrorDir, false);
      this.pendingInLock = false;
      this.log(`project.db devuelto al disco (${Date.now() - started} ms)`);
    } catch (e) {
      this.dirty = true;
      this.log(`No se pudo devolver project.db: ${String(e)}`, true);
    }
  }

  /** Al salir o pasar a segundo plano: devuelve si quedó algo pendiente. */
  flush() {
    if (this.dirty && !this.readOnly) this.giveBack();
  }

  /** Al salir del proyecto: devuelve lo pendiente y suelta el candado si es propio. */
  close() {
    this.flush();
    if (this.readOnly || this.dirty) return; // Con cambios sin devolver, el candado se queda con `pending`.
    if (readLock(this.project.uri).state === 'own') removeLock(this.project.uri);
  }

  /** Tomar el control: el candado pasa a esta tablet. Hay que volver a abrir
   *  la sesión para leer la base del disco tal como la dejó el otro. */
  static takeControl(project: DiskProject, mirrorDir: string) {
    writeOwnLock(project.uri, mirrorDir, false);
  }
}
