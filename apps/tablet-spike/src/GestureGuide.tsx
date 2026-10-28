// Guía de gestos del visor: como no hay botones de aprobar ni de quitar, una
// capa sobre la foto enseña cada gesto con dedos animados, paso a paso. La
// misma idea que driver.js en la web, hecha con vistas de React Native.
import { useEffect } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import Animated, {
  Easing,
  cancelAnimation,
  useAnimatedStyle,
  useSharedValue,
  withDelay,
  withRepeat,
  withSequence,
  withTiming,
} from 'react-native-reanimated';

type Motion = 'sides' | 'vertical' | 'twoFingerTap' | 'pinch' | 'tap';

const STEPS: { motion: Motion; title: string; text: string }[] = [
  { motion: 'sides', title: 'Cambiar de foto', text: 'Desliza a la izquierda o a la derecha.' },
  { motion: 'vertical', title: 'Aprobar', text: 'Desliza hacia arriba para aprobar y hacia abajo para quitar la aprobación.' },
  { motion: 'twoFingerTap', title: 'Aprobar más rápido', text: 'Toca dos veces con dos dedos para aprobar o quitar la aprobación.' },
  { motion: 'pinch', title: 'Revisar el detalle', text: 'Pellizca para acercar, o toca dos veces con un dedo donde quieras ver al 100 %.' },
  { motion: 'tap', title: 'Modo limpio', text: 'Toca una vez la foto para ocultar la tira, las estrellas y las etiquetas. Otro toque las regresa.' },
];

type Props = { step: number; onStep: (step: number) => void; onClose: () => void };

export function GestureGuide({ step, onStep, onClose }: Props) {
  const current = STEPS[step];
  const last = step === STEPS.length - 1;

  return (
    <View style={styles.overlay}>
      <Fingers key={step} motion={current.motion} />
      <View style={styles.card}>
        <Text style={styles.count}>
          {step + 1} de {STEPS.length}
        </Text>
        <Text style={styles.title}>{current.title}</Text>
        <Text style={styles.text}>{current.text}</Text>
        <View style={styles.buttons}>
          <Pressable style={styles.skip} onPress={onClose} hitSlop={8}>
            <Text style={styles.skipText}>Omitir</Text>
          </Pressable>
          <Pressable style={styles.next} onPress={() => (last ? onClose() : onStep(step + 1))}>
            <Text style={styles.nextText}>{last ? 'Entendido' : 'Siguiente'}</Text>
          </Pressable>
        </View>
      </View>
    </View>
  );
}

const TRAVEL = 140;

// Uno o dos dedos que repiten el gesto en bucle.
function Fingers({ motion }: { motion: Motion }) {
  const t = useSharedValue(0);

  useEffect(() => {
    const ease = { duration: 700, easing: Easing.inOut(Easing.cubic) };
    t.value = 0;
    if (motion === 'tap') {
      t.value = withRepeat(withSequence(withTiming(1, { duration: 140 }), withTiming(0, { duration: 140 }), withDelay(900, withTiming(0, { duration: 0 }))), -1);
    } else if (motion === 'twoFingerTap') {
      // Dos toques seguidos y una pausa.
      t.value = withRepeat(
        withSequence(withTiming(1, { duration: 120 }), withTiming(0, { duration: 120 }), withTiming(1, { duration: 120 }), withTiming(0, { duration: 120 }), withDelay(700, withTiming(0, { duration: 0 }))),
        -1,
      );
    } else {
      t.value = withRepeat(withSequence(withTiming(1, ease), withDelay(250, withTiming(-1, { ...ease, duration: 1400 })), withTiming(0, ease)), -1);
    }
    return () => cancelAnimation(t);
  }, [motion, t]);

  const first = useAnimatedStyle(() => {
    switch (motion) {
      case 'sides':
        return { transform: [{ translateX: -t.value * TRAVEL }] };
      case 'vertical':
        return { transform: [{ translateY: -t.value * TRAVEL }] };
      case 'twoFingerTap':
        return { transform: [{ translateX: -28 }, { scale: 1 - t.value * 0.25 }], opacity: 0.6 + t.value * 0.4 };
      case 'pinch':
        return { transform: [{ translateX: -30 - Math.abs(t.value) * 70 }, { translateY: 30 + Math.abs(t.value) * 70 }] };
      case 'tap':
        return { transform: [{ scale: 1 - t.value * 0.25 }], opacity: 0.6 + t.value * 0.4 };
    }
  });
  const second = useAnimatedStyle(() => {
    switch (motion) {
      case 'twoFingerTap':
        return { transform: [{ translateX: 28 }, { scale: 1 - t.value * 0.25 }], opacity: 0.6 + t.value * 0.4 };
      case 'pinch':
        return { transform: [{ translateX: 30 + Math.abs(t.value) * 70 }, { translateY: -30 - Math.abs(t.value) * 70 }] };
      default:
        return { opacity: 0 };
    }
  });

  return (
    <View style={styles.stage} pointerEvents="none">
      <Animated.View style={[styles.finger, first]} />
      <Animated.View style={[styles.finger, second]} />
    </View>
  );
}

const styles = StyleSheet.create({
  overlay: {
    position: 'absolute',
    top: 0,
    bottom: 0,
    left: 0,
    right: 0,
    backgroundColor: 'rgba(0,0,0,0.6)',
    alignItems: 'center',
    justifyContent: 'center',
  },
  // Los dedos van en la mitad de arriba, lejos de la tarjeta.
  stage: { position: 'absolute', top: 0, bottom: '45%', left: 0, right: 0, alignItems: 'center', justifyContent: 'center' },
  finger: {
    position: 'absolute',
    width: 56,
    height: 56,
    borderRadius: 28,
    backgroundColor: 'rgba(233,195,73,0.35)',
    borderWidth: 3,
    borderColor: '#e9c349',
  },
  card: {
    position: 'absolute',
    bottom: 24,
    width: 420,
    padding: 20,
    gap: 6,
    borderRadius: 14,
    backgroundColor: '#1d1d1d',
  },
  count: { color: '#9a9a9a', fontSize: 13 },
  title: { color: '#fff', fontSize: 20, fontWeight: '600' },
  text: { color: '#ddd', fontSize: 16, lineHeight: 22 },
  buttons: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', marginTop: 10 },
  skip: { minHeight: 44, justifyContent: 'center' },
  skipText: { color: '#9a9a9a', fontSize: 15 },
  next: { minHeight: 48, paddingHorizontal: 22, justifyContent: 'center', borderRadius: 24, backgroundColor: '#e9c349' },
  nextText: { color: '#131313', fontSize: 16, fontWeight: '700' },
});
