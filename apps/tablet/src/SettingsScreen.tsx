// Ajustes de la tablet (spec `tablet-settings`): la firma del fotógrafo,
// cómo se importa, la entrega al aprobar de los proyectos nuevos y el nombre con que la tablet aparece en el candado de los
// proyectos. Se guardan en la sandbox al tocar "Guardar".
import { useState } from 'react';
import { Pressable, ScrollView, StyleSheet, Switch, Text, TextInput, View } from 'react-native';

import { DeliveryChoice, defaultDelivery } from './delivery';
import { renameInstall, thisInstall } from './lock';
import { importSettings, savePrefs } from './prefs';

export function SettingsScreen() {
  const [settings, setSettings] = useState(importSettings);
  const [delivery, setDelivery] = useState(defaultDelivery);
  const [installName, setInstallName] = useState(() => thisInstall().name);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const change = (patch: Partial<typeof settings>) => {
    setSettings((s) => ({ ...s, ...patch }));
    setSaved(false);
  };

  const save = () => {
    try {
      savePrefs({ importSettings: settings, defaultDelivery: delivery });
      setInstallName(renameInstall(installName).name);
      setError(null);
      setSaved(true);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <>
      <View style={styles.header}>
        <Text style={styles.title}>Ajustes</Text>
        <Pressable style={({ pressed }) => [styles.primary, pressed && styles.pressed]} onPress={save}>
          <Text style={styles.primaryText}>{saved ? 'Guardado' : 'Guardar'}</Text>
        </Pressable>
      </View>
      <ScrollView contentContainerStyle={styles.content}>
        {error && <Text style={styles.error}>{error}</Text>}

        <Text style={styles.section}>Fotógrafo</Text>
        <Text style={styles.muted}>Se escribe en las fotos al importar, en un archivo .xmp junto a cada una.</Text>
        <Field label="Autor" value={settings.author} onChange={(author) => change({ author })} placeholder="Ana López" />
        <Field label="Copyright" value={settings.copyright} onChange={(copyright) => change({ copyright })} placeholder="© 2026 Ana López" />
        <Field label="URL de contacto" value={settings.url} onChange={(url) => change({ url })} placeholder="https://analopez.mx" url />

        <Text style={styles.section}>Importación</Text>
        <Toggle label="Firmar las fotos al importar" value={settings.sign} onChange={(sign) => change({ sign })} />
        <Toggle label="Renombrar por fecha de captura al importar" value={settings.rename} onChange={(rename) => change({ rename })} />

        <Text style={styles.section}>Entrega al aprobar en proyectos nuevos</Text>
        <Text style={styles.muted}>
          Cada proyecto toma este valor al crearse y después guarda el suyo. Se cambia al importar.
        </Text>
        <DeliveryChoice
          value={delivery}
          onChange={(mode) => {
            setDelivery(mode);
            setSaved(false);
          }}
        />

        <Text style={styles.section}>Esta tablet</Text>
        <Text style={styles.muted}>Así aparece en el desktop cuando tiene un proyecto abierto.</Text>
        <Field
          label="Nombre"
          value={installName}
          onChange={(v) => {
            setInstallName(v ?? '');
            setSaved(false);
          }}
        />
      </ScrollView>
    </>
  );
}

function Field({
  label,
  value,
  onChange,
  placeholder,
  url,
}: {
  label: string;
  value?: string;
  onChange: (v: string | undefined) => void;
  placeholder?: string;
  url?: boolean;
}) {
  return (
    <View style={styles.field}>
      <Text style={styles.label}>{label}</Text>
      <TextInput
        style={styles.input}
        value={value ?? ''}
        onChangeText={(t) => onChange(t.length ? t : undefined)}
        placeholder={placeholder}
        placeholderTextColor="#666"
        autoCapitalize={url ? 'none' : 'words'}
        keyboardType={url ? 'url' : 'default'}
      />
    </View>
  );
}

function Toggle({ label, value, onChange }: { label: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <Pressable style={styles.toggle} onPress={() => onChange(!value)}>
      <Text style={styles.toggleText}>{label}</Text>
      <Switch value={value} onValueChange={onChange} trackColor={{ true: '#3b5bdb', false: '#333' }} thumbColor="#fff" />
    </Pressable>
  );
}

const styles = StyleSheet.create({
  header: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', paddingHorizontal: 24, paddingVertical: 16, gap: 16 },
  title: { color: '#fff', fontSize: 28, fontWeight: '600' },
  primary: { minHeight: 48, paddingHorizontal: 20, justifyContent: 'center', borderRadius: 24, backgroundColor: '#2c3a5c' },
  primaryText: { color: '#c7d6ff', fontSize: 15, fontWeight: '600' },
  pressed: { opacity: 0.7 },
  content: { paddingHorizontal: 24, paddingBottom: 48, gap: 12, maxWidth: 720 },
  section: { color: '#fff', fontSize: 18, fontWeight: '600', marginTop: 16 },
  muted: { color: '#777', fontSize: 14 },
  error: { color: '#f87171', fontSize: 14 },
  field: { gap: 6 },
  label: { color: '#9a9a9a', fontSize: 14 },
  input: { minHeight: 48, paddingHorizontal: 14, borderRadius: 10, backgroundColor: '#262626', color: '#fff', fontSize: 16 },
  toggle: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', minHeight: 56, paddingHorizontal: 16, borderRadius: 12, backgroundColor: '#1f1f1f' },
  toggleText: { color: '#fff', fontSize: 16, flex: 1 },
});
