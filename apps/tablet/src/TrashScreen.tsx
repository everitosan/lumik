// Sección "Papelera" de la tablet (spec trash-bin), como la del desktop: lo que
// hay en la papelera de cada disco conectado, agrupado por disco. Cada elemento
// se restaura o se elimina definitivamente, y cada disco se puede vaciar.
import { Image } from 'expo-image';
import { useEffect, useRef, useState } from 'react';
import { Modal, Pressable, ScrollView, StyleSheet, Text, TextInput, View } from 'react-native';

import { Icon } from './Icon';
import { lockMessage } from './lock';
import { MoveSheet } from './MoveSheet';
import { ProjectLockedError, RestoreCollisionError } from './projectActions';
import type { DiskTrash, TrashedPhotoItem, TrashedProjectItem } from './trash';

export type TrashActions = {
  restoreProject: (item: TrashedProjectItem, newName?: string) => Promise<void>;
  checkProject: (item: TrashedProjectItem, newName: string) => Promise<{ free: boolean; target: string }>;
  purgeProject: (item: TrashedProjectItem) => Promise<void>;
  restorePhoto: (item: TrashedPhotoItem, newStem: string | undefined, force: boolean) => Promise<void>;
  checkPhoto: (item: TrashedPhotoItem, newStem: string) => Promise<{ free: boolean; target: string }>;
  purgePhoto: (item: TrashedPhotoItem, force: boolean) => Promise<void>;
  /** Lotes de lo marcado (spec trash-bin): devuelven lo que no se pudo, con su
   *  motivo (`collision` si el lugar está ocupado). */
  restoreMany: (projects: TrashedProjectItem[], photos: TrashedPhotoItem[]) => Promise<{ name: string; reason: string }[]>;
  purgeMany: (projects: TrashedProjectItem[], photos: TrashedPhotoItem[]) => Promise<{ name: string; reason: string }[]>;
  /** Elimina todo lo del disco; devuelve lo que no se pudo eliminar. */
  emptyDisk: (disk: DiskTrash) => Promise<{ name: string; reason: string }[]>;
};

type Props = {
  disks: DiskTrash[] | null;
  labelOf: (tree: string) => string;
  busy: string | null;
  actions: TrashActions;
};

/** "2026-11-10T19:54:03" → "10 nov 2026" */
function shortDate(raw?: string): string {
  if (!raw) return '';
  const d = new Date(raw.length === 10 ? `${raw}T00:00:00` : raw);
  if (isNaN(d.getTime())) return raw;
  return d.toLocaleDateString('es-MX', { day: 'numeric', month: 'short', year: 'numeric' });
}

const stemOf = (name: string) => (name.lastIndexOf('.') > 0 ? name.slice(0, name.lastIndexOf('.')) : name);
const plural = (n: number, one: string, other: string) => `${n} ${n === 1 ? one : other}`;

type Purge = { kind: 'project'; item: TrashedProjectItem } | { kind: 'photo'; item: TrashedPhotoItem };
type Collision = { kind: 'project'; item: TrashedProjectItem; occupied: string } | { kind: 'photo'; item: TrashedPhotoItem; occupied: string };

export function TrashScreen({ disks, labelOf, busy, actions }: Props) {
  const [error, setError] = useState<string | null>(null);
  const [purging, setPurging] = useState<Purge | null>(null);
  const [emptying, setEmptying] = useState<DiskTrash | null>(null);
  const [collision, setCollision] = useState<Collision | null>(null);
  // Foto de un proyecto que tiene otro dispositivo: se ofrece tomar el control.
  const [locked, setLocked] = useState<{ message: string; retry: () => Promise<void> } | null>(null);

  // Modo marcar (spec trash-bin): `null` fuera del modo; si no, las claves marcadas.
  const [marked, setMarked] = useState<Set<string> | null>(null);
  const [confirmPurgeMany, setConfirmPurgeMany] = useState(false);
  const [manyBusy, setManyBusy] = useState<string | null>(null);
  const [manyFailed, setManyFailed] = useState<{ name: string; reason: string }[] | null>(null);
  const projectKey = (item: TrashedProjectItem) => `project:${item.project.uri}`;
  const photoKey = (item: TrashedPhotoItem) => `photo:${item.project.uri}/${item.info.id}`;
  const toggle = (key: string) =>
    setMarked((m) => {
      const next = new Set(m ?? []);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  const pressCard = (key: string, open: () => void) => (marked ? toggle(key) : open());
  const longPressCard = (key: string) => setMarked((m) => m ?? new Set([key]));
  const markedItems = () => {
    const projects: TrashedProjectItem[] = [];
    const photos: TrashedPhotoItem[] = [];
    for (const d of disks ?? []) {
      for (const p of d.projects) if (marked?.has(projectKey(p))) projects.push(p);
      for (const p of d.photos) if (marked?.has(photoKey(p))) photos.push(p);
    }
    return { projects, photos };
  };
  // Lo que ya no está en la papelera deja de estar marcado.
  useEffect(() => {
    setMarked((m) => {
      if (!m) return m;
      const present = new Set((disks ?? []).flatMap((d) => [...d.projects.map(projectKey), ...d.photos.map(photoKey)]));
      const kept = new Set([...m].filter((k) => present.has(k)));
      return kept.size === m.size ? m : kept.size > 0 ? kept : null;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [disks]);

  const runMany = async (label: string, fn: () => Promise<{ name: string; reason: string }[]>) => {
    setManyBusy(label);
    try {
      const failed = await fn();
      setMarked(null);
      if (failed.length > 0) setManyFailed(failed);
    } catch (e) {
      setError(String(e));
    } finally {
      setManyBusy(null);
    }
  };

  const explain = (e: unknown) => (e instanceof ProjectLockedError ? lockMessage(e.lock) : String(e));

  // Restaurar un proyecto usa la misma hoja animada que mandarlo a la papelera.
  const [moving, setMoving] = useState<{ name: string; run: () => Promise<boolean> } | null>(null);
  const projectName = (item: TrashedProjectItem) => item.card?.name ?? item.project.name.split('/').pop() ?? item.project.name;

  const restoreProject = (item: TrashedProjectItem) => {
    setError(null);
    setMoving({
      name: projectName(item),
      run: async () => {
        try {
          await actions.restoreProject(item);
          return true;
        } catch (e) {
          if (!(e instanceof RestoreCollisionError)) throw e;
          setCollision({ kind: 'project', item, occupied: e.target });
          return false;
        }
      },
    });
  };

  const restorePhoto = async (item: TrashedPhotoItem, force = false) => {
    setError(null);
    try {
      await actions.restorePhoto(item, undefined, force);
    } catch (e) {
      if (e instanceof RestoreCollisionError) setCollision({ kind: 'photo', item, occupied: e.target });
      else if (e instanceof ProjectLockedError) setLocked({ message: lockMessage(e.lock), retry: () => restorePhoto(item, true) });
      else setError(explain(e));
    }
  };

  return (
    <>
      <View style={styles.header}>
        <View style={{ flex: 1 }}>
          <Text style={styles.title}>Papelera</Text>
          <Text style={styles.muted}>Lo que mandaste a la papelera en los discos conectados. Nada se borra hasta que lo elimines definitivamente.</Text>
        </View>
      </View>
      <ScrollView contentContainerStyle={styles.content}>
        {error && <Text style={styles.error}>{error}</Text>}
        {busy && <Text style={styles.busy}>{busy}…</Text>}
        {disks === null && <Text style={styles.muted}>Buscando…</Text>}
        {disks?.map((disk) => (
          <View key={disk.tree} style={styles.group}>
            <View style={styles.groupHeader}>
              <View style={styles.withIcon}>
                <Icon name="drive" size={18} color="#9a9a9a" />
                <Text style={styles.groupTitle}>{labelOf(disk.tree)}</Text>
              </View>
              <Pressable style={({ pressed }) => [styles.dangerButton, pressed && styles.pressed]} onPress={() => setEmptying(disk)} disabled={!!busy}>
                <Text style={styles.dangerButtonText}>Vaciar papelera de {labelOf(disk.tree)}</Text>
              </Pressable>
            </View>

            {disk.projects.length > 0 && <Text style={styles.subTitle}>PROYECTOS ({disk.projects.length})</Text>}
            <View style={styles.cards}>
              {disk.projects.map((item) => (
                <Pressable
                  key={item.project.uri}
                  style={[styles.card, marked?.has(projectKey(item)) && styles.cardMarked]}
                  onPress={() => pressCard(projectKey(item), () => undefined)}
                  onLongPress={() => longPressCard(projectKey(item))}
                >
                  <View style={styles.cardArt}>
                    {item.cover ? (
                      <Image source={{ uri: item.cover }} style={styles.cover} contentFit="cover" />
                    ) : (
                      <Icon name="image" size={40} color="#444" />
                    )}
                    {item.trashedAt && (
                      <View style={styles.badge}>
                        <Text style={styles.badgeText}>En la papelera desde {shortDate(item.trashedAt)}</Text>
                      </View>
                    )}
                  </View>
                  <Text style={styles.cardTitle} numberOfLines={1}>
                    {item.card?.name ?? item.project.name.split('/').pop()}
                  </Text>
                  <Text style={styles.muted}>
                    {shortDate(item.card?.sessionDate ?? item.card?.createdAt)} · {plural(item.card?.photoCount ?? 0, 'foto', 'fotos')}
                  </Text>
                  <View style={styles.cardButtons}>
                    <Pressable style={({ pressed }) => [styles.button, pressed && styles.pressed]} onPress={() => restoreProject(item)} disabled={!!busy}>
                      <Text style={styles.buttonText}>Restaurar</Text>
                    </Pressable>
                    <Pressable style={({ pressed }) => [styles.button, pressed && styles.pressed]} onPress={() => setPurging({ kind: 'project', item })} disabled={!!busy}>
                      <Text style={styles.dangerText}>Eliminar</Text>
                    </Pressable>
                  </View>
                  {marked && <MarkBox on={marked.has(projectKey(item))} />}
                </Pressable>
              ))}
            </View>

            {disk.photos.length > 0 && <Text style={styles.subTitle}>FOTOS ({disk.photos.length})</Text>}
            <View style={styles.cards}>
              {disk.photos.map((item) => (
                <Pressable
                  key={`${item.project.uri}/${item.info.id}`}
                  style={[styles.photoCard, marked?.has(photoKey(item)) && styles.cardMarked]}
                  onPress={() => pressCard(photoKey(item), () => undefined)}
                  onLongPress={() => longPressCard(photoKey(item))}
                >
                  <View style={styles.photoArt}>
                    <Image source={{ uri: item.thumb }} style={styles.cover} contentFit="cover" />
                  </View>
                  <Text style={styles.cardTitle} numberOfLines={1}>
                    {item.info.fileName}
                  </Text>
                  <Text style={styles.muted} numberOfLines={1}>
                    De «{item.projectName}»
                  </Text>
                  <View style={styles.cardButtons}>
                    <Pressable style={({ pressed }) => [styles.button, pressed && styles.pressed]} onPress={() => void restorePhoto(item)} disabled={!!busy}>
                      <Text style={styles.buttonText}>Restaurar</Text>
                    </Pressable>
                    <Pressable style={({ pressed }) => [styles.button, pressed && styles.pressed]} onPress={() => setPurging({ kind: 'photo', item })} disabled={!!busy}>
                      <Text style={styles.dangerText}>Eliminar</Text>
                    </Pressable>
                  </View>
                  {marked && <MarkBox on={marked.has(photoKey(item))} />}
                </Pressable>
              ))}
            </View>
          </View>
        ))}
      </ScrollView>

      {marked && (
        <View style={styles.markedBar}>
          <Text style={styles.markedCount}>{marked.size === 1 ? '1 elemento marcado' : `${marked.size} elementos marcados`}</Text>
          {manyBusy && <Text style={styles.busy}>{manyBusy}…</Text>}
          <View style={{ flex: 1 }} />
          <Pressable
            style={({ pressed }) => [styles.button, pressed && styles.pressed, (marked.size === 0 || !!manyBusy) && styles.off]}
            disabled={marked.size === 0 || !!manyBusy}
            onPress={() => {
              const { projects, photos } = markedItems();
              const restore = () => runMany('Restaurando', () => actions.restoreMany(projects, photos));
              // Con proyectos puede tardar: la misma hoja animada. Las fotos sueltas son rápidas.
              if (projects.length > 0) {
                const total = projects.length + photos.length;
                setMoving({ name: total === 1 ? projectName(projects[0]) : plural(total, 'elemento', 'elementos'), run: () => restore().then(() => true) });
              } else void restore();
            }}
          >
            <Text style={styles.buttonText}>Restaurar</Text>
          </Pressable>
          <Pressable
            style={({ pressed }) => [styles.dangerButton, pressed && styles.pressed, (marked.size === 0 || !!manyBusy) && styles.off]}
            disabled={marked.size === 0 || !!manyBusy}
            onPress={() => setConfirmPurgeMany(true)}
          >
            <Text style={styles.dangerButtonText}>Eliminar definitivamente</Text>
          </Pressable>
          <Pressable style={({ pressed }) => [styles.button, pressed && styles.pressed]} onPress={() => setMarked(null)} disabled={!!manyBusy}>
            <Text style={styles.buttonText}>Listo</Text>
          </Pressable>
        </View>
      )}

      {confirmPurgeMany && marked && (
        <Modal transparent animationType="fade" onRequestClose={() => setConfirmPurgeMany(false)}>
          <View style={styles.scrim}>
            <View style={styles.sheet}>
              <Text style={styles.sheetTitle}>Eliminar definitivamente</Text>
              <Text style={styles.sheetText}>
                Se eliminarán para siempre {plural(markedItems().projects.length, 'proyecto', 'proyectos')} y{' '}
                {plural(markedItems().photos.length, 'foto', 'fotos')} marcados.
              </Text>
              <Text style={styles.warning}>Esta acción no se puede deshacer.</Text>
              <Text style={styles.muted}>{BACKUP_NOTE}</Text>
              <View style={styles.sheetButtons}>
                <Pressable style={styles.textButton} onPress={() => setConfirmPurgeMany(false)}>
                  <Text style={styles.textButtonText}>Cancelar</Text>
                </Pressable>
                <Pressable
                  style={({ pressed }) => [styles.dangerButton, pressed && styles.pressed]}
                  onPress={() => {
                    setConfirmPurgeMany(false);
                    const { projects, photos } = markedItems();
                    void runMany('Eliminando', () => actions.purgeMany(projects, photos));
                  }}
                >
                  <Text style={styles.dangerButtonText}>Eliminar definitivamente</Text>
                </Pressable>
              </View>
            </View>
          </View>
        </Modal>
      )}
      {manyFailed && (
        <Modal transparent animationType="fade" onRequestClose={() => setManyFailed(null)}>
          <View style={styles.scrim}>
            <View style={styles.sheet}>
              <Text style={styles.sheetTitle}>Algunos elementos siguen en la papelera</Text>
              <Text style={styles.error}>{manyFailed.length === 1 ? 'No se pudo con 1 elemento:' : `No se pudo con ${manyFailed.length} elementos:`}</Text>
              {manyFailed.map((f, i) => (
                <Text key={i} style={styles.muted}>
                  {f.name}: {f.reason === 'collision' ? 'su lugar está ocupado' : f.reason}
                </Text>
              ))}
              <Text style={styles.muted}>Los que tienen el lugar ocupado se pueden restaurar uno por uno con otro nombre.</Text>
              <View style={styles.sheetButtons}>
                <Pressable style={({ pressed }) => [styles.primary, pressed && styles.pressed]} onPress={() => setManyFailed(null)}>
                  <Text style={styles.primaryText}>Entendido</Text>
                </Pressable>
              </View>
            </View>
          </View>
        </Modal>
      )}
      {purging && (
        <PurgeSheet
          purge={purging}
          onClose={() => setPurging(null)}
          onConfirm={async (force) => {
            if (purging.kind === 'project') await actions.purgeProject(purging.item);
            else await actions.purgePhoto(purging.item, force);
          }}
        />
      )}
      {emptying && (
        <EmptySheet
          disk={emptying}
          label={labelOf(emptying.tree)}
          onClose={() => setEmptying(null)}
          onConfirm={() => actions.emptyDisk(emptying)}
        />
      )}
      {collision && (
        <CollisionSheet
          key={collision.kind === 'project' ? collision.item.project.uri : collision.item.info.id}
          collision={collision}
          check={(value) => (collision.kind === 'project' ? actions.checkProject(collision.item, value) : actions.checkPhoto(collision.item, value))}
          onClose={() => setCollision(null)}
          onConfirm={async (value) => {
            if (collision.kind === 'photo') return actions.restorePhoto(collision.item, value, false);
            const item = collision.item;
            setCollision(null);
            setMoving({ name: value, run: () => actions.restoreProject(item, value).then(() => true) });
          }}
        />
      )}
      {moving && <MoveSheet direction="restore" name={moving.name} run={moving.run} onClose={() => setMoving(null)} />}
      {locked && (
        <Modal transparent animationType="fade" onRequestClose={() => setLocked(null)}>
          <View style={styles.scrim}>
            <View style={styles.sheet}>
              <Text style={styles.sheetTitle}>Proyecto en uso</Text>
              <Text style={styles.sheetText}>{locked.message}</Text>
              <View style={styles.sheetButtons}>
                <Pressable style={styles.textButton} onPress={() => setLocked(null)}>
                  <Text style={styles.textButtonText}>Cancelar</Text>
                </Pressable>
                <Pressable
                  style={({ pressed }) => [styles.primary, pressed && styles.pressed]}
                  onPress={async () => {
                    const retry = locked.retry;
                    setLocked(null);
                    await retry();
                  }}
                >
                  <Text style={styles.primaryText}>Tomar el control</Text>
                </Pressable>
              </View>
            </View>
          </View>
        </Modal>
      )}
    </>
  );
}

function MarkBox({ on }: { on: boolean }) {
  return (
    <View style={[styles.markBox, on && styles.markBoxOn]} pointerEvents="none">
      {on && <Icon name="check" size={16} color="#002d6f" />}
    </View>
  );
}

const BACKUP_NOTE = 'El respaldo en la nube no se toca. Si también quieres borrarlo, hazlo desde el portal web.';

function PurgeSheet({ purge, onClose, onConfirm }: { purge: Purge; onClose: () => void; onConfirm: (force: boolean) => Promise<void> }) {
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lock, setLock] = useState<string | null>(null);
  const run = async (force: boolean) => {
    setWorking(true);
    setError(null);
    try {
      await onConfirm(force);
      onClose();
    } catch (e) {
      if (e instanceof ProjectLockedError) setLock(lockMessage(e.lock));
      else setError(String(e));
    } finally {
      setWorking(false);
    }
  };
  return (
    <Modal transparent animationType="fade" onRequestClose={onClose}>
      <View style={styles.scrim}>
        <View style={styles.sheet}>
          <Text style={styles.sheetTitle}>Eliminar definitivamente</Text>
          <Text style={styles.sheetText}>
            {purge.kind === 'project'
              ? `Se borrará para siempre el proyecto «${purge.item.card?.name ?? purge.item.project.name}» con todas sus fotos y archivos.`
              : `Se borrará para siempre «${purge.item.info.fileName}» del proyecto «${purge.item.projectName}».`}
          </Text>
          <Text style={styles.warning}>Esta acción no se puede deshacer.</Text>
          <Text style={styles.muted}>{BACKUP_NOTE}</Text>
          {lock && <Text style={styles.error}>{lock}</Text>}
          {error && <Text style={styles.error}>{error}</Text>}
          <View style={styles.sheetButtons}>
            <Pressable style={styles.textButton} onPress={onClose} disabled={working}>
              <Text style={styles.textButtonText}>Cancelar</Text>
            </Pressable>
            <Pressable style={({ pressed }) => [styles.dangerButton, pressed && styles.pressed, working && styles.off]} disabled={working} onPress={() => void run(!!lock)}>
              <Text style={styles.dangerButtonText}>{working ? 'Eliminando…' : lock ? 'Tomar el control y eliminar' : 'Eliminar definitivamente'}</Text>
            </Pressable>
          </View>
        </View>
      </View>
    </Modal>
  );
}

function EmptySheet({
  disk,
  label,
  onClose,
  onConfirm,
}: {
  disk: DiskTrash;
  label: string;
  onClose: () => void;
  onConfirm: () => Promise<{ name: string; reason: string }[]>;
}) {
  const [working, setWorking] = useState(false);
  const [failed, setFailed] = useState<{ name: string; reason: string }[] | null>(null);
  return (
    <Modal transparent animationType="fade" onRequestClose={onClose}>
      <View style={styles.scrim}>
        <View style={styles.sheet}>
          <Text style={styles.sheetTitle}>Vaciar papelera de {label}</Text>
          {failed ? (
            <>
              <Text style={styles.error}>{failed.length === 1 ? 'No se pudo eliminar 1 elemento:' : `No se pudieron eliminar ${failed.length} elementos:`}</Text>
              {failed.map((f, i) => (
                <Text key={i} style={styles.muted}>
                  {f.name}: {f.reason}
                </Text>
              ))}
            </>
          ) : (
            <>
              <Text style={styles.sheetText}>
                Se eliminarán para siempre {plural(disk.projects.length, 'proyecto', 'proyectos')} y {plural(disk.photos.length, 'foto', 'fotos')} de la papelera de {label}.
              </Text>
              <Text style={styles.warning}>Esta acción no se puede deshacer.</Text>
              <Text style={styles.muted}>{BACKUP_NOTE}</Text>
            </>
          )}
          <View style={styles.sheetButtons}>
            <Pressable style={styles.textButton} onPress={onClose} disabled={working}>
              <Text style={styles.textButtonText}>{failed ? 'Cerrar' : 'Cancelar'}</Text>
            </Pressable>
            {!failed && (
              <Pressable
                style={({ pressed }) => [styles.dangerButton, pressed && styles.pressed, working && styles.off]}
                disabled={working}
                onPress={async () => {
                  setWorking(true);
                  const out = await onConfirm();
                  setWorking(false);
                  if (out.length > 0) setFailed(out);
                  else onClose();
                }}
              >
                <Text style={styles.dangerButtonText}>{working ? 'Vaciando…' : 'Vaciar papelera'}</Text>
              </Pressable>
            )}
          </View>
        </View>
      </View>
    </Modal>
  );
}

function CollisionSheet({
  collision,
  check,
  onClose,
  onConfirm,
}: {
  collision: Collision;
  check: (value: string) => Promise<{ free: boolean; target: string }>;
  onClose: () => void;
  onConfirm: (value: string) => Promise<void>;
}) {
  const name = collision.kind === 'project' ? (collision.item.card?.name ?? collision.item.project.name) : collision.item.info.fileName;
  const [value, setValue] = useState(collision.kind === 'project' ? `${name} 2` : `${stemOf(name)}_2`);
  const [status, setStatus] = useState<{ value: string; free: boolean; target: string } | null>(null);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const trimmed = value.trim();
  // `check` llega nuevo en cada render: se lee de un ref para no relanzar la validación.
  const checkRef = useRef(check);
  checkRef.current = check;

  useEffect(() => {
    if (!trimmed) return;
    let cancelled = false;
    const timer = setTimeout(() => {
      checkRef.current(trimmed)
        .then((c) => !cancelled && setStatus({ value: trimmed, ...c }))
        .catch((e) => !cancelled && setError(String(e)));
    }, 250);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [trimmed]);

  const current = status && status.value === trimmed ? status : null;
  const canConfirm = !!trimmed && current?.free === true && !working;

  return (
    <Modal transparent animationType="fade" onRequestClose={onClose}>
      <View style={styles.scrim}>
        <View style={styles.sheet}>
          <Text style={styles.sheetTitle}>Ya hay algo en ese lugar</Text>
          <Text style={styles.sheetText}>
            {collision.kind === 'project'
              ? `Ya existe la carpeta «${collision.occupied}» donde volvería «${name}». Elige otro nombre para restaurarlo.`
              : `Ya existe «${collision.occupied}» donde volvería «${name}». Elige otro nombre de archivo para restaurarla.`}
          </Text>
          <Text style={styles.label}>{collision.kind === 'project' ? 'Nombre del proyecto' : 'Nombre del archivo (sin extensión)'}</Text>
          <TextInput style={styles.input} value={value} onChangeText={setValue} autoFocus editable={!working} />
          {!trimmed && <Text style={styles.muted}>Escribe un nombre.</Text>}
          {current && !current.free && <Text style={styles.error}>«{current.target}» también está ocupado.</Text>}
          {error && <Text style={styles.error}>{error}</Text>}
          <View style={styles.sheetButtons}>
            <Pressable style={styles.textButton} onPress={onClose} disabled={working}>
              <Text style={styles.textButtonText}>Cancelar</Text>
            </Pressable>
            <Pressable
              style={({ pressed }) => [styles.primary, pressed && styles.pressed, !canConfirm && styles.off]}
              disabled={!canConfirm}
              onPress={async () => {
                setWorking(true);
                setError(null);
                try {
                  await onConfirm(trimmed);
                  onClose();
                } catch (e) {
                  setError(e instanceof RestoreCollisionError ? `«${e.target}» también está ocupado.` : String(e));
                } finally {
                  setWorking(false);
                }
              }}
            >
              <Text style={styles.primaryText}>{working ? 'Restaurando…' : 'Restaurar'}</Text>
            </Pressable>
          </View>
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  header: { flexDirection: 'row', alignItems: 'center', paddingHorizontal: 24, paddingVertical: 16, gap: 16 },
  title: { color: '#fff', fontSize: 28, fontWeight: '600' },
  content: { paddingHorizontal: 24, paddingBottom: 48, gap: 28 },
  muted: { color: '#8c8c8c', fontSize: 14 },
  error: { color: '#ffb4ab', fontSize: 14 },
  busy: { color: '#e9c349', fontSize: 15 },
  warning: { color: '#ffb4ab', fontSize: 14, fontWeight: '600' },
  group: { gap: 12 },
  groupHeader: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', gap: 12 },
  groupTitle: { color: '#fff', fontSize: 18, fontWeight: '600' },
  subTitle: { color: '#9a9a9a', fontSize: 12, fontWeight: '600', letterSpacing: 0.8 },
  withIcon: { flexDirection: 'row', alignItems: 'center', gap: 8 },
  cards: { flexDirection: 'row', flexWrap: 'wrap', gap: 16 },
  card: { width: 220, gap: 4 },
  photoCard: { width: 180, gap: 4 },
  cardMarked: { borderWidth: 2, borderColor: '#b0c6ff', borderRadius: 14, padding: 4, margin: -6 },
  markBox: {
    position: 'absolute',
    top: 8,
    left: 8,
    width: 26,
    height: 26,
    borderRadius: 7,
    borderWidth: 2,
    borderColor: 'rgba(255,255,255,0.85)',
    backgroundColor: 'rgba(0,0,0,0.45)',
    alignItems: 'center',
    justifyContent: 'center',
  },
  markBoxOn: { backgroundColor: '#b0c6ff', borderColor: '#b0c6ff' },
  markedBar: { flexDirection: 'row', alignItems: 'center', gap: 10, paddingHorizontal: 24, minHeight: 64, borderTopWidth: 1, borderTopColor: '#262626', backgroundColor: '#1a1a1a' },
  markedCount: { color: '#b0c6ff', fontSize: 16, fontWeight: '600' },
  cardArt: { aspectRatio: 220 / 150, borderRadius: 12, overflow: 'hidden', backgroundColor: '#1f1f1f', alignItems: 'center', justifyContent: 'center', marginBottom: 4 },
  photoArt: { aspectRatio: 3 / 2, borderRadius: 10, overflow: 'hidden', backgroundColor: '#1f1f1f', marginBottom: 4 },
  cover: { width: '100%', height: '100%' },
  badge: { position: 'absolute', top: 8, left: 8, paddingHorizontal: 8, paddingVertical: 3, borderRadius: 6, backgroundColor: 'rgba(0,0,0,0.65)' },
  badgeText: { color: '#ddd', fontSize: 11 },
  cardTitle: { color: '#fff', fontSize: 15, fontWeight: '600' },
  cardButtons: { flexDirection: 'row', gap: 8, marginTop: 6 },
  button: { minHeight: 44, paddingHorizontal: 14, justifyContent: 'center', borderRadius: 22, backgroundColor: '#2a2a2a' },
  buttonText: { color: '#ddd', fontSize: 14, fontWeight: '600' },
  // #ffb4ab sobre #2a2a2a = 8.45:1 (AAA); #f87171 daba 5.19:1.
  dangerText: { color: '#ffb4ab', fontSize: 14, fontWeight: '600' },
  dangerButton: { minHeight: 44, paddingHorizontal: 18, justifyContent: 'center', borderRadius: 22, backgroundColor: '#7f1d1d' },
  dangerButtonText: { color: '#fff', fontSize: 14, fontWeight: '600' },
  primary: { minHeight: 48, paddingHorizontal: 20, justifyContent: 'center', borderRadius: 24, backgroundColor: '#e9c349' },
  primaryText: { color: '#131313', fontSize: 15, fontWeight: '700' },
  pressed: { opacity: 0.7 },
  off: { opacity: 0.4 },
  scrim: { flex: 1, backgroundColor: 'rgba(0,0,0,0.6)', alignItems: 'center', justifyContent: 'center' },
  sheet: { width: 520, padding: 24, gap: 10, borderRadius: 16, backgroundColor: '#1d1d1d' },
  sheetTitle: { color: '#fff', fontSize: 22, fontWeight: '600', marginBottom: 4 },
  sheetText: { color: '#ddd', fontSize: 15, lineHeight: 21 },
  label: { color: '#9a9a9a', fontSize: 14, marginTop: 8 },
  input: { minHeight: 48, paddingHorizontal: 14, borderRadius: 10, backgroundColor: '#262626', color: '#fff', fontSize: 16 },
  sheetButtons: { flexDirection: 'row', justifyContent: 'flex-end', alignItems: 'center', gap: 12, marginTop: 16 },
  textButton: { minHeight: 48, paddingHorizontal: 16, justifyContent: 'center' },
  textButtonText: { color: '#9a9a9a', fontSize: 16 },
});
