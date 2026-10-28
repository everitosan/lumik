// Inicio del prototipo (D5, Android): rail de navegación con Proyectos (los de
// todos los discos conectados, como el dashboard del desktop) y Discos (como
// "Almacenamiento" en la barra del desktop). Importar no va aquí:
// como en el desktop, se importa desde dentro de un proyecto.
import { Image } from 'expo-image';
import { useState } from 'react';
import { Modal, Pressable, ScrollView, StyleSheet, Text, TextInput, View } from 'react-native';

import type { SafVolume } from '../../modules/saf/src/LumikSafModule';
import { type CardInfo, coverPath } from './covers';
import { type ProjectForm, today } from './createProject';
import type { DiskProject } from './session';

type Section = 'projects' | 'disks';

type Props = {
  trees: string[];
  projects: DiskProject[] | null;
  busy: string | null;
  /** Nombre del disco de un permiso. */
  labelOf: (tree: string) => string;
  /** Discos conectados sin acceso: se ofrece darlo en un toque. */
  pendingVolumes: SafVolume[];
  onGrant: (uuid: string) => void;
  /** Discos expulsados desde la app, que ya se pueden desconectar. */
  ejected: string[];
  onEject: (tree: string) => void;
  onUnEject: (tree: string) => void;
  onStorageSettings: () => void;
  onPickDisk: () => void;
  onOpen: (project: DiskProject) => void;
  /** Nombre y portada de cada proyecto (por `uri`), cuando ya llegaron. */
  cards: Map<string, CardInfo>;
  /** Crea el proyecto; si falla, lanza y el formulario muestra el error. */
  onCreate: (form: ProjectForm) => Promise<void>;
};

const RAIL: { key: Section | 'backup' | 'settings'; label: string; glyph: string }[] = [
  { key: 'projects', label: 'Proyectos', glyph: '▦' },
  { key: 'disks', label: 'Discos', glyph: '⛁' },
  { key: 'backup', label: 'Respaldo', glyph: '☁' },
  { key: 'settings', label: 'Ajustes', glyph: '⚙' },
];

// "content://…/tree/FB5F-B3EF%3A" → "FB5F-B3EF:"
export const treeLabel = (tree: string) => decodeURIComponent(tree.split('/tree/')[1] ?? tree);

export function HomeScreen(props: Props) {
  const { trees, projects, busy, labelOf, pendingVolumes, onGrant, onPickDisk, onOpen, cards, onCreate } = props;
  const [section, setSection] = useState<Section>('projects');
  const [creating, setCreating] = useState(false);

  return (
    <View style={styles.root}>
      <View style={styles.rail}>
        {RAIL.map((item) => {
          const enabled = item.key === 'projects' || item.key === 'disks';
          const on = item.key === section;
          return (
            <Pressable
              key={item.key}
              style={[styles.railItem, !enabled && styles.railOff]}
              onPress={() => enabled && setSection(item.key as Section)}
              disabled={!enabled}
            >
              <View style={[styles.railIcon, on && styles.railIconOn]}>
                <Text style={[styles.railGlyph, on && styles.railGlyphOn]}>{item.glyph}</Text>
              </View>
              <Text style={[styles.railLabel, on && styles.railLabelOn]}>{item.label}</Text>
            </Pressable>
          );
        })}
      </View>

      <View style={styles.main}>
        {pendingVolumes.map((v) => (
          <View key={v.uuid} style={styles.banner}>
            <Text style={styles.bannerText}>⛁ {v.label} está conectado y Lumik aún no tiene acceso.</Text>
            <Pressable style={({ pressed }) => [styles.bannerButton, pressed && styles.pressed]} onPress={() => onGrant(v.uuid)}>
              <Text style={styles.bannerButtonText}>Dar acceso</Text>
            </Pressable>
          </View>
        ))}
        {section === 'projects' ? (
          <Projects
            trees={trees}
            labelOf={labelOf}
            projects={projects}
            busy={busy}
            cards={cards}
            onOpen={onOpen}
            onNew={() => setCreating(true)}
            onChooseDisk={() => setSection('disks')}
          />
        ) : (
          <Disks
            trees={trees}
            projects={projects}
            busy={busy}
            labelOf={labelOf}
            ejected={props.ejected}
            onEject={props.onEject}
            onUnEject={props.onUnEject}
            onStorageSettings={props.onStorageSettings}
            onPickDisk={onPickDisk}
          />
        )}
        {creating && <NewProjectSheet trees={trees} onClose={() => setCreating(false)} onCreate={onCreate} />}
      </View>
    </View>
  );
}

function Projects({
  trees,
  labelOf,
  projects,
  busy,
  cards,
  onOpen,
  onNew,
  onChooseDisk,
}: {
  trees: string[];
  labelOf: (tree: string) => string;
  projects: DiskProject[] | null;
  busy: string | null;
  cards: Map<string, CardInfo>;
  onOpen: (p: DiskProject) => void;
  onNew: () => void;
  onChooseDisk: () => void;
}) {
  // Como el desktop: si el mismo proyecto (`project.id`) está en dos discos, se lista uno.
  const seen = new Set<string>();
  const unique = (projects ?? []).filter((p) => {
    const id = cards.get(p.uri)?.id;
    if (!id) return true;
    if (seen.has(id)) return false;
    seen.add(id);
    return true;
  });
  // Agrupados por año (la primera carpeta, en los discos de Lumik).
  const groups = new Map<string, DiskProject[]>();
  for (const p of unique) {
    const year = p.name.split('/')[0];
    const group = p.name.includes('/') && /^\d{4}$/.test(year) ? year : '';
    groups.set(group, [...(groups.get(group) ?? []), p]);
  }

  return (
    <>
      <View style={styles.header}>
        <Text style={styles.title}>Proyectos</Text>
        <Pressable style={({ pressed }) => [styles.primary, pressed && styles.pressed]} onPress={onNew} disabled={!!busy}>
          <Text style={styles.primaryText}>+ Proyecto</Text>
        </Pressable>
      </View>
      <ScrollView contentContainerStyle={styles.content}>
        {busy && <Text style={styles.busy}>{busy}…</Text>}
        {trees.length === 0 && (
          <Pressable onPress={onChooseDisk}>
            <Text style={styles.empty}>Agrega un disco en "Discos" para ver sus proyectos.</Text>
          </Pressable>
        )}
        {trees.length > 0 && projects && projects.length === 0 && (
          <Text style={styles.empty}>Los discos conectados no tienen proyectos todavía. Crea uno con "+ Proyecto".</Text>
        )}
        {[...groups.entries()].map(([group, list]) => (
          <View key={group} style={styles.group}>
            {group !== '' && <Text style={styles.groupTitle}>{group}</Text>}
            <View style={styles.cards}>
              {list.map((p) => {
                const card = cards.get(p.uri);
                const title = card?.title ?? p.name.split('/').pop() ?? p.name;
                return (
                  <Pressable
                    key={p.uri}
                    style={({ pressed }) => [styles.card, pressed && styles.pressed]}
                    onPress={() => onOpen(p)}
                    disabled={!!busy}
                  >
                    <View style={styles.cardArt}>
                      {card?.cover ? (
                        <Image source={{ uri: `file://${coverPath(p)}` }} style={styles.cover} contentFit="cover" transition={150} />
                      ) : (
                        <Text style={styles.cardInitial}>{title[0] ?? '?'}</Text>
                      )}
                    </View>
                    <Text style={styles.cardName} numberOfLines={2}>
                      {title}
                    </Text>
                    {trees.length > 1 && (
                      <Text style={styles.cardDisk} numberOfLines={1}>
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
    </>
  );
}

function Disks({
  trees,
  projects,
  busy,
  labelOf,
  ejected,
  onEject,
  onUnEject,
  onStorageSettings,
  onPickDisk,
}: {
  trees: string[];
  projects: DiskProject[] | null;
  busy: string | null;
  labelOf: (tree: string) => string;
  ejected: string[];
  onEject: (tree: string) => void;
  onUnEject: (tree: string) => void;
  onStorageSettings: () => void;
  onPickDisk: () => void;
}) {
  return (
    <>
      <View style={styles.header}>
        <Text style={styles.title}>Discos</Text>
        <Pressable style={({ pressed }) => [styles.primary, pressed && styles.pressed]} onPress={onPickDisk} disabled={!!busy}>
          <Text style={styles.primaryText}>+ Agregar disco</Text>
        </Pressable>
      </View>
      <ScrollView contentContainerStyle={styles.content}>
        {trees.length === 0 && (
          <Text style={styles.empty}>Conecta el disco y toca "+ Agregar disco" para elegir su carpeta. Solo se pide una vez.</Text>
        )}
        {trees.map((t) => (
          <View key={t} style={styles.diskRow}>
            <Text style={styles.diskGlyph}>⛁</Text>
            <Text style={styles.diskName}>{labelOf(t)}</Text>
            {projects && (
              <Text style={styles.diskCurrent}>
                {projects.filter((p) => p.tree === t).length} proyectos
              </Text>
            )}
            {/* La memoria interna no se expulsa. */}
            {!treeLabel(t).startsWith('primary:') && (
              <Pressable style={({ pressed }) => [styles.eject, pressed && styles.pressed]} onPress={() => onEject(t)} disabled={!!busy}>
                <Text style={styles.ejectText}>⏏ Expulsar</Text>
              </Pressable>
            )}
          </View>
        ))}
        {ejected.map((t) => (
          <View key={t} style={[styles.diskRow, styles.diskRowEjected]}>
            <Text style={styles.diskGlyph}>⏏</Text>
            <View style={styles.diskEjectedText}>
              <Text style={styles.diskName}>{labelOf(t)}</Text>
              <Text style={styles.muted}>Expulsado: ya puedes desconectarlo.</Text>
            </View>
            <Pressable style={({ pressed }) => [styles.eject, pressed && styles.pressed]} onPress={onStorageSettings}>
              <Text style={styles.ejectText}>Almacenamiento de Android</Text>
            </Pressable>
            <Pressable style={({ pressed }) => [styles.eject, pressed && styles.pressed]} onPress={() => onUnEject(t)}>
              <Text style={styles.ejectText}>Volver a usar</Text>
            </Pressable>
          </View>
        ))}
        <Text style={styles.muted}>Solo aparecen los discos conectados. En "Proyectos" están los de todos.</Text>
      </ScrollView>
    </>
  );
}

// Nuevo proyecto: los mismos campos que el modal del desktop (nombre, fecha
// de la sesión, descripción y disco); la carpeta sale de la fecha. Como en el
// desktop, el disco es obligatorio y se preselecciona solo si hay uno.
function NewProjectSheet({
  trees,
  onClose,
  onCreate,
}: {
  trees: string[];
  onClose: () => void;
  onCreate: (f: ProjectForm) => Promise<void>;
}) {
  const [form, setForm] = useState<ProjectForm>({
    name: '',
    date: today(),
    description: '',
    disk: trees.length === 1 ? trees[0] : null,
  });
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [tried, setTried] = useState(false);
  const valid = form.name.trim().length > 0 && /^\d{4}-\d{2}-\d{2}$/.test(form.date.trim()) && !!form.disk;

  const save = async () => {
    setTried(true);
    if (!valid) return;
    setSaving(true);
    setError(null);
    try {
      await onCreate(form);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal transparent animationType="fade" onRequestClose={onClose}>
      <View style={styles.scrim}>
        <View style={styles.sheet}>
          <Text style={styles.sheetTitle}>Nuevo proyecto</Text>
          <Text style={styles.label}>Nombre</Text>
          <TextInput
            style={styles.input}
            value={form.name}
            onChangeText={(name) => setForm((f) => ({ ...f, name }))}
            placeholder="Boda de Ana y Luis"
            placeholderTextColor="#666"
            autoFocus
          />
          <Text style={styles.label}>Fecha de la sesión (AAAA-MM-DD)</Text>
          <TextInput
            style={styles.input}
            value={form.date}
            onChangeText={(date) => setForm((f) => ({ ...f, date }))}
            keyboardType="numbers-and-punctuation"
          />
          <Text style={styles.label}>Disco</Text>
          {trees.length === 0 && <Text style={styles.error}>No hay discos conectados: agrega uno en "Discos".</Text>}
          <View style={styles.diskChoices}>
            {trees.map((t) => (
              <Pressable
                key={t}
                style={({ pressed }) => [styles.diskChoice, form.disk === t && styles.diskChoiceOn, pressed && styles.pressed]}
                onPress={() => setForm((f) => ({ ...f, disk: t }))}
              >
                <Text style={[styles.diskChoiceText, form.disk === t && styles.diskChoiceTextOn]}>⛁ {treeLabel(t)}</Text>
              </Pressable>
            ))}
          </View>
          {tried && !form.disk && trees.length > 0 && <Text style={styles.error}>Elige el disco del proyecto.</Text>}
          <Text style={styles.label}>Descripción (opcional)</Text>
          <TextInput
            style={[styles.input, styles.multiline]}
            value={form.description}
            onChangeText={(description) => setForm((f) => ({ ...f, description }))}
            multiline
          />
          {error && <Text style={styles.error}>{error}</Text>}
          <View style={styles.sheetButtons}>
            <Pressable style={styles.textButton} onPress={onClose} disabled={saving}>
              <Text style={styles.textButtonText}>Cancelar</Text>
            </Pressable>
            <Pressable
              style={({ pressed }) => [styles.create, pressed && styles.pressed, (!valid || saving) && styles.off]}
              onPress={save}
              disabled={saving}
            >
              <Text style={styles.createText}>{saving ? 'Creando…' : 'Crear'}</Text>
            </Pressable>
          </View>
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, flexDirection: 'row', backgroundColor: '#131313' },
  rail: { width: 88, paddingTop: 16, alignItems: 'center', gap: 12, borderRightWidth: 1, borderRightColor: '#222' },
  railItem: { alignItems: 'center', gap: 4, minHeight: 56 },
  railOff: { opacity: 0.35 },
  railIcon: { width: 56, height: 32, borderRadius: 16, alignItems: 'center', justifyContent: 'center' },
  railIconOn: { backgroundColor: '#2c3a5c' },
  railGlyph: { color: '#9a9a9a', fontSize: 18 },
  railGlyphOn: { color: '#c7d6ff' },
  railLabel: { color: '#9a9a9a', fontSize: 12 },
  railLabelOn: { color: '#fff', fontWeight: '600' },
  main: { flex: 1 },
  header: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', paddingHorizontal: 24, paddingVertical: 16, gap: 16 },
  title: { color: '#fff', fontSize: 28, fontWeight: '600' },
  primary: { minHeight: 48, paddingHorizontal: 20, justifyContent: 'center', borderRadius: 24, backgroundColor: '#2c3a5c' },
  primaryText: { color: '#c7d6ff', fontSize: 15, fontWeight: '600' },
  pressed: { opacity: 0.7 },
  off: { opacity: 0.4 },
  content: { paddingHorizontal: 24, paddingBottom: 48, gap: 20 },
  busy: { color: '#e9c349', fontSize: 15 },
  empty: { color: '#9a9a9a', fontSize: 16, paddingVertical: 24 },
  muted: { color: '#777', fontSize: 14 },
  group: { gap: 10 },
  groupTitle: { color: '#9a9a9a', fontSize: 15, fontWeight: '600' },
  cards: { flexDirection: 'row', flexWrap: 'wrap', gap: 16 },
  card: { width: 200, gap: 8 },
  cardArt: { height: 140, borderRadius: 12, overflow: 'hidden', backgroundColor: '#1f1f1f', alignItems: 'center', justifyContent: 'center' },
  cover: { width: '100%', height: '100%' },
  cardInitial: { color: '#444', fontSize: 48, fontWeight: '700' },
  cardName: { color: '#fff', fontSize: 16 },
  cardDisk: { color: '#9a9a9a', fontSize: 13 },
  banner: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 16,
    marginHorizontal: 32,
    marginTop: 16,
    paddingHorizontal: 20,
    paddingVertical: 10,
    borderRadius: 12,
    backgroundColor: '#1c2438',
  },
  bannerText: { color: '#c7d6ff', fontSize: 15, flex: 1 },
  bannerButton: { minHeight: 44, paddingHorizontal: 18, justifyContent: 'center', borderRadius: 22, backgroundColor: '#3b5bdb' },
  bannerButtonText: { color: '#fff', fontSize: 15, fontWeight: '600' },
  eject: { minHeight: 44, paddingHorizontal: 16, justifyContent: 'center', borderRadius: 22, backgroundColor: '#2a2a2a' },
  ejectText: { color: '#ddd', fontSize: 14, fontWeight: '600' },
  diskRowEjected: { opacity: 0.8 },
  diskEjectedText: { flex: 1, gap: 2 },
  lockTag: { color: '#c7d6ff', fontSize: 13 },
  lockTagWarn: { color: '#e9c349' },
  diskRow: { flexDirection: 'row', alignItems: 'center', gap: 16, minHeight: 64, paddingHorizontal: 20, borderRadius: 12, backgroundColor: '#1f1f1f' },
  diskGlyph: { color: '#9a9a9a', fontSize: 22 },
  diskName: { color: '#fff', fontSize: 17, flex: 1 },
  diskCurrent: { color: '#c7d6ff', fontSize: 14 },
  diskChoices: { flexDirection: 'row', flexWrap: 'wrap', gap: 8 },
  diskChoice: { minHeight: 48, paddingHorizontal: 16, justifyContent: 'center', borderRadius: 10, backgroundColor: '#262626', borderWidth: 2, borderColor: 'transparent' },
  diskChoiceOn: { borderColor: '#3b5bdb', backgroundColor: '#1c2540' },
  diskChoiceText: { color: '#ddd', fontSize: 15 },
  diskChoiceTextOn: { color: '#fff', fontWeight: '600' },
  scrim: { flex: 1, backgroundColor: 'rgba(0,0,0,0.6)', alignItems: 'center', justifyContent: 'center' },
  sheet: { width: 520, padding: 24, gap: 8, borderRadius: 16, backgroundColor: '#1d1d1d' },
  sheetTitle: { color: '#fff', fontSize: 22, fontWeight: '600', marginBottom: 8 },
  label: { color: '#9a9a9a', fontSize: 14, marginTop: 8 },
  input: { minHeight: 48, paddingHorizontal: 14, borderRadius: 10, backgroundColor: '#262626', color: '#fff', fontSize: 16 },
  multiline: { minHeight: 80, paddingTop: 12, textAlignVertical: 'top' },
  error: { color: '#f87171', fontSize: 14, marginTop: 8 },
  sheetButtons: { flexDirection: 'row', justifyContent: 'flex-end', alignItems: 'center', gap: 12, marginTop: 16 },
  textButton: { minHeight: 48, paddingHorizontal: 16, justifyContent: 'center' },
  textButtonText: { color: '#9a9a9a', fontSize: 16 },
  create: { minHeight: 48, paddingHorizontal: 24, justifyContent: 'center', borderRadius: 24, backgroundColor: '#e9c349' },
  createText: { color: '#131313', fontSize: 16, fontWeight: '700' },
});
