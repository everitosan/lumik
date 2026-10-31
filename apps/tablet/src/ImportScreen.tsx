// Importar (Android): pantalla completa paso a paso, desde un proyecto, como el
// wizard del desktop (spec `tablet-import`): 1 archivos · 2 ajustes · 3 importar.
// Los archivos se eligen con el selector del sistema (selección múltiple) en una
// lista a la que se agregan y de la que se quitan. Los ajustes salen de los de la
// tablet y valen solo para esta importación; la entrega al aprobar es del proyecto.
import type { ImportSettings } from '@lumik/core-native';
import { useEffect, useRef, useState } from 'react';
import { ActivityIndicator, BackHandler, FlatList, Pressable, ScrollView, StyleSheet, Switch, Text, View } from 'react-native';

import Saf from '../modules/saf/src/LumikSafModule';
import type { SafEntry } from '../modules/saf/src/LumikSaf.types';
import { DeliveryChoice } from './delivery';
import { formatBytes, volumeOf } from './diskName';
import { Icon } from './Icon';
import { importCard, type ImportProgress, type ImportReport, isImportable } from './importer';
import { importSettings } from './prefs';
import { type DiskProject, ProjectSession } from './session';

type Props = {
  project: DiskProject;
  /** Nombre del proyecto para mostrar. */
  title: string;
  /** Volver al proyecto (cancelando, o al terminar para cullar lo nuevo). */
  onClose: () => void;
};

type Step = 1 | 2 | 3;

const PHASES: Record<string, string> = {
  copy: 'Copiando de la tarjeta',
  metadata: 'Renombrando y firmando',
  move: 'Ordenando en el proyecto',
  register: 'Registrando y creando miniaturas',
  complete: 'Registrando y creando miniaturas',
  previews: 'Creando vistas previas',
};

// Misma regla que `sanitize_file_name` del núcleo: letras, dígitos, `-` y `_`.
const slug = (name: string) => name.trim().replace(/[^\p{L}\p{N}_-]/gu, '_');

/** Cómo quedaría un archivo renombrado: `2026-10-30__14-30-22__Boda.cr2`. */
function renameExample(title: string) {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}__${p(d.getHours())}-${p(d.getMinutes())}-${p(d.getSeconds())}__${slug(title)}.cr2`;
}

export function ImportScreen({ project, title, onClose }: Props) {
  const [card, setCard] = useState<SafEntry[]>([]);
  const [step, setStep] = useState<Step>(1);
  const [ignored, setIgnored] = useState(0);
  const [settings, setSettings] = useState<ImportSettings>(importSettings);
  const [delivery, setDelivery] = useState<string | null>(null);
  // Cambio de entrega que afecta a fotos ya aprobadas: se pregunta antes de aplicarlo.
  const [pendingDelivery, setPendingDelivery] = useState<{ mode: string; affected: number } | null>(null);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [startedAt, setStartedAt] = useState(0);
  // Abierta desde el paso de ajustes hasta que termina el import o se sale.
  const sessionRef = useRef<ProjectSession | null>(null);

  const running = busy && step === 3;

  const closeSession = () => {
    try {
      sessionRef.current?.close();
    } catch (e) {
      console.warn(`Al cerrar el proyecto: ${String(e)}`);
    }
    sessionRef.current = null;
  };

  const leave = () => {
    closeSession();
    onClose();
  };

  // Atrás: un paso hacia atrás; del primero, al proyecto. Mientras se importa, nada.
  useEffect(() => {
    const sub = BackHandler.addEventListener('hardwareBackPress', () => {
      if (running) return true;
      if (report || step === 1) leave();
      else setStep((s) => (s - 1) as Step);
      return true;
    });
    return () => sub.remove();
  });

  // Al salir de la pantalla por otro camino, el candado no se queda tomado.
  useEffect(() => () => closeSession(), []);

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
      console.warn(`Import falló: ${String(e)}`);
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  // Paso 2: se abre el proyecto (copia de trabajo y candado) para leer su entrega.
  const toSettings = async () => {
    setError(null);
    setBusy(true);
    try {
      if (!sessionRef.current) {
        const session = await ProjectSession.open(project, () => {}, () => {});
        if (session.readOnly) {
          session.close();
          throw new Error(`${session.readOnly.installName} tiene este proyecto. Ábrelo y toma el control antes de importar.`);
        }
        sessionRef.current = session;
      }
      setDelivery(sessionRef.current.delivery());
      setStep(2);
    } catch (e) {
      console.warn(`Import falló: ${String(e)}`);
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const chooseDelivery = (mode: string) => {
    const session = sessionRef.current;
    if (!session || mode === delivery) return;
    try {
      const plan = session.planDelivery(mode);
      const affected = plan.deliver + plan.returnBack;
      if (affected > 0) {
        setPendingDelivery({ mode, affected });
        return;
      }
      session.setDelivery(mode, false);
      setDelivery(mode);
    } catch (e) {
      setError(String(e));
    }
  };

  const applyDelivery = (toApproved: boolean) => {
    const session = sessionRef.current;
    if (!session || !pendingDelivery) return;
    try {
      session.setDelivery(pendingDelivery.mode, toApproved);
      setDelivery(pendingDelivery.mode);
    } catch (e) {
      setError(String(e));
    } finally {
      setPendingDelivery(null);
    }
  };

  const start = async () => {
    const session = sessionRef.current;
    if (!session || card.length === 0) return;
    setError(null);
    setBusy(true);
    setStartedAt(Date.now());
    try {
      const r = await importCard(session, card, setProgress, settings);
      // Diagnóstico: los errores también al log de Android (logcat, ReactNativeJS).
      console.warn(`Import: ${r.result.imported} nuevas, ${r.result.failed} fallidas de ${card.length}`);
      r.result.errors.forEach((e) => console.warn(`Import error: ${e}`));
      setReport(r);
    } catch (e) {
      console.warn(`Import falló: ${String(e)}`);
      setError(String(e));
    } finally {
      // El import terminó: el candado se suelta (o se queda con `pending` si algo no llegó).
      closeSession();
      setBusy(false);
    }
  };

  const raws = card.filter((f) => !/\.(jpe?g|heic)$/i.test(f.name)).length;
  const bytes = card.reduce((s, f) => s + f.size, 0);
  // Como el desktop: no se empieza si no cabe en el disco del proyecto (null si Android no lo da).
  const free = Saf.volumes().find((v) => v.uuid === volumeOf(project.tree))?.freeBytes ?? null;
  const fits = free === null || bytes <= free;

  return (
    <View style={styles.root}>
      <View style={styles.header}>
        <Pressable style={styles.back} onPress={() => (step === 1 || report ? leave() : setStep((s) => (s - 1) as Step))} disabled={running} hitSlop={8}>
          <Icon name="chevron-left" size={26} />
        </Pressable>
        <Text style={styles.title}>Importar a {title}</Text>
        <Text style={styles.step}>{report ? 'Listo' : `${step} de 3`}</Text>
      </View>

      <ScrollView contentContainerStyle={styles.content}>
        {error && <Text style={styles.error}>{error}</Text>}

        {step === 1 && (
          <>
            <Text style={styles.h2}>Origen</Text>
            <Text style={styles.muted}>
              Elige las fotos de la tarjeta. En el explorador puedes marcar varias, o usar "Seleccionar todo" en una carpeta.
            </Text>
            <View style={styles.row}>
              <Button label={card.length ? 'Agregar más' : 'Elegir archivos'} onPress={pickFiles} disabled={busy} />
              {card.length > 0 && <Button label="Siguiente" onPress={toSettings} primary disabled={busy} />}
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
                        <Icon name="x" size={18} color="#9a9a9a" />
                      </Pressable>
                    </View>
                  )}
                />
              </>
            )}
          </>
        )}

        {step === 2 && (
          <>
            <Text style={styles.h2}>Ajustes de esta importación</Text>
            <Text style={styles.muted}>Vienen de Ajustes; cambiarlos aquí no cambia los de la tablet.</Text>
            <Toggle
              label="Renombrar por fecha de captura"
              detail={settings.rename ? `p. ej. ${renameExample(title)}` : 'Las fotos conservan el nombre de la cámara.'}
              value={settings.rename}
              onChange={(rename) => setSettings((s) => ({ ...s, rename }))}
            />
            <Toggle
              label="Firmar las fotos"
              detail={
                settings.sign
                  ? settings.author
                    ? `Autor: ${settings.author}, en un .xmp junto a cada foto.`
                    : 'Sin autor en Ajustes: solo se escribe la descripción del proyecto.'
                  : 'No se escribe ningún .xmp.'
              }
              value={settings.sign}
              onChange={(sign) => setSettings((s) => ({ ...s, sign }))}
            />

            <Text style={[styles.h2, styles.section]}>Del proyecto: al aprobar, entregar directamente</Text>
            <DeliveryChoice value={delivery} onChange={chooseDelivery} disabled={!!pendingDelivery} />
            {pendingDelivery && (
              <View style={styles.ask}>
                <Text style={styles.text}>
                  {pendingDelivery.affected} {pendingDelivery.affected === 1 ? 'foto aprobada cambia' : 'fotos aprobadas cambian'} con esta opción. ¿Aplicarla
                  también a ellas?
                </Text>
                <View style={styles.row}>
                  <Button label="Solo a las siguientes" onPress={() => applyDelivery(false)} />
                  <Button label="También a las aprobadas" onPress={() => applyDelivery(true)} primary />
                </View>
              </View>
            )}

            <View style={styles.row}>
              <Button label="Atrás" onPress={() => setStep(1)} />
              <Button label="Siguiente" onPress={() => setStep(3)} primary disabled={!!pendingDelivery} />
            </View>
          </>
        )}

        {step === 3 && !report && (
          <>
            <Text style={styles.h2}>
              {card.length} archivos · {(bytes / 1e9).toFixed(2)} GB → {title}
            </Text>
            <Text style={styles.muted}>
              {raws} RAW · {card.length - raws} JPG. Los RAW+JPG de la misma toma se unen en una foto.
            </Text>
            <Text style={styles.muted}>
              {settings.rename ? 'Se renombran por fecha' : 'Conservan el nombre de la cámara'} ·{' '}
              {settings.sign ? 'con firma' : 'sin firma'}
            </Text>
            {free !== null && (
              <Text style={fits ? styles.muted : styles.warn}>
                {fits
                  ? `Espacio libre en el disco: ${formatBytes(free)}`
                  : `Falta espacio en el disco: necesitas ${formatBytes(bytes)} y quedan ${formatBytes(free)}.`}
              </Text>
            )}
            {!running && (
              <View style={styles.row}>
                <Button label="Atrás" onPress={() => setStep(2)} />
                <Button label="Importar" onPress={start} primary disabled={!fits} />
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
              Copia y proceso {(report.importMs / 1000).toFixed(1)} s · ordenar en el disco {(report.uploadMs / 1000).toFixed(1)} s (
              {(report.uploadedBytes / 1e9).toFixed(2)} GB) · total {((report.importMs + report.uploadMs) / 1000).toFixed(1)} s
            </Text>
            {report.result.errors.map((e, i) => (
              <Text key={i} style={styles.error}>
                {e}
              </Text>
            ))}
            <Button label="Ir al proyecto" onPress={leave} primary />
          </>
        )}
      </ScrollView>
    </View>
  );
}

function Toggle({ label, detail, value, onChange }: { label: string; detail: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <Pressable style={styles.toggle} onPress={() => onChange(!value)}>
      <View style={styles.toggleText}>
        <Text style={styles.text}>{label}</Text>
        <Text style={styles.muted}>{detail}</Text>
      </View>
      <Switch value={value} onValueChange={onChange} trackColor={{ true: '#3b5bdb', false: '#333' }} thumbColor="#fff" />
    </Pressable>
  );
}

function Progress({ progress, startedAt }: { progress: ImportProgress | null; startedAt: number }) {
  const secs = ((Date.now() - startedAt) / 1000).toFixed(0);
  if (!progress) return <ActivityIndicator color="#e9c349" />;
  if (progress.step === 'upload') {
    return (
      <View style={styles.progress}>
        <Text style={styles.text}>Ordenando en el disco</Text>
        <Bar value={progress.done / Math.max(1, progress.total)} />
        <Text style={styles.muted}>
          {progress.done}/{progress.total} archivos · {(progress.bytes / 1e9).toFixed(2)} GB · {secs} s
        </Text>
      </View>
    );
  }
  const s = progress.status;
  const total = Number(s.bytesTotal);
  const done = Number(s.bytesDone);
  const fraction = s.phase === 'copy' && total > 0 ? done / total : s.fileTotal ? s.fileIndex / s.fileTotal : 0;
  // Tiempo restante de la copia, al ritmo que lleva (lo demás es corto).
  const elapsed = (Date.now() - startedAt) / 1000;
  const rate = elapsed > 2 ? done / elapsed : 0;
  const left = s.phase === 'copy' && rate > 0 ? (total - done) / rate : null;
  return (
    <View style={styles.progress}>
      <Text style={styles.text}>{PHASES[s.phase] ?? s.phase}</Text>
      <Bar value={fraction} />
      <Text style={styles.muted}>
        {s.fileTotal ? `${s.fileIndex}/${s.fileTotal} · ` : ''}
        {s.fileName} · {secs} s
        {left !== null ? ` · quedan ~${formatSecs(left)} de copia (${(rate / 1e6).toFixed(0)} MB/s)` : ''}
      </Text>
      <Text style={styles.warn}>No cierres Lumik mientras se importa.</Text>
    </View>
  );
}

const formatSecs = (s: number) => (s >= 90 ? `${Math.round(s / 60)} min` : `${Math.max(1, Math.round(s))} s`);

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
  progress: { gap: 10 },
  bar: { height: 8, borderRadius: 4, backgroundColor: '#2a2a2a', overflow: 'hidden' },
  barFill: { height: 8, backgroundColor: '#e9c349' },
  section: { marginTop: 16 },
  toggle: { flexDirection: 'row', alignItems: 'center', gap: 16, minHeight: 64, paddingHorizontal: 16, paddingVertical: 10, borderRadius: 12, backgroundColor: '#1f1f1f' },
  toggleText: { flex: 1, gap: 2 },
  ask: { gap: 12, padding: 16, borderRadius: 12, backgroundColor: '#2a2414' },
});
