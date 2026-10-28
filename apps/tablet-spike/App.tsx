// Spike de la app de tablet: una pestaña por experimento.
import { StatusBar } from 'expo-status-bar';
import { useState } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { GestureHandlerRootView } from 'react-native-gesture-handler';

import BridgeScreen from './src/BridgeScreen';
import CullingScreen from './src/CullingScreen';
import SafScreen from './src/SafScreen';

const TABS = [
  { key: 'culling', label: '2.4 Grid y visor' },
  { key: 'saf', label: '2.5 Disco (SAF)' },
  { key: 'bridge', label: '2.1 Puente' },
] as const;

type Tab = (typeof TABS)[number]['key'];

export default function App() {
  const [tab, setTab] = useState<Tab>('culling');
  // El modo limpio del culling también oculta las pestañas y la barra de estado.
  const [clean, setClean] = useState(false);

  return (
    <GestureHandlerRootView style={styles.root}>
      {!(tab === 'culling' && clean) && (
        <View style={styles.tabs}>
          {TABS.map((t) => (
            <Pressable key={t.key} style={[styles.tab, tab === t.key && styles.tabOn]} onPress={() => setTab(t.key)}>
              <Text style={[styles.tabText, tab === t.key && styles.tabTextOn]}>{t.label}</Text>
            </Pressable>
          ))}
        </View>
      )}
      {tab === 'culling' && <CullingScreen onCleanChange={setClean} />}
      {tab === 'saf' && <SafScreen />}
      {tab === 'bridge' && <BridgeScreen />}
      <StatusBar style="light" hidden={tab === 'culling' && clean} />
    </GestureHandlerRootView>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#131313' },
  tabs: { flexDirection: 'row', gap: 8, paddingTop: 36, paddingHorizontal: 16, paddingBottom: 8 },
  tab: { minHeight: 44, paddingHorizontal: 16, justifyContent: 'center', borderRadius: 22 },
  tabOn: { backgroundColor: '#262626' },
  tabText: { color: '#9a9a9a', fontSize: 15 },
  tabTextOn: { color: '#fff', fontWeight: '600' },
});
