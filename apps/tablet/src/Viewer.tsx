// Visor del culling (D3/D5): pellizco = zoom libre que sigue al dedo, doble
// toque = acercar al punto tocado (o volver), un toque = modo limpio, doble
// toque con dos dedos = aprobar o quitar la aprobación, y sin zoom deslizar arriba aprueba, abajo
// quita la aprobación y a los lados cambia de foto. Todo corre en el hilo de
// UI (worklets); solo la decisión final cruza a JS.
//
// Las fotos van en una tira horizontal: cada una tiene un lugar fijo
// (`índice × (ancho + separación)`) y solo se monta la ventana alrededor de la
// actual. Cambiar de foto es mover la tira; al llegar, el índice nuevo ya
// coincide con la posición, así que no hay que reacomodar nada ni recargar.
import { Image } from 'expo-image';
import { useEffect, useState } from 'react';
import { StyleSheet, Text, View } from 'react-native';
import {
  GestureDetector,
  useExclusiveGestures,
  usePanGesture,
  usePinchGesture,
  useSimultaneousGestures,
  useTapGesture,
} from 'react-native-gesture-handler';
import Animated, { Easing, useAnimatedStyle, useSharedValue, withSpring, withTiming } from 'react-native-reanimated';
import { scheduleOnRN } from 'react-native-worklets';

const MAX_SCALE = 5;
const DOUBLE_TAP_SCALE = 2.5;
const SWIPE_DISTANCE = 110;
const SWIPE_VELOCITY = 900;
const GAP = 32;
const PAGE_MS = 240;
// Fotos montadas a cada lado de la actual.
const WINDOW = 1;
const SPRING = { damping: 22, stiffness: 260 };
const TAP_SLOP = 10;
const TAP_MS = 250;

export type ViewerItem = {
  uri: string;
  /** Grados (0 / 90 / 180 / 270) guardados en `project.db`. */
  rotation: number;
};

type Props = {
  items: ViewerItem[];
  index: number;
  onApprove: (approved: boolean) => void;
  onToggleApprove: () => void;
  onStep: (delta: 1 | -1) => void;
  onTap: () => void;
};

export function Viewer({ items, index, onApprove, onToggleApprove, onStep, onTap }: Props) {
  const [size, setSize] = useState({ width: 0, height: 0 });
  const width = useSharedValue(1);
  const height = useSharedValue(1);
  const count = items.length;

  // Posición de la tira: -índice × paso, más el arrastre del dedo.
  const offset = useSharedValue(0);
  const dragX = useSharedValue(0);
  const dragY = useSharedValue(0);
  const current = useSharedValue(index);

  const scale = useSharedValue(1);
  const tx = useSharedValue(0);
  const ty = useSharedValue(0);
  const startScale = useSharedValue(1);
  const startTx = useSharedValue(0);
  const startTy = useSharedValue(0);

  // Al cambiar de foto (por gesto, botón o el grid): sin zoom y la tira en su
  // lugar. Tras un deslizamiento la tira ya está ahí y esto no se nota.
  useEffect(() => {
    current.value = index;
    offset.value = -index * (width.value + GAP);
    scale.value = 1;
    tx.value = 0;
    ty.value = 0;
  }, [index, current, offset, width, scale, tx, ty]);

  const clampTranslation = (s: number) => {
    'worklet';
    const maxX = (width.value * (s - 1)) / 2;
    const maxY = (height.value * (s - 1)) / 2;
    tx.value = withSpring(Math.min(maxX, Math.max(-maxX, tx.value)), SPRING);
    ty.value = withSpring(Math.min(maxY, Math.max(-maxY, ty.value)), SPRING);
  };

  const page = (delta: 1 | -1) => {
    'worklet';
    const target = current.value + delta;
    // La tira llega a la foto vecina y el arrastre se funde en la posición.
    offset.value = offset.value + dragX.value;
    dragX.value = 0;
    offset.value = withTiming(
      -target * (width.value + GAP),
      { duration: PAGE_MS, easing: Easing.out(Easing.cubic) },
      (finished) => {
        if (finished) scheduleOnRN(onStep, delta);
      },
    );
  };

  const pinch = usePinchGesture({
    onBegin: () => {
      'worklet';
      startScale.value = scale.value;
      startTx.value = tx.value;
      startTy.value = ty.value;
    },
    onUpdate: (e) => {
      'worklet';
      const s = Math.min(MAX_SCALE, Math.max(1, startScale.value * e.scale));
      // El punto bajo los dedos se queda bajo los dedos.
      const fx = e.focalX - width.value / 2;
      const fy = e.focalY - height.value / 2;
      const ratio = s / startScale.value;
      tx.value = fx - (fx - startTx.value) * ratio;
      ty.value = fy - (fy - startTy.value) * ratio;
      scale.value = s;
    },
    onDeactivate: () => {
      'worklet';
      if (scale.value < 1.05) {
        scale.value = withSpring(1, SPRING);
        tx.value = withSpring(0, SPRING);
        ty.value = withSpring(0, SPRING);
      } else {
        clampTranslation(scale.value);
      }
    },
  });

  const pan = usePanGesture({
    maxPointers: 1,
    onBegin: () => {
      'worklet';
      startTx.value = tx.value;
      startTy.value = ty.value;
    },
    onUpdate: (e) => {
      'worklet';
      if (scale.value > 1) {
        tx.value = startTx.value + e.translationX;
        ty.value = startTy.value + e.translationY;
        return;
      }
      const vertical = Math.abs(e.translationY) > Math.abs(e.translationX);
      if (vertical) {
        dragX.value = 0;
        dragY.value = e.translationY;
      } else {
        // En los extremos la tira resiste: no hay foto que mostrar.
        const atEdge = (e.translationX > 0 && current.value === 0) || (e.translationX < 0 && current.value === count - 1);
        dragX.value = atEdge ? e.translationX / 3 : e.translationX;
        dragY.value = 0;
      }
    },
    onDeactivate: (e) => {
      'worklet';
      if (scale.value > 1) {
        clampTranslation(scale.value);
        return;
      }
      const vertical = Math.abs(e.translationY) > Math.abs(e.translationX);
      if (vertical) {
        const up = e.translationY < -SWIPE_DISTANCE || e.velocityY < -SWIPE_VELOCITY;
        const down = e.translationY > SWIPE_DISTANCE || e.velocityY > SWIPE_VELOCITY;
        if (up || down) scheduleOnRN(onApprove, up);
        dragY.value = withSpring(0, SPRING);
        return;
      }
      const next = e.translationX < -SWIPE_DISTANCE || e.velocityX < -SWIPE_VELOCITY;
      const prev = e.translationX > SWIPE_DISTANCE || e.velocityX > SWIPE_VELOCITY;
      if (next && current.value < count - 1) page(1);
      else if (prev && current.value > 0) page(-1);
      else dragX.value = withSpring(0, SPRING);
    },
  });

  const doubleTap = useTapGesture({
    numberOfTaps: 2,
    onActivate: (e) => {
      'worklet';
      if (scale.value > 1) {
        scale.value = withTiming(1);
        tx.value = withTiming(0);
        ty.value = withTiming(0);
      } else {
        const px = e.x - width.value / 2;
        const py = e.y - height.value / 2;
        scale.value = withTiming(DOUBLE_TAP_SCALE);
        tx.value = withTiming(-px * (DOUBLE_TAP_SCALE - 1));
        ty.value = withTiming(-py * (DOUBLE_TAP_SCALE - 1));
      }
    },
  });

  const twoFingerDoubleTap = useTapGesture({
    numberOfTaps: 2,
    minPointers: 2,
    onActivate: () => {
      'worklet';
      scheduleOnRN(onToggleApprove);
    },
  });

  // Un toque es dedo quieto y breve: sin límite, soltar un deslizamiento
  // también contaba como toque y alternaba el modo limpio al cambiar de foto.
  const singleTap = useTapGesture({
    numberOfTaps: 1,
    maxDistance: TAP_SLOP,
    maxDuration: TAP_MS,
    onActivate: () => {
      'worklet';
      scheduleOnRN(onTap);
    },
  });

  // En orden de prioridad: el de dos dedos no cuenta como doble toque de zoom,
  // y un toque solo espera a que el doble toque se descarte.
  const taps = useExclusiveGestures(twoFingerDoubleTap, doubleTap, singleTap);
  const gesture = useSimultaneousGestures(pinch, pan, taps);

  const stripStyle = useAnimatedStyle(() => ({
    transform: [{ translateX: offset.value + dragX.value }],
  }));
  const currentStyle = useAnimatedStyle(() => ({
    transform: [{ translateX: tx.value }, { translateY: ty.value + dragY.value * 0.5 }, { scale: scale.value }],
  }));
  const approveHint = useAnimatedStyle(() => ({
    opacity: Math.min(1, Math.max(0, -dragY.value / SWIPE_DISTANCE)),
  }));
  const rejectHint = useAnimatedStyle(() => ({
    opacity: Math.min(1, Math.max(0, dragY.value / SWIPE_DISTANCE)),
  }));

  const first = Math.max(0, index - WINDOW);
  const last = Math.min(count - 1, index + WINDOW);
  const slots = [];
  for (let i = first; i <= last; i++) slots.push(i);

  return (
    <GestureDetector gesture={gesture}>
      <View
        style={styles.root}
        onLayout={(e) => {
          const { width: w, height: h } = e.nativeEvent.layout;
          width.value = w;
          height.value = h;
          offset.value = -current.value * (w + GAP);
          setSize({ width: w, height: h });
        }}
      >
        {size.width > 0 && (
          <Animated.View style={[StyleSheet.absoluteFill, stripStyle]}>
            {slots.map((i) => (
              <Animated.View
                key={i}
                style={[
                  styles.slot,
                  { left: i * (size.width + GAP), width: size.width, height: size.height },
                  i === index && currentStyle,
                ]}
              >
                <Rotated item={items[i]} width={size.width} height={size.height} />
              </Animated.View>
            ))}
          </Animated.View>
        )}
        <Animated.View style={[styles.hint, styles.hintTop, approveHint]} pointerEvents="none">
          <Text style={styles.hintText}>Aprobar</Text>
        </Animated.View>
        <Animated.View style={[styles.hint, styles.hintBottom, rejectHint]} pointerEvents="none">
          <Text style={styles.hintText}>Quitar aprobación</Text>
        </Animated.View>
      </View>
    </GestureDetector>
  );
}

// La vista previa está sin girar (como en el disco): a 90/270 se dibuja en una
// caja con el ancho y el alto cambiados y se gira para que llene el visor.
function Rotated({ item, width, height }: { item: ViewerItem; width: number; height: number }) {
  const quarter = item.rotation === 90 || item.rotation === 270;
  const w = quarter ? height : width;
  const h = quarter ? width : height;
  return (
    <View
      style={{
        position: 'absolute',
        left: (width - w) / 2,
        top: (height - h) / 2,
        width: w,
        height: h,
        transform: [{ rotate: `${item.rotation}deg` }],
      }}
    >
      <Image source={{ uri: item.uri }} style={StyleSheet.absoluteFill} contentFit="contain" transition={0} />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, overflow: 'hidden', backgroundColor: '#0b0b0b' },
  slot: { position: 'absolute', top: 0 },
  hint: {
    position: 'absolute',
    alignSelf: 'center',
    paddingHorizontal: 20,
    paddingVertical: 10,
    borderRadius: 24,
    backgroundColor: 'rgba(19,19,19,0.85)',
  },
  hintTop: { top: 24 },
  hintBottom: { bottom: 24 },
  hintText: { color: '#e9c349', fontSize: 18, fontWeight: '600' },
});
