// Fecha de la sesión con el selector de fecha de Android (spec `project-management`,
// "Formulario de crear proyecto"). El valor sigue siendo `AAAA-MM-DD`, como en el desktop.
import { DateTimePickerAndroid } from '@react-native-community/datetimepicker';
import { Pressable, StyleSheet, Text } from 'react-native';

import { Icon } from './Icon';

const toDate = (value: string) => {
  const [y, m, d] = value.split('-').map(Number);
  return y && m && d ? new Date(y, m - 1, d) : new Date();
};

const toValue = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;

/** "30 de octubre de 2026" */
const pretty = (value: string) =>
  toDate(value).toLocaleDateString('es-MX', { day: 'numeric', month: 'long', year: 'numeric' });

export function DateField({ value, onChange, disabled }: { value: string; onChange: (v: string) => void; disabled?: boolean }) {
  const open = () =>
    DateTimePickerAndroid.open({
      value: toDate(value),
      mode: 'date',
      onChange: (event, date) => {
        if (event.type === 'set' && date) onChange(toValue(date));
      },
    });

  return (
    <Pressable style={({ pressed }) => [styles.field, pressed && styles.pressed]} onPress={open} disabled={disabled}>
      <Text style={styles.text}>{value ? pretty(value) : 'Elegir fecha'}</Text>
      <Icon name="calendar" size={18} color="#9a9a9a" />
    </Pressable>
  );
}

const styles = StyleSheet.create({
  field: { minHeight: 48, paddingHorizontal: 14, borderRadius: 10, backgroundColor: '#262626', flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between' },
  pressed: { opacity: 0.7 },
  text: { color: '#fff', fontSize: 16 },
});
