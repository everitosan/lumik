// Proyectos del inicio, como el dashboard del desktop (`Projects.tsx`): por
// año, con las tarjetas (portada, nombre, fecha, fotos, estado, disco,
// archivado, quién lo tiene) y su menú "⋯" (o pulsación larga): abrir
// carpeta, editar, archivar o desarchivar y eliminar. Abajo a la derecha, "Archivados (n)" y
// "Ordenar". Un proyecto que tiene otro dispositivo pide antes tomar el control.
import { Image } from 'expo-image';
import { useMemo, useState } from 'react';
import { Modal, Pressable, ScrollView, StyleSheet, Switch, Text, TextInput, View } from 'react-native';

import type { LockView } from '@lumik/core-native';
import { type CardInfo, coverPath } from './covers';
import { lockMessage } from './lock';
import { type EditForm, ProjectLockedError } from './projectActions';
import { readPrefs, savePrefs, type SortDirection } from './prefs';
import type { DiskProject } from './session';

export type ProjectMenuActions = {
  archive: (p: DiskProject, archived: boolean, force: boolean) => Promise<void>;
  edit: (p: DiskProject, form: EditForm, force: boolean) => Promise<{ renamed: number }>;
  remove: (p: DiskProject, force: boolean) => Promise<void>;
  /** Abre la carpeta del proyecto en la app de Archivos. */
  openFolder: (p: DiskProject) => void;
};

type Props = {
  trees: string[];
  labelOf: (tree: string) => string;
  projects: DiskProject[] | null;
  busy: string | null;
  cards: Map<string, CardInfo>;
  onOpen: (p: DiskProject) => void;
  onNew: () => void;
  onChooseDisk: () => void;
  actions: ProjectMenuActions;
};

const STATUS: Record<string, string> = { imported: 'Importado', edited: 'Editado', delivered: 'Entregado' };

const compareIn = (direction: SortDirection, a: string, b: string) => (direction === 'asc' ? a.localeCompare(b) : b.localeCompare(a));

/** "2026-06-28…" → "28 jun 2026" */
function shortDate(raw?: string): string {
  if (!raw) return '';
  const d = new Date(`${raw.slice(0, 10)}T00:00:00`);
  return isNaN(d.getTime()) ? raw : d.toLocaleDateString('es-MX', { day: 'numeric', month: 'short', year: 'numeric' });
}

/** Como `sanitizeProjectName` del desktop, para la pista de "Renombrar también los archivos". */
const sanitize = (name: string) => name.replace(/[^\p{L}\p{N}_-]/gu, '_');

type Pending = { kind: 'edit' | 'archive' | 'delete'; project: DiskProject };

export function ProjectsDashboard({ trees, labelOf, projects, busy, cards, onOpen, onNew, onChooseDisk, actions }: Props) {
  const [sort, setSortState] = useState(() => readPrefs().dashboardSort ?? { years: 'desc' as SortDirection, projects: 'desc' as SortDirection });
  const [showArchived, setShowArchivedState] = useState(() => readPrefs().showArchived ?? false);
  const [showSort, setShowSort] = useState(false);
  const [menu, setMenu] = useState<DiskProject | null>(null);
  const [pending, setPending] = useState<Pending | null>(null);
  // Acción pedida sobre un proyecto de otro dispositivo: espera "Tomar el control".
  const [locked, setLocked] = useState<{ lock: LockView; retry: () => void } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const setSort = (s: typeof sort) => {
    setSortState(s);
    savePrefs({ dashboardSort: s });
  };
  const setShowArchived = (v: boolean) => {
    setShowArchivedState(v);
    savePrefs({ showArchived: v });
  };

  // Como el desktop: si el mismo proyecto (`project.id`) está en dos discos, se lista uno.
  const unique = useMemo(() => {
    const seen = new Set<string>();
    return (projects ?? []).filter((p) => {
      const id = cards.get(p.uri)?.id;
      if (!id) return true;
      if (seen.has(id)) return false;
      seen.add(id);
      return true;
    });
  }, [projects, cards]);
  const archivedCount = unique.filter((p) => cards.get(p.uri)?.card?.archived).length;
  const shown = unique.filter((p) => showArchived || !cards.get(p.uri)?.card?.archived);
  const totalPhotos = shown.reduce((n, p) => n + (cards.get(p.uri)?.card?.photoCount ?? 0), 0);

  // Por año de la sesión (o de creación), como el desktop; mientras llegan los
  // datos, el año de la carpeta.
  const dateOf = (p: DiskProject) => {
    const c = cards.get(p.uri)?.card;
    return c?.sessionDate ?? c?.createdAt ?? '';
  };
  const groups = useMemo(() => {
    const byYear = new Map<string, DiskProject[]>();
    for (const p of shown) {
      const raw = dateOf(p);
      const folderYear = p.name.split('/')[0];
      const year = raw ? raw.slice(0, 4) : /^\d{4}$/.test(folderYear) ? folderYear : 'sin-fecha';
      byYear.set(year, [...(byYear.get(year) ?? []), p]);
    }
    return [...byYear.entries()]
      .sort(([a], [b]) => (a === 'sin-fecha' ? 1 : b === 'sin-fecha' ? -1 : compareIn(sort.years, a, b)))
      .map(([year, list]) => [year, [...list].sort((a, b) => compareIn(sort.projects, dateOf(a), dateOf(b)))] as const);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [shown, sort, cards]);

  // Corre la acción; si el proyecto es de otro, pide tomar el control y la repite con `force`.
  const attempt = async (run: (force: boolean) => Promise<void>, force = false) => {
    setError(null);
    try {
      await run(force);
      return true;
    } catch (e) {
      if (e instanceof ProjectLockedError) {
        setLocked({ lock: e.lock, retry: () => void attempt(run, true) });
        return false;
      }
      setError(String(e));
      return false;
    }
  };

  const archive = (p: DiskProject) => {
    const archived = !cards.get(p.uri)?.card?.archived;
    void attempt((force) => actions.archive(p, archived, force));
  };

  return (
    <>
      <View style={styles.header}>
        <View style={styles.headerLeft}>
          <Text style={styles.title}>Proyectos</Text>
          <Text style={styles.stats}>
            <Text style={styles.statValue}>{shown.length}</Text> proyectos ·{' '}
            <Text style={styles.statValue}>{totalPhotos.toLocaleString('es-MX')}</Text> fotos
          </Text>
        </View>
        <Pressable style={({ pressed }) => [styles.primary, pressed && styles.pressed]} onPress={onNew} disabled={!!busy}>
          <Text style={styles.primaryText}>+ Proyecto</Text>
        </Pressable>
      </View>
      <ScrollView contentContainerStyle={styles.content}>
        {busy && <Text style={styles.busy}>{busy}…</Text>}
        {error && <Text style={styles.error}>{error}</Text>}
        {trees.length === 0 && (
          <Pressable onPress={onChooseDisk}>
            <Text style={styles.empty}>Agrega un disco en "Discos" para ver sus proyectos.</Text>
          </Pressable>
        )}
        {trees.length > 0 && projects && projects.length === 0 && (
          <Text style={styles.empty}>Los discos conectados no tienen proyectos todavía. Crea uno con "+ Proyecto".</Text>
        )}
        {groups.map(([year, list]) => (
          <View key={year} style={styles.group}>
            <Text style={styles.groupTitle}>{year === 'sin-fecha' ? 'Sin fecha' : year}</Text>
            <View style={styles.cards}>
              {list.map((p) => {
                const info = cards.get(p.uri);
                const card = info?.card;
                const title = info?.title ?? p.name.split('/').pop() ?? p.name;
                return (
                  <Pressable
                    key={p.uri}
                    style={({ pressed }) => [styles.card, card?.archived && styles.cardArchived, pressed && styles.pressed]}
                    onPress={() => onOpen(p)}
                    onLongPress={() => setMenu(p)}
                    disabled={!!busy}
                  >
                    <View style={styles.cardArt}>
                      {info?.cover ? (
                        <Image source={{ uri: `file://${coverPath(p)}` }} style={styles.cover} contentFit="cover" transition={150} />
                      ) : (
                        <Text style={styles.cardInitial}>{title[0] ?? '?'}</Text>
                      )}
                      {card && <Text style={styles.status}>{STATUS[card.workflowStatus] ?? card.workflowStatus}</Text>}
                      {card?.archived && <Text style={styles.archivedTag}>Archivado</Text>}
                    </View>
                    <View style={styles.cardTitleRow}>
                      <Text style={styles.cardName} numberOfLines={2}>
                        {title}
                      </Text>
                      <Pressable style={styles.menuButton} onPress={() => setMenu(p)} hitSlop={10} disabled={!!busy}>
                        <Text style={styles.menuButtonText}>⋯</Text>
                      </Pressable>
                    </View>
                    {card && (
                      <Text style={styles.cardMeta} numberOfLines={1}>
                        {shortDate(card.sessionDate ?? card.createdAt)} · {card.photoCount.toLocaleString('es-MX')} fotos
                      </Text>
                    )}
                    {trees.length > 1 && (
                      <Text style={styles.cardMeta} numberOfLines={1}>
                        ⛁ {labelOf(p.tree)}
                      </Text>
                    )}
                    {p.lock?.state === 'foreign' && (
                      <Text style={[styles.lockTag, p.lock.pending && styles.lockTagWarn]} numberOfLines={1}>
                        En {p.lock.installName}
                        {p.lock.pending ? ' · cambios sin devolver' : ''}
                      </Text>
                    )}
                  </Pressable>
                );
              })}
            </View>
          </View>
        ))}
      </ScrollView>

      {/* Pie, como el FilterBar del dashboard: archivados y orden, abajo a la derecha. */}
      <View style={styles.footer}>
        {archivedCount > 0 && (
          <Pressable
            style={({ pressed }) => [styles.footerButton, showArchived && styles.footerButtonOn, pressed && styles.pressed]}
            onPress={() => setShowArchived(!showArchived)}
          >
            <Text style={[styles.footerButtonText, showArchived && styles.footerButtonTextOn]}>Archivados ({archivedCount})</Text>
          </Pressable>
        )}
        <Pressable
          style={({ pressed }) => [styles.footerButton, showSort && styles.footerButtonOn, pressed && styles.pressed]}
          onPress={() => setShowSort((v) => !v)}
        >
          <Text style={[styles.footerButtonText, showSort && styles.footerButtonTextOn]}>Ordenar</Text>
        </Pressable>
      </View>
      {showSort && (
        <View style={styles.sortPanel}>
          {(
            [
              ['years', 'Años'],
              ['projects', 'Proyectos dentro del año'],
            ] as const
          ).map(([key, label]) => (
            <View key={key} style={styles.sortGroup}>
              <Text style={styles.sortLabel}>{label}</Text>
              {(
                [
                  ['desc', 'Más recientes primero'],
                  ['asc', 'Más antiguos primero'],
                ] as const
              ).map(([value, text]) => (
                <Pressable key={value} style={styles.sortOption} onPress={() => setSort({ ...sort, [key]: value })}>
                  <View style={[styles.radio, sort[key] === value && styles.radioOn]} />
                  <Text style={[styles.sortText, sort[key] === value && styles.sortTextOn]}>{text}</Text>
                </Pressable>
              ))}
            </View>
          ))}
        </View>
      )}

      {menu && (
        <Modal transparent animationType="fade" onRequestClose={() => setMenu(null)}>
          <Pressable style={styles.scrim} onPress={() => setMenu(null)}>
            <View style={styles.menu}>
              <Text style={styles.menuTitle} numberOfLines={1}>
                {cards.get(menu.uri)?.title ?? menu.name}
              </Text>
              {(
                [
                  ['Abrir carpeta', () => actions.openFolder(menu)],
                  ['Editar', () => setPending({ kind: 'edit', project: menu })],
                  [cards.get(menu.uri)?.card?.archived ? 'Desarchivar' : 'Archivar', () => archive(menu)],
                  ['Eliminar', () => setPending({ kind: 'delete', project: menu })],
                ] as const
              ).map(([label, run]) => (
                <Pressable
                  key={label}
                  style={({ pressed }) => [styles.menuItem, pressed && styles.pressed]}
                  onPress={() => {
                    setMenu(null);
                    run();
                  }}
                >
                  <Text style={[styles.menuItemText, label === 'Eliminar' && styles.danger]}>{label}</Text>
                </Pressable>
              ))}
            </View>
          </Pressable>
        </Modal>
      )}

      {pending?.kind === 'edit' && (
        <EditProjectSheet
          project={pending.project}
          info={cards.get(pending.project.uri)}
          onClose={() => setPending(null)}
          onSave={(form) =>
            attempt(async (force) => {
              await actions.edit(pending.project, form, force);
            })
          }
        />
      )}

      {pending?.kind === 'delete' && (
        <DeleteProjectSheet
          name={cards.get(pending.project.uri)?.title ?? pending.project.name}
          onClose={() => setPending(null)}
          onConfirm={() => attempt((force) => actions.remove(pending.project, force))}
        />
      )}

      {locked && (
        <Modal transparent animationType="fade" onRequestClose={() => setLocked(null)}>
          <View style={styles.scrim}>
            <View style={styles.sheet}>
              <Text style={styles.sheetTitle}>Proyecto abierto en otro dispositivo</Text>
              <Text style={styles.sheetText}>{lockMessage(locked.lock)}</Text>
              <Text style={styles.muted}>
                {locked.lock.pending
                  ? 'Tiene cambios que no llegaron al disco: si tomas el control, esos cambios quedan fuera.'
                  : 'El disco está al día.'}
              </Text>
              <View style={styles.sheetButtons}>
                <Pressable style={styles.textButton} onPress={() => setLocked(null)}>
                  <Text style={styles.textButtonText}>Cancelar</Text>
                </Pressable>
                <Pressable
                  style={({ pressed }) => [styles.create, pressed && styles.pressed]}
                  onPress={() => {
                    const { retry } = locked;
                    setLocked(null);
                    retry();
                  }}
                >
                  <Text style={styles.createText}>Tomar el control</Text>
                </Pressable>
              </View>
            </View>
          </View>
        </Modal>
      )}
    </>
  );
}

// Editar proyecto: los campos del modal del desktop (nombre, fecha, descripción)
// y "Renombrar también los archivos" cuando cambia el nombre y hay fotos.
function EditProjectSheet({
  project,
  info,
  onClose,
  onSave,
}: {
  project: DiskProject;
  info?: CardInfo;
  onClose: () => void;
  onSave: (form: EditForm) => Promise<boolean>;
}) {
  const card = info?.card;
  const initialName = card?.name ?? info?.title ?? project.name.split('/').pop() ?? '';
  const [form, setForm] = useState<EditForm>({
    name: initialName,
    date: (card?.sessionDate ?? card?.createdAt ?? '').slice(0, 10),
    description: card?.description ?? '',
    renameFiles: false,
  });
  const [saving, setSaving] = useState(false);
  const [tried, setTried] = useState(false);
  const nameOk = form.name.trim().length > 0;
  const dateOk = /^\d{4}-\d{2}-\d{2}$/.test(form.date.trim());
  const showRename = (card?.photoCount ?? 0) > 0 && form.name.trim() !== initialName.trim() && nameOk;

  const save = async () => {
    setTried(true);
    if (!nameOk || !dateOk) return;
    setSaving(true);
    const ok = await onSave({ ...form, renameFiles: showRename && form.renameFiles });
    setSaving(false);
    if (ok) onClose();
  };

  return (
    <Modal transparent animationType="fade" onRequestClose={onClose}>
      <View style={styles.scrim}>
        <View style={styles.sheet}>
          <Text style={styles.sheetTitle}>Editar proyecto</Text>
          <Text style={styles.label}>Nombre del proyecto</Text>
          <TextInput style={styles.input} value={form.name} onChangeText={(name) => setForm((f) => ({ ...f, name }))} editable={!saving} />
          {tried && !nameOk && <Text style={styles.error}>El nombre del proyecto es requerido</Text>}
          <Text style={styles.label}>Fecha de sesión (AAAA-MM-DD)</Text>
          <TextInput
            style={styles.input}
            value={form.date}
            onChangeText={(date) => setForm((f) => ({ ...f, date }))}
            keyboardType="numbers-and-punctuation"
            editable={!saving}
          />
          {tried && !dateOk && <Text style={styles.error}>La fecha de sesión es requerida</Text>}
          <Text style={styles.label}>Descripción</Text>
          <TextInput
            style={[styles.input, styles.multiline]}
            value={form.description}
            onChangeText={(description) => setForm((f) => ({ ...f, description }))}
            multiline
            editable={!saving}
          />
          {showRename && (
            <View style={styles.option}>
              <View style={styles.optionText}>
                <Text style={styles.sheetText}>Renombrar también los archivos</Text>
                <Text style={styles.muted}>Las fotos importadas con el nombre anterior pasan a llamarse …__{sanitize(form.name.trim())}</Text>
                {form.renameFiles && (
                  <Text style={styles.warning}>
                    Si editas estas fotos fuera de Lumik (Lightroom, Capture One…), tu editor puede perder la referencia a los archivos
                    renombrados. Los archivos de _exported no cambian.
                  </Text>
                )}
              </View>
              <Switch value={form.renameFiles} onValueChange={(renameFiles) => setForm((f) => ({ ...f, renameFiles }))} disabled={saving} />
            </View>
          )}
          <View style={styles.sheetButtons}>
            <Pressable style={styles.textButton} onPress={onClose} disabled={saving}>
              <Text style={styles.textButtonText}>Cancelar</Text>
            </Pressable>
            <Pressable style={({ pressed }) => [styles.create, pressed && styles.pressed, saving && styles.off]} onPress={save} disabled={saving}>
              <Text style={styles.createText}>{saving ? 'Guardando…' : 'Guardar'}</Text>
            </Pressable>
          </View>
        </View>
      </View>
    </Modal>
  );
}

function DeleteProjectSheet({ name, onClose, onConfirm }: { name: string; onClose: () => void; onConfirm: () => Promise<boolean> }) {
  const [deleting, setDeleting] = useState(false);
  return (
    <Modal transparent animationType="fade" onRequestClose={onClose}>
      <View style={styles.scrim}>
        <View style={styles.sheet}>
          <Text style={styles.sheetTitle}>Eliminar proyecto</Text>
          <Text style={styles.sheetText}>¿Seguro que quieres eliminar «{name}»?</Text>
          <Text style={styles.warning}>
            Se eliminarán de forma permanente la carpeta del proyecto y todos sus archivos (fotos, videos y metadatos). Esta acción no se puede
            deshacer.
          </Text>
          <View style={styles.sheetButtons}>
            <Pressable style={styles.textButton} onPress={onClose} disabled={deleting}>
              <Text style={styles.textButtonText}>Cancelar</Text>
            </Pressable>
            <Pressable
              style={({ pressed }) => [styles.dangerButton, pressed && styles.pressed, deleting && styles.off]}
              disabled={deleting}
              onPress={async () => {
                setDeleting(true);
                const ok = await onConfirm();
                setDeleting(false);
                if (ok) onClose();
              }}
            >
              <Text style={styles.dangerButtonText}>{deleting ? 'Eliminando…' : 'Eliminar todo'}</Text>
            </Pressable>
          </View>
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  header: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', paddingHorizontal: 24, paddingVertical: 16, gap: 16 },
  headerLeft: { gap: 2 },
  title: { color: '#fff', fontSize: 28, fontWeight: '600' },
  stats: { color: '#9a9a9a', fontSize: 14 },
  statValue: { color: '#fff', fontWeight: '600' },
  primary: { minHeight: 48, paddingHorizontal: 20, justifyContent: 'center', borderRadius: 24, backgroundColor: '#2c3a5c' },
  primaryText: { color: '#c7d6ff', fontSize: 15, fontWeight: '600' },
  pressed: { opacity: 0.7 },
  off: { opacity: 0.4 },
  content: { paddingHorizontal: 24, paddingBottom: 48, gap: 20 },
  busy: { color: '#e9c349', fontSize: 15 },
  empty: { color: '#9a9a9a', fontSize: 16, paddingVertical: 24 },
  muted: { color: '#8c8c8c', fontSize: 14 },
  error: { color: '#f87171', fontSize: 14 },
  warning: { color: '#e9c349', fontSize: 14 },
  group: { gap: 10 },
  groupTitle: { color: '#9a9a9a', fontSize: 15, fontWeight: '600' },
  cards: { flexDirection: 'row', flexWrap: 'wrap', gap: 16 },
  card: { width: 220, gap: 4 },
  cardArchived: { opacity: 0.6 },
  cardArt: { height: 150, borderRadius: 12, overflow: 'hidden', backgroundColor: '#1f1f1f', alignItems: 'center', justifyContent: 'center', marginBottom: 4 },
  cover: { width: '100%', height: '100%' },
  cardInitial: { color: '#444', fontSize: 48, fontWeight: '700' },
  status: {
    position: 'absolute',
    top: 8,
    left: 8,
    paddingHorizontal: 8,
    paddingVertical: 2,
    borderRadius: 6,
    overflow: 'hidden',
    backgroundColor: 'rgba(0,0,0,0.65)',
    color: '#e5e2e1',
    fontSize: 12,
    fontWeight: '600',
  },
  archivedTag: {
    position: 'absolute',
    top: 8,
    right: 8,
    paddingHorizontal: 8,
    paddingVertical: 2,
    borderRadius: 6,
    overflow: 'hidden',
    backgroundColor: 'rgba(0,0,0,0.65)',
    color: '#e9c349',
    fontSize: 12,
    fontWeight: '600',
  },
  cardTitleRow: { flexDirection: 'row', alignItems: 'flex-start', gap: 4 },
  cardName: { color: '#fff', fontSize: 16, flex: 1 },
  menuButton: { width: 36, height: 28, alignItems: 'center', justifyContent: 'center' },
  menuButtonText: { color: '#c2c6d7', fontSize: 20, fontWeight: '700' },
  cardMeta: { color: '#9a9a9a', fontSize: 13 },
  lockTag: { color: '#c7d6ff', fontSize: 13 },
  lockTagWarn: { color: '#e9c349' },
  footer: {
    flexDirection: 'row',
    justifyContent: 'flex-end',
    alignItems: 'center',
    gap: 8,
    paddingHorizontal: 24,
    minHeight: 64,
    borderTopWidth: 1,
    borderTopColor: '#262626',
  },
  footerButton: { minHeight: 48, paddingHorizontal: 20, justifyContent: 'center', borderRadius: 24, backgroundColor: '#262626' },
  footerButtonOn: { backgroundColor: '#2c3a5c' },
  footerButtonText: { color: '#ddd', fontSize: 15, fontWeight: '600' },
  footerButtonTextOn: { color: '#c7d6ff' },
  sortPanel: {
    position: 'absolute',
    right: 24,
    bottom: 76,
    width: 340,
    padding: 16,
    gap: 12,
    borderRadius: 14,
    backgroundColor: '#1d1d1d',
    borderWidth: 1,
    borderColor: '#2a2a2a',
  },
  sortGroup: { gap: 2 },
  sortLabel: { color: '#9a9a9a', fontSize: 13, marginBottom: 4 },
  sortOption: { flexDirection: 'row', alignItems: 'center', gap: 12, minHeight: 44 },
  radio: { width: 18, height: 18, borderRadius: 9, borderWidth: 2, borderColor: '#666' },
  radioOn: { borderColor: '#c7d6ff', backgroundColor: '#3b5bdb' },
  sortText: { color: '#ddd', fontSize: 15 },
  sortTextOn: { color: '#fff', fontWeight: '600' },
  scrim: { flex: 1, backgroundColor: 'rgba(0,0,0,0.6)', alignItems: 'center', justifyContent: 'center' },
  menu: { width: 360, paddingVertical: 8, borderRadius: 14, backgroundColor: '#1d1d1d' },
  menuTitle: { color: '#9a9a9a', fontSize: 13, paddingHorizontal: 20, paddingVertical: 10 },
  menuItem: { minHeight: 52, paddingHorizontal: 20, justifyContent: 'center' },
  menuItemText: { color: '#fff', fontSize: 16 },
  danger: { color: '#f87171' },
  sheet: { width: 560, padding: 24, gap: 8, borderRadius: 16, backgroundColor: '#1d1d1d' },
  sheetTitle: { color: '#fff', fontSize: 22, fontWeight: '600', marginBottom: 8 },
  sheetText: { color: '#ddd', fontSize: 15 },
  label: { color: '#9a9a9a', fontSize: 14, marginTop: 8 },
  input: { minHeight: 48, paddingHorizontal: 14, borderRadius: 10, backgroundColor: '#262626', color: '#fff', fontSize: 16 },
  multiline: { minHeight: 80, paddingTop: 12, textAlignVertical: 'top' },
  option: { flexDirection: 'row', alignItems: 'center', gap: 16, marginTop: 12, padding: 12, borderRadius: 10, backgroundColor: '#262626' },
  optionText: { flex: 1, gap: 4 },
  sheetButtons: { flexDirection: 'row', justifyContent: 'flex-end', alignItems: 'center', gap: 12, marginTop: 16 },
  textButton: { minHeight: 48, paddingHorizontal: 16, justifyContent: 'center' },
  textButtonText: { color: '#9a9a9a', fontSize: 16 },
  create: { minHeight: 48, paddingHorizontal: 24, justifyContent: 'center', borderRadius: 24, backgroundColor: '#e9c349' },
  createText: { color: '#131313', fontSize: 16, fontWeight: '700' },
  dangerButton: { minHeight: 48, paddingHorizontal: 22, justifyContent: 'center', borderRadius: 24, backgroundColor: '#7f1d1d' },
  dangerButtonText: { color: '#fecaca', fontSize: 15, fontWeight: '600' },
});
