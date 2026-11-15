// Aviso animado mientras se leen los proyectos de un disco recién conectado
// (base y portada de cada uno). Del mismo estilo que `MoveSheet`: carpetas que
// saltan del disco a la cuadrícula de proyectos, con una barra de avance. No es
// un modal: el dashboard se puede usar mientras tanto.
import { useEffect } from 'react';
import { StyleSheet, Text, View } from 'react-native';
import Animated, {
  cancelAnimation,
  Easing,
  interpolate,
  type SharedValue,
  useAnimatedStyle,
  useReducedMotion,
  useSharedValue,
  withDelay,
  withRepeat,
  withTiming,
} from 'react-native-reanimated';
import Svg, { Path } from 'react-native-svg';

import { Icon } from './Icon';

const ORB = 48;
const SPAN = 112;
const HOP = 22;
const STAGE_W = ORB + SPAN;
/** Carpetas en vuelo a la vez, desfasadas. */
const FLYERS = 3;
const CYCLE = 1500;

function Flyer({ hop }: { hop: SharedValue<number> }) {
  const style = useAnimatedStyle(() => {
    const p = hop.value;
    return {
      opacity: interpolate(p, [0, 0.15, 0.8, 1], [0, 1, 1, 0]),
      transform: [
        { translateX: p * SPAN },
        { translateY: -Math.sin(Math.PI * p) * HOP },
        { rotate: `${interpolate(p, [0, 0.5, 1], [-14, 0, 12])}deg` },
        { scale: interpolate(p, [0, 0.2, 0.85, 1], [0.4, 1, 0.85, 0.4]) },
      ],
    };
  });
  return (
    <Animated.View style={[styles.flyer, style]} pointerEvents="none">
      <Icon name="folder" size={16} color="#e9c349" />
    </Animated.View>
  );
}

export function ScanBanner({ done, total }: { done: number; total: number }) {
  const still = useReducedMotion();
  const h0 = useSharedValue(0);
  const h1 = useSharedValue(0);
  const h2 = useSharedValue(0);
  const hops = [h0, h1, h2];
  const pulse = useSharedValue(0);
  const bump = useSharedValue(0);
  const progress = useSharedValue(total > 0 ? done / total : 0);

  useEffect(() => {
    if (still) return;
    hops.forEach((h, i) => {
      h.value = withDelay(
        (i * CYCLE) / FLYERS,
        withRepeat(withTiming(1, { duration: CYCLE, easing: Easing.inOut(Easing.cubic) }), -1, false),
      );
    });
    pulse.value = withRepeat(withTiming(1, { duration: 1400, easing: Easing.out(Easing.quad) }), -1, false);
    return () => {
      hops.forEach((h) => cancelAnimation(h));
      cancelAnimation(pulse);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [still]);

  // Cada proyecto leído hace "rebotar" la cuadrícula y avanza la barra.
  useEffect(() => {
    progress.value = withTiming(total > 0 ? done / total : 0, { duration: 300 });
    if (!still && done > 0) {
      bump.value = 1;
      bump.value = withTiming(0, { duration: 260 });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [done, total]);

  const ringStyle = useAnimatedStyle(() => ({
    opacity: interpolate(pulse.value, [0, 1], [0.55, 0]),
    transform: [{ scale: interpolate(pulse.value, [0, 1], [1, 1.5]) }],
  }));
  const gridStyle = useAnimatedStyle(() => ({ transform: [{ scale: 1 + bump.value * 0.12 }] }));
  const barStyle = useAnimatedStyle(() => ({ width: `${Math.max(4, progress.value * 100)}%` }));

  return (
    <View style={styles.banner} accessibilityRole="progressbar" accessibilityLabel={`Analizando disco: ${done} de ${total} proyectos`}>
      <View style={styles.stage}>
        <Svg width={STAGE_W} height={ORB + HOP} style={styles.arc}>
          <Path
            d={`M ${ORB / 2 + 22} ${HOP + ORB / 2 - 6} Q ${STAGE_W / 2} ${ORB / 2 - 8} ${STAGE_W - ORB / 2 - 22} ${HOP + ORB / 2 - 6}`}
            stroke="#34405e"
            strokeWidth={1.5}
            strokeDasharray="3 6"
            strokeLinecap="round"
            fill="none"
          />
        </Svg>
        <View>
          <Animated.View style={[styles.orb, styles.ring, ringStyle]} />
          <View style={styles.orb}>
            <Icon name="drive" size={22} color="#c7d6ff" />
          </View>
        </View>
        <Animated.View style={[styles.orb, styles.gridOrb, gridStyle]}>
          <Icon name="projects" size={22} color="#e9c349" />
        </Animated.View>
        {!still && hops.map((h, i) => <Flyer key={i} hop={h} />)}
      </View>
      <View style={styles.info}>
        <Text style={styles.title}>Analizando disco</Text>
        <Text style={styles.count}>
          {done} de {total} proyectos
        </Text>
        <View style={styles.track}>
          <Animated.View style={[styles.bar, barStyle]} />
        </View>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  banner: { flexDirection: 'row', alignItems: 'center', gap: 20, paddingHorizontal: 20, paddingTop: 14 + HOP / 2, paddingBottom: 14, borderRadius: 16, backgroundColor: '#1c2438' },
  stage: { width: STAGE_W, height: ORB, flexDirection: 'row', justifyContent: 'space-between', marginTop: HOP / 2 },
  arc: { position: 'absolute', left: 0, top: -HOP },
  orb: { width: ORB, height: ORB, borderRadius: ORB / 2, backgroundColor: '#2c3a5c', alignItems: 'center', justifyContent: 'center' },
  ring: { position: 'absolute', backgroundColor: 'transparent', borderWidth: 2, borderColor: '#c7d6ff' },
  gridOrb: { backgroundColor: '#3a3320' },
  flyer: { position: 'absolute', left: ORB / 2 - 8, top: ORB / 2 - 8 },
  info: { flex: 1, gap: 4 },
  title: { color: '#fff', fontSize: 16, fontWeight: '600' },
  count: { color: '#c7d6ff', fontSize: 14 },
  track: { height: 6, borderRadius: 3, backgroundColor: '#2c3a5c', overflow: 'hidden', marginTop: 4, maxWidth: 360 },
  bar: { height: 6, borderRadius: 3, backgroundColor: '#e9c349' },
});
