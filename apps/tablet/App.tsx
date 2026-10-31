// Lumik para tablet: inicio (proyectos y discos) → proyecto → foto, e importar
// desde el proyecto. El modo limpio del visor oculta también la barra de estado.
import { StatusBar } from 'expo-status-bar';
import { useState } from 'react';
import { StatusBar as RNStatusBar, StyleSheet } from 'react-native';
import { GestureHandlerRootView } from 'react-native-gesture-handler';

import AppScreen from './src/AppScreen';

export default function App() {
  const [clean, setClean] = useState(false);

  return (
    // Fuera del modo limpio, el contenido empieza bajo la barra de estado.
    <GestureHandlerRootView style={[styles.root, { paddingTop: clean ? 0 : (RNStatusBar.currentHeight ?? 0) }]}>
      <AppScreen onCleanChange={setClean} />
      <StatusBar style="light" hidden={clean} />
    </GestureHandlerRootView>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#131313' },
});
