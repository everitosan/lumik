// Importar (D5, Android): pantalla completa paso a paso.
//  1 de 3: a qué proyecto · 2 de 3: qué archivos · 3 de 3: importar.
// Los archivos se eligen como en el wizard del desktop: selector del sistema
// con selección múltiple, y una lista a la que se agregan y de la que se quitan.
import { useState } from 'react';
import { ActivityIndicator, FlatList, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';

import Saf from '../../modules/saf/src/LumikSafModule';
import type { SafEntry } from '../../modules/saf/src/LumikSaf.types';
import { importCard, type ImportProgress, type ImportReport, isImportable } from './importer';
import { type DiskProject, ProjectSession } from './session';

type Props = {
  projects: DiskProject[];
  onClose: () => void;
  /** Al terminar: abrir el proyecto para cullar. */
  onOpenProject: (project: DiskProject) => void;
};

const PHASES: Record<string, string> = {
  copy: 'Copiando de la tarjeta',
  metadata: 'Renombrando y firmando',
  move: 'Ordenando en el proyecto',
  register: 'Registrando y creando miniaturas',
  complete: 'Registrando y creando miniaturas',
  previews: 'Creando vistas previas',
};

export function ImportScreen({ projects, onClose, onOpenProject }: Props) {
  const [project, setProject] = useState<DiskProject | null>(null);
  const [card, setCard] = useState<SafEntry[]>([]);
  const [confirmed, setConfirmed] = useState(false);
  const [ignored, setIgnored] = useState(0);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [startedAt, setStartedAt] = useState(0);

  const step = !project ? 1 : !confirmed ? 2 : 3;
  const running = busy && step === 3;

  const pickFiles = async () => {
    setError(null);
    setBusy(true);
    try {
      const picked = await Saf.pickFiles();
      const ok = picked.filter((f) => isImportable(f.name));
      setIgnored(picked.length - ok.length);
      setCard((current) => {
        const seen = new Set(current.map((f) => f.uri));
        return [...current, ...ok.filter((f) => !seen.has(f.uri))];
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const start = async () => {
    if (!project || !card) return;
    setError(null);
    setBusy(true);
    setStartedAt(Date.now());
    try {
      const session = await ProjectSession.open(project, () => {}, () => {});
      setReport(await importCard(session, card, setProgress));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const raws = card.filter((f) => !/\.(jpe?g|heic)$/i.test(f.name)).length;
  const bytes = card.reduce((s, f) => s + f.size, 0);

  return (
    <View style={styles.root}>
      <View style={styles.header}>
        <Pressable style={styles.back} onPress={onClose} disabled={running} hitSlop={8}>
          <Text style={styles.backText}>←</Text>
        </Pressable>
        <Text style={styles.title}>Importar</Text>
        <Text style={styles.step}>{report ? 'Listo' : `${step} de 3`}</Text>
      </View>

      <ScrollView contentContainerStyle={styles.content}>
        {error && <Text style={styles.error}>{error}</Text>}

        {step === 1 && (
          <>
            <Text style={styles.h2}>¿A qué proyecto?</Text>
            {projects.map((p) => (
              <Pressable key={p.uri} style={({ pressed }) => [styles.item, pressed && styles.pressed]} onPress={() => setProject(p)}>
                <Text style={styles.text}>{p.name}</Text>
              </Pressable>
            ))}
          </>
        )}

        {step === 2 && (
          <>
            <Text style={styles.h2}>Origen → {project!.name}</Text>
            <Text style={styles.muted}>
              Elige las fotos de la tarjeta. En el explorador puedes marcar varias, o usar "Seleccionar todo" en una carpeta.
            </Text>
            <View style={styles.row}>
              <Button label={card.length ? 'Agregar más' : 'Elegir archivos'} onPress={pickFiles} disabled={busy} />
              {card.length > 0 && <Button label="Siguiente" onPress={() => setConfirmed(true)} primary />}
            </View>
            {ignored > 0 && <Text style={styles.warn}>{ignored} archivos no son fotos que Lumik importe y se dejaron fuera.</Text>}
            {card.length > 0 && (
              <>
                <Text style={styles.muted}>
                  {card.length} archivos · {(bytes / 1e9).toFixed(2)} GB
                </Text>
                <FlatList
                  data={card}
                  keyExtractor={(f) => f.uri}
                  scrollEnabled={false}
                  renderItem={({ item }) => (
                    <View style={styles.file}>
                      <Text style={styles.text} numberOfLines={1}>
                        {item.name}
                      </Text>
                      <Text style={styles.muted}>{(item.size / 1e6).toFixed(1)} MB</Text>
                      <Pressable
                        style={styles.remove}
                        onPress={() => setCard((c) => c.filter((f) => f.uri !== item.uri))}
                        hitSlop={8}
                      >
                        <Text style={styles.removeText}>✕</Text>
                      </Pressable>
                    </View>
                  )}
                />
              </>
            )}
          </>
        )}

        {step === 3 && !report && (
          <>
            <Text style={styles.h2}>
              {card.length} archivos · {(bytes / 1e9).toFixed(2)} GB → {project!.name}
            </Text>
            <Text style={styles.muted}>
              {raws} RAW · {card.length - raws} JPG. Los RAW+JPG de la misma toma se unen en una foto.
            </Text>
            {!running && (
              <View style={styles.row}>
                <Button label="Atrás" onPress={() => setConfirmed(false)} />
                <Button label="Importar" onPress={start} primary />
              </View>
            )}
            {running && <Progress progress={progress} startedAt={startedAt} />}
          </>
        )}

        {report && (
          <>
            <Text style={styles.h2}>
              {report.result.imported} fotos nuevas{report.result.joined ? ` · ${report.result.joined} unidas` : ''}
              {report.result.failed ? ` · ${report.result.failed} fallidas` : ''}
            </Text>
            <Text style={styles.muted}>
              Import en la tablet {(report.importMs / 1000).toFixed(1)} s · subir al disco {(report.uploadMs / 1000).toFixed(1)} s (
              {(report.uploadedBytes / 1e9).toFixed(2)} GB) · total {((report.importMs + report.uploadMs) / 1000).toFixed(1)} s
            </Text>
            {report.result.errors.map((e, i) => (
              <Text key={i} style={styles.error}>
                {e}
              </Text>
            ))}
            <Button label="Abrir el proyecto" onPress={() => onOpenProject(project!)} />
          </>
        )}
      </ScrollView>
    </View>
  );
}

function Progress({ progress, startedAt }: { progress: ImportProgress | null; startedAt: number }) {
  const secs = ((Date.now() - startedAt) / 1000).toFixed(0);
  if (!progress) return <ActivityIndicator color="#e9c349" />;
  if (progress.step === 'upload') {
    return (
      <View style={styles.progress}>
        <Text style={styles.text}>Subiendo al disco</Text>
        <Bar value={progress.done / Math.max(1, progress.total)} />
        <Text style={styles.muted}>
          {progress.done}/{progress.total} archivos · {(progress.bytes / 1e9).toFixed(2)} GB · {secs} s
        </Text>
      </View>
    );
  }
  const s = progress.status;
  const total = Number(s.bytesTotal);
  const fraction = s.phase === 'copy' && total > 0 ? Number(s.bytesDone) / total : s.fileTotal ? s.fileIndex / s.fileTotal : 0;
  return (
    <View style={styles.progress}>
      <Text style={styles.text}>{PHASES[s.phase] ?? s.phase}</Text>
      <Bar value={fraction} />
      <Text style={styles.muted}>
        {s.fileTotal ? `${s.fileIndex}/${s.fileTotal} · ` : ''}
        {s.fileName} · {secs} s
      </Text>
      <Text style={styles.warn}>No cierres Lumik mientras se importa.</Text>
    </View>
  );
}

function Bar({ value }: { value: number }) {
  return (
    <View style={styles.bar}>
      <View style={[styles.barFill, { width: `${Math.min(1, Math.max(0, value)) * 100}%` }]} />
    </View>
  );
}

function Button({ label, onPress, disabled, primary }: { label: string; onPress: () => void; disabled?: boolean; primary?: boolean }) {
  return (
    <Pressable
      style={({ pressed }) => [styles.button, primary && styles.primary, pressed && styles.pressed, disabled && styles.off]}
      onPress={onPress}
      disabled={disabled}
    >
      <Text style={styles.buttonText}>{label}</Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#131313' },
  header: { flexDirection: 'row', alignItems: 'center', gap: 16, paddingHorizontal: 16, paddingVertical: 12, borderBottomWidth: 1, borderBottomColor: '#222' },
  back: { width: 48, height: 48, alignItems: 'center', justifyContent: 'center' },
  backText: { color: '#fff', fontSize: 24 },
  title: { color: '#fff', fontSize: 22, fontWeight: '600', flex: 1 },
  step: { color: '#9a9a9a', fontSize: 15 },
  content: { padding: 32, gap: 16, maxWidth: 820 },
  h2: { color: '#fff', fontSize: 20, fontWeight: '600' },
  text: { color: '#ddd', fontSize: 16 },
  muted: { color: '#9a9a9a', fontSize: 15 },
  warn: { color: '#e9c349', fontSize: 14 },
  error: { color: '#f87171', fontSize: 14 },
  item: { minHeight: 56, justifyContent: 'center', paddingHorizontal: 16, borderRadius: 10, backgroundColor: '#1f1f1f' },
  pressed: { opacity: 0.7 },
  off: { opacity: 0.4 },
  button: { alignSelf: 'flex-start', minHeight: 56, paddingHorizontal: 28, justifyContent: 'center', borderRadius: 16, backgroundColor: '#2c3a5c' },
  buttonText: { color: '#c7d6ff', fontSize: 16, fontWeight: '600' },
  primary: { backgroundColor: '#3b5bdb' },
  row: { flexDirection: 'row', gap: 12 },
  file: { flexDirection: 'row', alignItems: 'center', gap: 12, minHeight: 48, borderBottomWidth: 1, borderBottomColor: '#222' },
  remove: { width: 44, height: 44, alignItems: 'center', justifyContent: 'center', marginLeft: 'auto' },
  removeText: { color: '#9a9a9a', fontSize: 18 },
  progress: { gap: 10 },
  bar: { height: 8, borderRadius: 4, backgroundColor: '#2a2a2a', overflow: 'hidden' },
  barFill: { height: 8, backgroundColor: '#e9c349' },
});
