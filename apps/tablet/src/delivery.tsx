// "Al aprobar, entregar directamente" (spec `camera-jpeg-delivery`): las tres
// opciones del desktop (`DeliveryModeSelect`) y el selector que usan Ajustes
// (la de los proyectos nuevos) y el import (la del proyecto).
import { Pressable, StyleSheet, Text, View } from 'react-native';

import { readPrefs } from './prefs';

export const DELIVERY: { mode: string; label: string; detail: string }[] = [
  { mode: 'none', label: 'Nada', detail: 'Exportas desde tu editor.' },
  { mode: 'pairs', label: 'El JPG de los pares RAW+JPG', detail: 'Al aprobar, el JPG de cámara va a _exported/.' },
  { mode: 'all_camera_jpg', label: 'Todo JPG de cámara', detail: 'Pares y JPG sueltos van a _exported/ al aprobar.' },
];

/** Entrega con que se crean los proyectos nuevos; como el desktop, `none` de fábrica. */
export function defaultDelivery(): string {
  return readPrefs().defaultDelivery ?? 'none';
}

export function DeliveryChoice({ value, onChange, disabled }: { value: string | null; onChange: (mode: string) => void; disabled?: boolean }) {
  return (
    <View style={styles.list}>
      {DELIVERY.map((d) => (
        <Pressable
          key={d.mode}
          style={({ pressed }) => [styles.option, value === d.mode && styles.optionOn, pressed && styles.pressed]}
          onPress={() => onChange(d.mode)}
          disabled={disabled}
        >
          <Text style={[styles.label, value === d.mode && styles.labelOn]}>{d.label}</Text>
          <Text style={styles.detail}>{d.detail}</Text>
        </Pressable>
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  list: { gap: 12 },
  option: { gap: 2, paddingHorizontal: 16, paddingVertical: 12, borderRadius: 12, backgroundColor: '#1f1f1f', borderWidth: 2, borderColor: 'transparent' },
  optionOn: { borderColor: '#3b5bdb', backgroundColor: '#1c2540' },
  pressed: { opacity: 0.7 },
  label: { color: '#ddd', fontSize: 16 },
  labelOn: { color: '#fff', fontWeight: '600' },
  detail: { color: '#9a9a9a', fontSize: 15 },
});
