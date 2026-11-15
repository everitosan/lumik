// Sección "Papelera" (spec trash-bin): lo que hay en la papelera de cada disco
// conectado, agrupado por disco. Cada elemento se restaura o se elimina
// definitivamente, y cada disco se puede vaciar. Ver
// docs/useCases/flujo-papelera-proyecto.md y flujo-papelera-foto.md.
import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Icon, MarkedBar, PhotoCard, ProjectCard, photoFormat, type ColorLabel } from '@lumik/ui';
import * as api from '../../lib/api';
import { useTrash } from '../../lib/TrashContext';
import type { BatchFailure, DiskTrash, EmptyTrashReport, TrashItemRef, TrashedPhoto, TrashedProject } from '../../lib/types';
import { hasPlatformModifier } from '../../lib/hooks';
import { clickIntent, markRange, toggleMark } from '../../lib/marking';
import { BatchPurgeModal, BatchReportModal, EmptyTrashModal, PurgeModal, RestoreCollisionModal } from './TrashModals';

const pageStyles: CSSProperties = { display: 'flex', flexDirection: 'column', height: '100%', padding: '24px 32px', gap: '16px', overflow: 'hidden' };
const titleStyles: CSSProperties = { margin: 0, fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '24px', fontWeight: 600, color: 'var(--lumik-on-surface, #e5e2e1)' };
const mutedStyles: CSSProperties = { margin: 0, fontSize: '13px', lineHeight: 1.5, color: 'var(--lumik-on-surface-variant, #c2c6d7)' };
const errorStyles: CSSProperties = { ...mutedStyles, color: 'var(--lumik-error, #ffb4ab)' };
const scrollStyles: CSSProperties = { flex: 1, overflowY: 'auto', paddingRight: '8px', display: 'flex', flexDirection: 'column', gap: '32px' };
const diskHeaderStyles: CSSProperties = { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '12px', marginBottom: '16px' };
const diskTitleStyles: CSSProperties = { display: 'flex', alignItems: 'center', gap: '8px', margin: 0, fontSize: '16px', fontWeight: 600, color: 'var(--lumik-on-surface, #e5e2e1)' };
const subTitleStyles: CSSProperties = { margin: '0 0 12px', fontSize: '12px', fontWeight: 600, letterSpacing: '0.06em', textTransform: 'uppercase', color: 'var(--lumik-on-surface-variant, #c2c6d7)' };
const projectGridStyles: CSSProperties = { display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))', gap: '16px', marginBottom: '24px' };
const photoGridStyles: CSSProperties = { display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: '16px' };

const projectKey = (disk: DiskTrash, p: TrashedProject) => `project:${disk.device_uuid}:${p.rel_path}`;
const photoKey = (item: TrashedPhoto) => `photo:${item.project_id}:${item.photo.id}`;

function fileNameOf(path: string): string {
  return path.split('/').pop() ?? path;
}

function stemOf(name: string): string {
  const dot = name.lastIndexOf('.');
  return dot > 0 ? name.slice(0, dot) : name;
}

function parseColorLabels(raw: string | null): ColorLabel[] {
  if (!raw) return [];
  return raw
    .split(',')
    .map((n) => parseInt(n.trim(), 10))
    .filter((n) => n >= 1 && n <= 5) as ColorLabel[];
}

function TrashedProjectCard({
  disk,
  project,
  onRestore,
  onPurge,
  marked,
  onToggleMark,
  onClick,
}: {
  disk: DiskTrash;
  project: TrashedProject;
  onRestore: () => void;
  onPurge: () => void;
  marked: boolean;
  onToggleMark: () => void;
  onClick: (e: React.MouseEvent | React.KeyboardEvent) => void;
}) {
  const [cover, setCover] = useState<string | undefined>();
  useEffect(() => {
    let cancelled = false;
    api
      .getTrashedProjectCover(disk.device_uuid, project.rel_path)
      .then((url) => !cancelled && setCover(url ?? undefined))
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [disk.device_uuid, project.rel_path]);

  return (
    <ProjectCard
      name={project.name}
      photoCount={project.photo_count}
      date={project.session_date ?? project.created_at}
      driveName={disk.device_name}
      status={project.workflow_status}
      thumbnailUrl={cover}
      trashedAt={project.trashed_at ?? undefined}
      onRestore={onRestore}
      onPurge={onPurge}
      marked={marked}
      onToggleMark={onToggleMark}
      onClick={onClick}
    />
  );
}

function TrashedPhotoCard({
  item,
  onRestore,
  onPurge,
  marked,
  onToggleMark,
  onClick,
}: {
  item: TrashedPhoto;
  onRestore: () => void;
  onPurge: () => void;
  marked: boolean;
  onToggleMark: () => void;
  onClick: (e: React.MouseEvent | React.KeyboardEvent) => void;
}) {
  const { t } = useTranslation();
  const [url, setUrl] = useState<string | undefined>();
  useEffect(() => {
    let cancelled = false;
    api
      .getThumbnail(item.project_id, item.photo.id)
      .then((data) => !cancelled && setUrl(data ?? undefined))
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [item.project_id, item.photo.id]);

  const { photo } = item;
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
      <PhotoCard
        filename={photo.dng_path}
        thumbnailUrl={url}
        stars={photo.stars}
        culled={photo.culled}
        captureDate={photo.capture_date ?? undefined}
        colorLabels={parseColorLabels(photo.color_label)}
        format={photoFormat(photo.dng_path, photo.camera_jpg_path)}
        onRestore={onRestore}
        onPurge={onPurge}
        marked={marked}
        onToggleMark={onToggleMark}
        onClick={onClick}
      />
      <span style={{ ...mutedStyles, fontSize: '12px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
        {t('trash.photoOf', { project: item.project_name })}
      </span>
    </div>
  );
}

type PurgeTarget =
  | { kind: 'project'; disk: DiskTrash; project: TrashedProject }
  | { kind: 'photo'; item: TrashedPhoto };

type Collision =
  | { kind: 'project'; disk: DiskTrash; project: TrashedProject; occupied: string }
  | { kind: 'photo'; item: TrashedPhoto; occupied: string };

export function TrashPage() {
  const { t } = useTranslation();
  const { disks, error: loadError, refresh } = useTrash();

  const [actionError, setActionError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [purging, setPurging] = useState<PurgeTarget | null>(null);
  const [purgeError, setPurgeError] = useState<string | null>(null);
  const [emptying, setEmptying] = useState<DiskTrash | null>(null);
  const [emptyError, setEmptyError] = useState<string | null>(null);
  const [emptyReport, setEmptyReport] = useState<EmptyTrashReport | null>(null);
  const [collision, setCollision] = useState<Collision | null>(null);
  const [collisionError, setCollisionError] = useState<string | null>(null);

  // ---- Marcas (spec trash-bin, "Marcar elementos en la papelera") -------------
  const [marks, setMarks] = useState<Set<string>>(() => new Set());
  const [anchor, setAnchor] = useState<string | null>(null);
  const [batchBusy, setBatchBusy] = useState<string | null>(null);
  const [batchPurgeOpen, setBatchPurgeOpen] = useState(false);
  const [batchError, setBatchError] = useState<string | null>(null);
  const [report, setReport] = useState<BatchFailure[] | null>(null);
  const clearMarks = useCallback(() => {
    setMarks(new Set());
    setAnchor(null);
  }, []);

  // Clave de cada elemento y orden en que se ve, por disco (los rangos no cruzan discos).
  const { orderByDisk, itemByKey } = useMemo(() => {
    const orderByDisk = new Map<string, string[]>();
    const itemByKey = new Map<string, { ref: TrashItemRef; disk: string; kind: 'project' | 'photo' }>();
    for (const disk of disks ?? []) {
      const order: string[] = [];
      for (const p of disk.projects) {
        const key = projectKey(disk, p);
        order.push(key);
        itemByKey.set(key, { ref: { kind: 'project', device_uuid: disk.device_uuid, rel_path: p.rel_path }, disk: disk.device_uuid, kind: 'project' });
      }
      for (const item of disk.photos) {
        const key = photoKey(item);
        order.push(key);
        itemByKey.set(key, { ref: { kind: 'photo', project_id: item.project_id, photo_id: item.photo.id }, disk: disk.device_uuid, kind: 'photo' });
      }
      orderByDisk.set(disk.device_uuid, order);
    }
    return { orderByDisk, itemByKey };
  }, [disks]);

  // Lo que ya no está en la papelera deja de estar marcado.
  useEffect(() => {
    setMarks((m) => {
      const kept = new Set([...m].filter((k) => itemByKey.has(k)));
      return kept.size === m.size ? m : kept;
    });
  }, [itemByKey]);

  const marksRef = useRef(marks);
  marksRef.current = marks;
  useEffect(() => {
    if (batchPurgeOpen || purging || emptying || collision) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && marksRef.current.size > 0) {
        e.preventDefault();
        clearMarks();
      }
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [batchPurgeOpen, purging, emptying, collision, clearMarks]);

  const toggleOne = (key: string) => {
    setMarks((m) => toggleMark(m, key));
    setAnchor(key);
  };

  const handleCardClick = (disk: DiskTrash, key: string, e: React.MouseEvent | React.KeyboardEvent) => {
    const intent = clickIntent(e, hasPlatformModifier(e), marks.size > 0);
    if (intent === 'range') {
      setMarks((m) => markRange(m, orderByDisk.get(disk.device_uuid) ?? [], anchor, key));
      setAnchor(key);
    } else if (intent === 'toggle') {
      toggleOne(key);
    }
    // Sin marcas, un clic en la papelera no abre nada: las acciones están en el menú.
  };

  const markedRefs = () => [...marks].map((k) => itemByKey.get(k)).filter((x): x is NonNullable<typeof x> => !!x);
  const markedCounts = () => {
    const refs = markedRefs();
    return { projects: refs.filter((r) => r.kind === 'project').length, photos: refs.filter((r) => r.kind === 'photo').length };
  };

  const restoreMarked = async () => {
    setActionError(null);
    setBatchBusy(t('trash.marked.restoring'));
    try {
      const out = await api.restoreTrashItems(markedRefs().map((r) => r.ref));
      clearMarks();
      if (out.failed.length > 0) setReport(out.failed);
    } catch (err) {
      setActionError(describe(err));
    } finally {
      setBatchBusy(null);
    }
  };

  const purgeMarked = async () => {
    setBatchBusy(t('trash.marked.purging'));
    setBatchError(null);
    try {
      const out = await api.purgeTrashItems(markedRefs().map((r) => r.ref));
      clearMarks();
      setBatchPurgeOpen(false);
      if (out.failed.length > 0) setReport(out.failed);
    } catch (err) {
      setBatchError(describe(err));
    } finally {
      setBatchBusy(null);
    }
  };

  const describe = useCallback(
    (err: unknown) => {
      const holder = api.projectLockHolder(err);
      return holder !== null ? t('trash.locked', { device: holder }) : t('trash.actionError', { error: String(err) });
    },
    [t],
  );

  // ---- Restaurar ---------------------------------------------------------------

  const restoreProject = async (disk: DiskTrash, project: TrashedProject) => {
    setActionError(null);
    try {
      await api.restoreProject(disk.device_uuid, project.rel_path);
    } catch (err) {
      const occupied = api.restoreCollision(err);
      if (occupied !== null) setCollision({ kind: 'project', disk, project, occupied });
      else setActionError(describe(err));
    }
  };

  const restorePhoto = async (item: TrashedPhoto) => {
    setActionError(null);
    try {
      await api.restorePhoto(item.project_id, item.photo.id);
    } catch (err) {
      const occupied = api.restoreCollision(err);
      if (occupied !== null) setCollision({ kind: 'photo', item, occupied });
      else setActionError(describe(err));
    }
  };

  const checkCollision = useCallback(
    (value: string) => {
      if (!collision) return Promise.resolve({ free: false, target: '' });
      return collision.kind === 'project'
        ? api.checkRestoreTarget(collision.disk.device_uuid, collision.project.rel_path, value)
        : api.checkPhotoRestore(collision.item.project_id, collision.item.photo.id, value);
    },
    [collision],
  );

  const confirmCollision = async (value: string) => {
    if (!collision) return;
    setBusy(true);
    setCollisionError(null);
    try {
      if (collision.kind === 'project') {
        await api.restoreProject(collision.disk.device_uuid, collision.project.rel_path, value);
      } else {
        await api.restorePhoto(collision.item.project_id, collision.item.photo.id, value);
      }
      setCollision(null);
    } catch (err) {
      const occupied = api.restoreCollision(err);
      setCollisionError(occupied !== null ? t('trash.collision.taken', { target: occupied }) : describe(err));
    } finally {
      setBusy(false);
    }
  };

  // ---- Eliminar definitivamente ------------------------------------------------

  const confirmPurge = async () => {
    if (!purging) return;
    setBusy(true);
    setPurgeError(null);
    try {
      if (purging.kind === 'project') await api.purgeProject(purging.disk.device_uuid, purging.project.rel_path);
      else await api.purgePhoto(purging.item.project_id, purging.item.photo.id);
      setPurging(null);
    } catch (err) {
      setPurgeError(describe(err));
      void refresh();
    } finally {
      setBusy(false);
    }
  };

  const confirmEmpty = async () => {
    if (!emptying) return;
    setBusy(true);
    setEmptyError(null);
    try {
      const report = await api.emptyTrash(emptying.device_uuid);
      if (report.failed.length > 0) setEmptyReport(report);
      else setEmptying(null);
    } catch (err) {
      setEmptyError(describe(err));
    } finally {
      setBusy(false);
    }
  };

  // Al vaciar con fallos, el modal se queda abierto mostrando qué falló y lo que sigue en el disco.
  const emptyingNow = emptying ? disks?.find((d) => d.device_uuid === emptying.device_uuid) : undefined;

  return (
    <div style={pageStyles}>
      <div>
        <h1 style={titleStyles}>{t('trash.title')}</h1>
        <p style={{ ...mutedStyles, marginTop: '6px' }}>{t('trash.subtitle')}</p>
      </div>
      {loadError && <p style={errorStyles}>{loadError}</p>}
      {actionError && <p style={errorStyles}>{actionError}</p>}
      {disks === null && <p style={mutedStyles}>{t('common.loading')}</p>}

      <div style={scrollStyles}>
        {disks?.map((disk) => (
          <section key={disk.device_uuid}>
            <div style={diskHeaderStyles}>
              <h2 style={diskTitleStyles}>
                <Icon name="drive" size="sm" aria-hidden="true" />
                {disk.device_name}
              </h2>
              <Button
                variant="danger"
                size="sm"
                onClick={() => {
                  setEmptying(disk);
                  setEmptyError(null);
                  setEmptyReport(null);
                }}
              >
                {t('trash.emptyDisk', { disk: disk.device_name })}
              </Button>
            </div>

            {disk.projects.length > 0 && (
              <>
                <h3 style={subTitleStyles}>{t('trash.projects', { count: disk.projects.length })}</h3>
                <div style={projectGridStyles}>
                  {disk.projects.map((project) => (
                    <TrashedProjectCard
                      key={project.rel_path}
                      disk={disk}
                      project={project}
                      marked={marks.has(projectKey(disk, project))}
                      onToggleMark={() => toggleOne(projectKey(disk, project))}
                      onClick={(e) => handleCardClick(disk, projectKey(disk, project), e)}
                      onRestore={() => void restoreProject(disk, project)}
                      onPurge={() => {
                        setPurging({ kind: 'project', disk, project });
                        setPurgeError(null);
                      }}
                    />
                  ))}
                </div>
              </>
            )}

            {disk.photos.length > 0 && (
              <>
                <h3 style={subTitleStyles}>{t('trash.photos', { count: disk.photos.length })}</h3>
                <div style={photoGridStyles}>
                  {disk.photos.map((item) => (
                    <TrashedPhotoCard
                      key={item.photo.id}
                      item={item}
                      marked={marks.has(photoKey(item))}
                      onToggleMark={() => toggleOne(photoKey(item))}
                      onClick={(e) => handleCardClick(disk, photoKey(item), e)}
                      onRestore={() => void restorePhoto(item)}
                      onPurge={() => {
                        setPurging({ kind: 'photo', item });
                        setPurgeError(null);
                      }}
                    />
                  ))}
                </div>
              </>
            )}
          </section>
        ))}
      </div>

      {marks.size > 0 && (
        <MarkedBar
          count={marks.size}
          noun="items"
          busy={batchBusy}
          onClear={clearMarks}
          actions={[
            { icon: 'archive-restore', label: t('trash.marked.restore'), onClick: () => void restoreMarked() },
            {
              icon: 'trash',
              label: t('trash.purge.confirm'),
              danger: true,
              onClick: () => {
                setBatchError(null);
                setBatchPurgeOpen(true);
              },
            },
          ]}
        />
      )}

      <BatchPurgeModal
        open={batchPurgeOpen}
        {...markedCounts()}
        onClose={() => setBatchPurgeOpen(false)}
        onConfirm={() => void purgeMarked()}
        loading={batchBusy !== null}
        error={batchError}
      />

      <BatchReportModal failed={report} onClose={() => setReport(null)} />

      <PurgeModal
        open={purging !== null}
        target={
          purging?.kind === 'project'
            ? { kind: 'project', name: purging.project.name }
            : purging
              ? { kind: 'photo', name: fileNameOf(purging.item.photo.dng_path), project: purging.item.project_name }
              : null
        }
        onClose={() => setPurging(null)}
        onConfirm={() => void confirmPurge()}
        loading={busy}
        error={purgeError}
      />

      <EmptyTrashModal
        open={emptying !== null}
        disk={emptying?.device_name ?? ''}
        projects={emptyingNow?.projects.length ?? 0}
        photos={emptyingNow?.photos.length ?? 0}
        onClose={() => {
          setEmptying(null);
          setEmptyReport(null);
        }}
        onConfirm={() => void confirmEmpty()}
        loading={busy}
        error={emptyError}
        failed={emptyReport?.failed}
      />

      <RestoreCollisionModal
        open={collision !== null}
        kind={collision?.kind ?? 'project'}
        name={
          collision?.kind === 'project'
            ? collision.project.name
            : collision
              ? fileNameOf(collision.item.photo.dng_path)
              : ''
        }
        occupied={collision?.occupied ?? ''}
        initialValue={
          collision?.kind === 'project'
            ? `${collision.project.name} 2`
            : collision
              ? `${stemOf(fileNameOf(collision.item.photo.dng_path))}_2`
              : ''
        }
        check={checkCollision}
        onClose={() => {
          setCollision(null);
          setCollisionError(null);
        }}
        onConfirm={(value) => void confirmCollision(value)}
        loading={busy}
        error={collisionError}
      />
    </div>
  );
}
