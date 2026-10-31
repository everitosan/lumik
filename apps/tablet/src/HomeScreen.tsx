// Inicio (Android): rail de navegación con Proyectos (los de
// todos los discos conectados, como el dashboard del desktop), Discos (como
// "Almacenamiento" en la barra del desktop) y Ajustes. Importar no va aquí:
// como en el desktop, se importa desde dentro de un proyecto.
import { useState } from 'react';
import { Modal, Pressable, ScrollView, StyleSheet, Text, TextInput, View } from 'react-native';

import type { SafVolume } from '../modules/saf/src/LumikSafModule';
import type { CardInfo } from './covers';
import { type ProjectForm, today } from './createProject';
import { type ProjectMenuActions, ProjectsDashboard } from './ProjectsDashboard';
import { SettingsScreen } from './SettingsScreen';
import type { DiskProject } from './session';

type Section = 'projects' | 'disks' | 'settings';

type Props = {
  trees: string[];
  projects: DiskProject[] | null;
  busy: string | null;
  /** Nombre del disco de un permiso. */
  labelOf: (tree: string) => string;
  /** Discos conectados sin acceso: se ofrece darlo en un toque. */
  pendingVolumes: SafVolume[];
  /** Descripción de un disco sin acceso. */
  pendingLabelOf: (volume: SafVolume) => string;
  onGrant: (uuid: string) => void;
  /** De dónde viene el nombre del disco: etiqueta (no se cambia), a mano, ninguno, o disco de solo lectura. */
  nameSourceOf: (tree: string) => 'label' | 'user' | 'none' | 'readonly';
  /** Disco al que se le está poniendo nombre. */
  naming: { tree: string; suggestion: string; current?: string } | null;
  onRename: (tree: string) => void;
  /** Guarda el nombre; si falla, lanza y el formulario muestra el error. */
  onSaveName: (tree: string, name: string) => void;
  onCloseNaming: () => void;
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
  /** Menú de las tarjetas: editar, archivar y eliminar. */
  actions: ProjectMenuActions;
};

const RAIL: { key: Section | 'backup'; label: string; glyph: string }[] = [
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
          // El respaldo todavía no está en la tablet.
          const enabled = item.key !== 'backup';
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
            <Text style={styles.bannerText}>⛁ {props.pendingLabelOf(v)} está conectado y Lumik aún no tiene acceso.</Text>
            <Pressable style={({ pressed }) => [styles.bannerButton, pressed && styles.pressed]} onPress={() => onGrant(v.uuid)}>
              <Text style={styles.bannerButtonText}>Dar acceso</Text>
            </Pressable>
          </View>
        ))}
        {section === 'projects' ? (
          <ProjectsDashboard
            trees={trees}
            labelOf={labelOf}
            projects={projects}
            busy={busy}
            cards={cards}
            onOpen={onOpen}
            onNew={() => setCreating(true)}
            onChooseDisk={() => setSection('disks')}
            actions={props.actions}
          />
        ) : section === 'settings' ? (
          <SettingsScreen />
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
            nameSourceOf={props.nameSourceOf}
            onRename={props.onRename}
          />
        )}
        {creating && <NewProjectSheet trees={trees} labelOf={labelOf} onClose={() => setCreating(false)} onCreate={onCreate} />}
        {props.naming && (
          <DiskNameSheet
            key={props.naming.tree}
            {...props.naming}
            onSave={(name) => props.onSaveName(props.naming!.tree, name)}
            onClose={props.onCloseNaming}
          />
        )}
      </View>
    </View>
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
  nameSourceOf,
  onRename,
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
  nameSourceOf: Props['nameSourceOf'];
  onRename: (tree: string) => void;
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
            {/* El nombre que viene de la etiqueta se cambia en la compu, no aquí. */}
            {nameSourceOf(t) === 'label' ? (
              <Text style={styles.muted}>Etiqueta del disco</Text>
            ) : (
              !treeLabel(t).startsWith('primary:') &&
              nameSourceOf(t) !== 'readonly' && (
                <Pressable style={({ pressed }) => [styles.eject, pressed && styles.pressed]} onPress={() => onRename(t)} disabled={!!busy}>
                  <Text style={styles.ejectText}>{nameSourceOf(t) === 'user' ? 'Cambiar nombre' : 'Poner nombre'}</Text>
                </Pressable>
              )
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
  labelOf,
  onClose,
  onCreate,
}: {
  trees: string[];
  labelOf: (tree: string) => string;
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
                <Text style={[styles.diskChoiceText, form.disk === t && styles.diskChoiceTextOn]}>⛁ {labelOf(t)}</Text>
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

// Nombre del disco, guardado en el disco (`lumik/.disk.json`) para verlo igual
// en otras tablets. Sale al dar acceso a un disco sin nombre y desde "Discos".
function DiskNameSheet({
  suggestion,
  current,
  onSave,
  onClose,
}: {
  suggestion: string;
  current?: string;
  onSave: (name: string) => void;
  onClose: () => void;
}) {
  const [name, setName] = useState(current ?? '');
  const [error, setError] = useState<string | null>(null);
  const valid = name.trim().length > 0;

  const save = () => {
    if (!valid) return;
    try {
      onSave(name);
      onClose();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <Modal transparent animationType="fade" onRequestClose={onClose}>
      <View style={styles.scrim}>
        <View style={styles.sheet}>
          <Text style={styles.sheetTitle}>Nombre del disco</Text>
          <Text style={styles.muted}>
            Android no da el nombre de este disco. Ponle uno para reconocerlo; se guarda en el disco y si lo conectas a una
            compu con Lumik, se usa su etiqueta.
          </Text>
          <Text style={styles.label}>Nombre</Text>
          <TextInput
            style={styles.input}
            value={name}
            onChangeText={setName}
            placeholder={suggestion}
            placeholderTextColor="#666"
            autoFocus
            onSubmitEditing={save}
          />
          {error && <Text style={styles.error}>{error}</Text>}
          <View style={styles.sheetButtons}>
            <Pressable style={styles.textButton} onPress={onClose}>
              <Text style={styles.textButtonText}>{current ? 'Cancelar' : 'Omitir'}</Text>
            </Pressable>
            <Pressable style={({ pressed }) => [styles.create, pressed && styles.pressed, !valid && styles.off]} onPress={save}>
              <Text style={styles.createText}>Guardar</Text>
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
  empty: { color: '#9a9a9a', fontSize: 16, paddingVertical: 24 },
  muted: { color: '#777', fontSize: 14 },
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
