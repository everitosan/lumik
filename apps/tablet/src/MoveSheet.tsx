// Mandar un proyecto a la papelera y restaurarlo, con la misma hoja animada
// (spec trash-bin): una carpeta salta de un ícono al otro mientras dura el
// movimiento, que en un disco USB puede tardar decenas de segundos. Al mandar,
// la hoja pide confirmación primero; al restaurar, empieza moviendo.
import { useEffect, useRef, useState } from 'react';
import { Modal, Pressable, StyleSheet, Text, View } from 'react-native';
import Animated, {
  cancelAnimation,
  Easing,
  interpolate,
  useAnimatedStyle,
  useReducedMotion,
  useSharedValue,
  withRepeat,
  withSequence,
  withSpring,
  withTiming,
} from 'react-native-reanimated';
import Svg, { Path } from 'react-native-svg';

import { Icon, type IconName } from './Icon';

type Direction = 'trash' | 'restore';
type Phase = 'confirm' | 'working' | 'done' | 'error';

type Props = {
  direction: Direction;
  /** Lo que se mueve: el nombre del proyecto o «3 elementos». */
  name: string;
  /** Pedir confirmación antes de mover (mandar a la papelera). */
  confirm?: boolean;
  /** Hace el movimiento. `false`: otro diálogo se encarga (candado, colisión) y la hoja se cierra. */
  run: () => Promise<boolean>;
  onClose: () => void;
};

const STAGE_W = 320;
const ORB = 76;
const HOP = 44;
/** Distancia que recorre la carpeta, de centro a centro de los íconos. */
const SPAN = STAGE_W - ORB;

const TEXT: Record<Direction, { ask: string; working: string; done: string; button: string }> = {
  trash: { ask: 'Mover a la papelera', working: 'Moviendo a la papelera', done: 'En la papelera', button: 'Mover' },
  restore: { ask: 'Restaurar', working: 'Restaurando', done: 'Restaurado', button: 'Restaurar' },
};

export function MoveSheet({ direction, name, confirm = false, run, onClose }: Props) {
  const still = useReducedMotion();
  const [phase, setPhase] = useState<Phase>(confirm ? 'confirm' : 'working');
  const [error, setError] = useState<string | null>(null);
  const [elapsed, setElapsed] = useState(0);
  const started = useRef(false);

  const hop = useSharedValue(0); // 0 → 1 en cada salto
  const bob = useSharedValue(0); // vaivén del ícono de origen mientras espera
  const land = useSharedValue(1); // escala del destino
  const check = useSharedValue(0); // 0 → 1 al terminar

  const go = async () => {
    if (started.current) return;
    started.current = true;
    setPhase('working');
    try {
      const ok = await run();
      if (!ok) {
        onClose();
        return;
      }
      setPhase('done');
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPhase('error');
    }
  };

  // Sin confirmación, el movimiento empieza al abrir.
  useEffect(() => {
    if (!confirm) void go();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    cancelAnimation(hop);
    cancelAnimation(bob);
    if (phase === 'confirm' && !still) {
      bob.value = withRepeat(withTiming(1, { duration: 900, easing: Easing.inOut(Easing.sin) }), -1, true);
    } else {
      bob.value = withTiming(0, { duration: 150 });
    }
    if (phase === 'working') {
      hop.value = 0;
      if (!still) hop.value = withRepeat(withTiming(1, { duration: 1100, easing: Easing.inOut(Easing.cubic) }), -1, false);
      setElapsed(0);
      const t0 = Date.now();
      const timer = setInterval(() => setElapsed(Math.floor((Date.now() - t0) / 1000)), 1000);
      return () => clearInterval(timer);
    }
    if (phase === 'done') {
      hop.value = 0;
      land.value = withSequence(withTiming(1.15, { duration: 140 }), withSpring(1));
      check.value = withSpring(1, { damping: 12, stiffness: 180 });
      const timer = setTimeout(onClose, 900);
      return () => clearTimeout(timer);
    }
    return undefined;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase, still]);

  const travelerStyle = useAnimatedStyle(() => {
    const p = hop.value;
    return {
      opacity: interpolate(p, [0, 0.12, 0.8, 1], [0, 1, 1, 0]),
      transform: [
        { translateX: p * SPAN },
        { translateY: -Math.sin(Math.PI * p) * HOP },
        { rotate: `${interpolate(p, [0, 0.5, 1], [-12, 0, 14])}deg` },
        { scale: interpolate(p, [0, 0.15, 0.8, 1], [0.6, 1, 0.9, 0.5]) },
      ],
    };
  });
  const sourceStyle = useAnimatedStyle(() => ({
    transform: [{ translateY: -bob.value * 5 }, { scale: 1 - interpolate(hop.value, [0, 0.1, 0.3], [0, 0.08, 0]) }],
  }));
  const targetStyle = useAnimatedStyle(() => ({
    transform: [{ scale: land.value * (1 + interpolate(hop.value, [0.8, 0.92, 1], [0, 0.12, 0])) }],
  }));
  const checkStyle = useAnimatedStyle(() => ({ opacity: check.value, transform: [{ scale: 0.4 + check.value * 0.6 }] }));

  const from: IconName = direction === 'trash' ? 'folder' : 'trash';
  const to: IconName = direction === 'trash' ? 'trash' : 'folder';
  const accent = direction === 'trash' ? styles.trashOrb : styles.folderOrb;
  const accentColor = direction === 'trash' ? '#ffb4ab' : '#c7d6ff';
  const text = TEXT[direction];

  const title =
    phase === 'confirm' ? text.ask : phase === 'working' ? `${text.working}…` : phase === 'done' ? text.done : 'No se pudo completar';

  return (
    <Modal transparent animationType="fade" onRequestClose={() => (phase === 'confirm' || phase === 'error' ? onClose() : undefined)}>
      <View style={styles.scrim}>
        <View style={styles.sheet} accessibilityLiveRegion="polite">
          <View style={styles.stage}>
            {/* El arco del salto, por encima de los íconos (el lienzo sube HOP para no recortarlo). */}
            <Svg width={STAGE_W} height={ORB + HOP} style={styles.arc}>
              <Path
                d={`M ${ORB / 2 + 30} ${HOP + ORB / 2 - 10} Q ${STAGE_W / 2} ${ORB / 2 - 14} ${STAGE_W - ORB / 2 - 30} ${HOP + ORB / 2 - 10}`}
                stroke="#3a3a3a"
                strokeWidth={2}
                strokeDasharray="4 8"
                strokeLinecap="round"
                fill="none"
              />
            </Svg>
            <Animated.View style={[styles.orb, sourceStyle]}>
              <Icon name={from} size={34} color="#ddd" />
            </Animated.View>
            <Animated.View style={[styles.orb, accent, targetStyle]}>
              {phase === 'done' ? (
                <Animated.View style={checkStyle}>
                  <Icon name="check" size={36} color="#86efac" strokeWidth={2.5} />
                </Animated.View>
              ) : (
                <Icon name={to} size={34} color={accentColor} />
              )}
            </Animated.View>
            {phase === 'working' && (
              <Animated.View style={[styles.traveler, travelerStyle]} pointerEvents="none">
                <Icon name="folder" size={26} color="#e9c349" />
              </Animated.View>
            )}
          </View>

          <Text style={styles.title}>{title}</Text>
          <Text style={styles.name} numberOfLines={2}>
            {name}
          </Text>
          {phase === 'confirm' && direction === 'trash' && <Text style={styles.hint}>Podrás restaurarlo cuando quieras.</Text>}
          {phase === 'working' && <Text style={styles.hint}>{elapsed >= 3 ? `${elapsed} s · ` : ''}No desconectes el disco.</Text>}
          {phase === 'error' && <Text style={styles.error}>{error}</Text>}

          {(phase === 'confirm' || phase === 'error') && (
            <View style={styles.buttons}>
              <Pressable style={styles.textButton} onPress={onClose}>
                <Text style={styles.textButtonText}>{phase === 'error' ? 'Cerrar' : 'Cancelar'}</Text>
              </Pressable>
              {phase === 'confirm' && (
                <Pressable
                  style={({ pressed }) => [direction === 'trash' ? styles.danger : styles.primary, pressed && styles.pressed]}
                  onPress={() => void go()}
                >
                  <Icon name={to} size={18} color={direction === 'trash' ? '#fff' : '#c7d6ff'} />
                  <Text style={direction === 'trash' ? styles.dangerText : styles.primaryText}>{text.button}</Text>
                </Pressable>
              )}
            </View>
          )}
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  scrim: { flex: 1, backgroundColor: 'rgba(0,0,0,0.6)', alignItems: 'center', justifyContent: 'center' },
  sheet: { width: 440, paddingHorizontal: 28, paddingTop: 36, paddingBottom: 24, borderRadius: 24, backgroundColor: '#1d1d1d', alignItems: 'center', gap: 6 },
  stage: { width: STAGE_W, height: ORB, flexDirection: 'row', justifyContent: 'space-between', marginTop: HOP, marginBottom: 20 },
  orb: { width: ORB, height: ORB, borderRadius: ORB / 2, backgroundColor: '#262626', alignItems: 'center', justifyContent: 'center' },
  trashOrb: { backgroundColor: '#3b1a1a' },
  folderOrb: { backgroundColor: '#1f2a44' },
  arc: { position: 'absolute', left: 0, top: -HOP },
  traveler: { position: 'absolute', left: ORB / 2 - 13, top: ORB / 2 - 13 },
  title: { color: '#fff', fontSize: 22, fontWeight: '600', textAlign: 'center' },
  name: { color: '#ddd', fontSize: 16, textAlign: 'center' },
  hint: { color: '#9a9a9a', fontSize: 14, textAlign: 'center', marginTop: 4 },
  error: { color: '#ffb4ab', fontSize: 14, textAlign: 'center', marginTop: 4 },
  buttons: { flexDirection: 'row', justifyContent: 'center', alignItems: 'center', gap: 12, marginTop: 20 },
  textButton: { minHeight: 48, paddingHorizontal: 20, justifyContent: 'center' },
  textButtonText: { color: '#c7d6ff', fontSize: 15, fontWeight: '600' },
  danger: { minHeight: 48, paddingHorizontal: 24, flexDirection: 'row', alignItems: 'center', gap: 8, borderRadius: 24, backgroundColor: '#7f1d1d' },
  dangerText: { color: '#fff', fontSize: 15, fontWeight: '600' },
  primary: { minHeight: 48, paddingHorizontal: 24, flexDirection: 'row', alignItems: 'center', gap: 8, borderRadius: 24, backgroundColor: '#2c3a5c' },
  primaryText: { color: '#c7d6ff', fontSize: 15, fontWeight: '600' },
  pressed: { opacity: 0.7 },
});
