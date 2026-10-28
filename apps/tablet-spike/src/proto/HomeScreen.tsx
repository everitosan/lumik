// Inicio del prototipo (D5, Android): rail de navegación, los proyectos del
// disco en tarjetas y el FAB de importar.
import { Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';

import type { DiskProject } from './session';

type Props = {
  trees: string[];
  disk: string | null;
  projects: DiskProject[] | null;
  busy: string | null;
  onPickDisk: () => void;
  onSelectDisk: (tree: string) => void;
  onOpen: (project: DiskProject) => void;
  onImport: () => void;
};

const RAIL = ['Proyectos', 'Discos', 'Respaldo', 'Ajustes'];

// "content://…/tree/FB5F-B3EF%3A" → "FB5F-B3EF:"
export const treeLabel = (tree: string) => decodeURIComponent(tree.split('/tree/')[1] ?? tree);

export function HomeScreen({ trees, disk, projects, busy, onPickDisk, onSelectDisk, onOpen, onImport }: Props) {
  // Agrupados por la carpeta de arriba (el año, en los discos de Lumik).
  const groups = new Map<string, DiskProject[]>();
  for (const p of projects ?? []) {
    const parts = p.name.split('/');
    const group = parts.length > 1 ? parts.slice(0, -1).join('/') : '';
    groups.set(group, [...(groups.get(group) ?? []), p]);
  }

  return (
    <View style={styles.root}>
      <View style={styles.rail}>
        {RAIL.map((label, i) => (
          <View key={label} style={styles.railItem}>
            <View style={[styles.railIcon, i === 0 && styles.railIconOn]}>
              <Text style={[styles.railGlyph, i === 0 && styles.railGlyphOn]}>{label[0]}</Text>
            </View>
            <Text style={[styles.railLabel, i === 0 && styles.railLabelOn]}>{label}</Text>
          </View>
        ))}
      </View>

      <View style={styles.main}>
        <View style={styles.header}>
          <Text style={styles.title}>Proyectos</Text>
          <View style={styles.disks}>
            {trees.map((t) => (
              <Pressable
                key={t}
                style={({ pressed }) => [styles.disk, t === disk && styles.diskOn, pressed && styles.pressed]}
                onPress={() => onSelectDisk(t)}
                disabled={!!busy}
              >
                <Text style={[styles.diskText, t === disk && styles.diskTextOn]}>{treeLabel(t)}</Text>
              </Pressable>
            ))}
            <Pressable style={({ pressed }) => [styles.disk, pressed && styles.pressed]} onPress={onPickDisk} disabled={!!busy}>
              <Text style={styles.diskText}>+ Disco</Text>
            </Pressable>
          </View>
        </View>

        <ScrollView contentContainerStyle={styles.content}>
          {busy && <Text style={styles.busy}>{busy}…</Text>}
          {!disk && <Text style={styles.empty}>Elige la carpeta del disco con "+ Disco".</Text>}
          {disk && projects && projects.length === 0 && (
            <Text style={styles.empty}>No hay proyectos en este disco (se buscan dentro de lumik/, hasta 4 niveles).</Text>
          )}
          {[...groups.entries()].map(([group, list]) => (
            <View key={group} style={styles.group}>
              {group !== '' && <Text style={styles.groupTitle}>{group}</Text>}
              <View style={styles.cards}>
                {list.map((p) => (
                  <Pressable
                    key={p.uri}
                    style={({ pressed }) => [styles.card, pressed && styles.pressed]}
                    onPress={() => onOpen(p)}
                    disabled={!!busy}
                  >
                    <View style={styles.cardArt}>
                      <Text style={styles.cardInitial}>{p.name.split('/').pop()?.[0] ?? '?'}</Text>
                    </View>
                    <Text style={styles.cardName} numberOfLines={2}>
                      {p.name.split('/').pop()}
                    </Text>
                  </Pressable>
                ))}
              </View>
            </View>
          ))}
        </ScrollView>

        <Pressable
          style={({ pressed }) => [styles.fab, pressed && styles.pressed, !projects?.length && styles.fabOff]}
          onPress={onImport}
          disabled={!projects?.length || !!busy}
        >
          <Text style={styles.fabText}>Importar</Text>
        </Pressable>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, flexDirection: 'row', backgroundColor: '#131313' },
  rail: { width: 88, paddingTop: 16, alignItems: 'center', gap: 12, borderRightWidth: 1, borderRightColor: '#222' },
  railItem: { alignItems: 'center', gap: 4, minHeight: 56 },
  railIcon: { width: 56, height: 32, borderRadius: 16, alignItems: 'center', justifyContent: 'center' },
  railIconOn: { backgroundColor: '#2c3a5c' },
  railGlyph: { color: '#9a9a9a', fontSize: 16, fontWeight: '700' },
  railGlyphOn: { color: '#c7d6ff' },
  railLabel: { color: '#9a9a9a', fontSize: 12 },
  railLabelOn: { color: '#fff', fontWeight: '600' },
  main: { flex: 1 },
  header: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', paddingHorizontal: 24, paddingVertical: 16, gap: 16 },
  title: { color: '#fff', fontSize: 28, fontWeight: '600' },
  disks: { flexDirection: 'row', flexWrap: 'wrap', gap: 8, justifyContent: 'flex-end', flexShrink: 1 },
  disk: { minHeight: 44, paddingHorizontal: 14, justifyContent: 'center', borderRadius: 22, backgroundColor: '#262626' },
  diskOn: { backgroundColor: '#e9e9e9' },
  diskText: { color: '#ddd', fontSize: 14 },
  diskTextOn: { color: '#131313', fontWeight: '600' },
  pressed: { opacity: 0.7 },
  content: { paddingHorizontal: 24, paddingBottom: 120, gap: 20 },
  busy: { color: '#e9c349', fontSize: 15 },
  empty: { color: '#9a9a9a', fontSize: 16, paddingVertical: 24 },
  group: { gap: 10 },
  groupTitle: { color: '#9a9a9a', fontSize: 15, fontWeight: '600' },
  cards: { flexDirection: 'row', flexWrap: 'wrap', gap: 16 },
  card: { width: 200, gap: 8 },
  cardArt: { height: 140, borderRadius: 12, backgroundColor: '#1f1f1f', alignItems: 'center', justifyContent: 'center' },
  cardInitial: { color: '#444', fontSize: 48, fontWeight: '700' },
  cardName: { color: '#fff', fontSize: 16 },
  fab: { position: 'absolute', right: 24, bottom: 24, minHeight: 56, paddingHorizontal: 24, borderRadius: 16, justifyContent: 'center', backgroundColor: '#2c3a5c' },
  fabOff: { opacity: 0.4 },
  fabText: { color: '#c7d6ff', fontSize: 16, fontWeight: '600' },
});
