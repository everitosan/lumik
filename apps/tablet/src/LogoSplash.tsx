// Splash de arranque (spec `app-splash`, design D5): sigue al splash del sistema
// (el libro cerrado, `assets/splash-book.png`) en el mismo lugar y tamaño, abre el
// libro sobre un resplandor, saca el nombre letra por letra y se desvanece dejando
// ver la app. La geometría y la línea de tiempo son las del desktop:
// `@lumik/ui/brand` (nunca la raíz de `@lumik/ui`).
import {
  BOOK_CLOSED,
  BOOK_HINGE,
  bookEdgePlane,
  COVER_OVERLAP,
  LOGO_BODY,
  LOGO_PAGES,
  OUT_BACK_OVERSHOOT,
  SPLASH_BACKGROUND,
  SPLASH_FRAME,
  SPLASH_GLOW,
  SPLASH_TIMELINE,
  type SplashStep,
  WORDMARK,
} from '@lumik/ui/brand';
import * as SplashScreen from 'expo-splash-screen';
import { type ComponentClass, useState } from 'react';
import { StyleSheet } from 'react-native';
import Animated, {
  Easing,
  ReduceMotion,
  runOnJS,
  type SharedValue,
  useAnimatedProps,
  useAnimatedStyle,
  useReducedMotion,
  useSharedValue,
  withDelay,
  withTiming,
} from 'react-native-reanimated';
import Svg, { Circle, ClipPath, Defs, G, type GProps, LinearGradient, Path, RadialGradient, Rect, Stop } from 'react-native-svg';

/**
 * Lado de `SPLASH_FRAME` en dp: el tamaño con que el splash del sistema muestra
 * `assets/splash-book.png` (el mismo cuadro). Si cambia, cambiar `imageWidth`
 * del plugin `expo-splash-screen` en `app.config.ts`.
 */
export const SPLASH_IMAGE_DP = 300;

const F = SPLASH_FRAME;
const { x: hx, y: hy } = BOOK_HINGE;
const { dot } = WORDMARK;
const T = SPLASH_TIMELINE;
const PAGES_DONE = T.pages[T.pages.length - 1].to;

const easing: Record<SplashStep['easing'], (x: number) => number> = {
  linear: Easing.linear,
  'out-cubic': Easing.out(Easing.cubic),
  'out-back': Easing.out(Easing.back(OUT_BACK_OVERSHOOT)),
};

/**
 * Lleva `value` de 0 a 1 según un paso de la línea de tiempo. Con "reducir
 * movimiento", Reanimated saltaría al final (y el splash se quitaría sin verse):
 * este componente ya lo resuelve mostrando el logo quieto, así que no lo aplica.
 */
function play(value: SharedValue<number>, step: SplashStep, done?: () => void) {
  value.value = withDelay(
    step.from * 1000,
    withTiming(1, { duration: (step.to - step.from) * 1000, easing: easing[step.easing], reduceMotion: ReduceMotion.Never }, (finished) => {
      if (finished && done) runOnJS(done)();
    }),
    ReduceMotion.Never,
  );
}

// Con Fabric, Reanimated manda los props animados directo al componente nativo, y el
// `G` nativo de react-native-svg solo entiende `matrix` ([a, b, c, d, e, f], como
// `matrix()` de SVG): `rotation`, `scale` o `translateY` no llegan. Por eso cada
// transformación se anima como matriz, y la del primer cuadro va en `transform`.
type Matrix = [number, number, number, number, number, number];

/** Giro de `deg` grados (horario) alrededor de (cx, cy). */
function rotateAbout(deg: number, cx: number, cy: number): Matrix {
  'worklet';
  const a = (deg * Math.PI) / 180;
  const [c, s] = [Math.cos(a), Math.sin(a)];
  return [c, s, -s, c, cx - c * cx + s * cy, cy - s * cx - c * cy];
}

/** Escala `k` alrededor de (cx, cy); nunca 0 exacto, que el nativo no invierte. */
function scaleAbout(k: number, cx: number, cy: number): Matrix {
  'worklet';
  const m = Math.max(k, 0.0001);
  return [m, 0, 0, m, cx * (1 - m), cy * (1 - m)];
}

function translateY(y: number): Matrix {
  'worklet';
  return [1, 0, 0, 1, 0, y];
}

// `matrix` no está en los tipos de `G`, pero es el prop nativo que recibe.
const AnimatedG = Animated.createAnimatedComponent(G as unknown as ComponentClass<GProps & { matrix?: Matrix }>);
const AnimatedCircle = Animated.createAnimatedComponent(Circle);

// Mientras el splash animado no esté dibujado, se queda el del sistema.
SplashScreen.preventAutoHideAsync().catch(() => {});

export function LogoSplash() {
  const still = useReducedMotion();
  const [done, setDone] = useState(false);
  // Avance (0 → 1) de cada pieza; con movimiento reducido todo empieza en su lugar.
  const at = still ? 1 : 0;
  const glow = useSharedValue(at);
  // El libro cerrado ya está a la vista: es lo que deja el splash del sistema.
  const book = useSharedValue(1);
  const cover = useSharedValue(at);
  const p0 = useSharedValue(at);
  const p1 = useSharedValue(at);
  const p2 = useSharedValue(at);
  const whole = useSharedValue(at);
  const l0 = useSharedValue(at);
  const l1 = useSharedValue(at);
  const l2 = useSharedValue(at);
  const l3 = useSharedValue(at);
  const l4 = useSharedValue(at);
  const dotIn = useSharedValue(at);
  const fade = useSharedValue(0);
  const pages = [p0, p1, p2];
  const letters = [l0, l1, l2, l3, l4];

  const start = () => {
    SplashScreen.hideAsync().catch(() => {});
    if (!still) {
      play(glow, T.glow);
      play(cover, T.cover);
      T.pages.forEach((step, i) => play(pages[i], step));
      // Con las piezas en su lugar, el logo sin recortes (idéntico al ícono).
      whole.value = withDelay(PAGES_DONE * 1000, withTiming(1, { duration: 0, reduceMotion: ReduceMotion.Never }), ReduceMotion.Never);
      T.letters.forEach((step, i) => play(letters[i], step));
      play(dotIn, T.dot);
    }
    play(fade, T.fade, () => setDone(true));
  };

  const rootStyle = useAnimatedStyle(() => ({ opacity: 1 - fade.value }));
  const glowProps = useAnimatedProps(() => ({ opacity: glow.value }));
  const bookProps = useAnimatedProps(() => ({ opacity: book.value }));
  const openingProps = useAnimatedProps(() => ({ opacity: 1 - whole.value }));
  const wholeProps = useAnimatedProps(() => ({ opacity: whole.value }));
  const coverProps = useAnimatedProps(() => ({ matrix: rotateAbout(BOOK_CLOSED.cover * (1 - cover.value), hx, hy) }));
  const pageProps = [
    useAnimatedProps(() => ({ matrix: rotateAbout(BOOK_CLOSED.pages[0] * (1 - p0.value), hx, hy) })),
    useAnimatedProps(() => ({ matrix: rotateAbout(BOOK_CLOSED.pages[1] * (1 - p1.value), hx, hy) })),
    useAnimatedProps(() => ({ matrix: rotateAbout(BOOK_CLOSED.pages[2] * (1 - p2.value), hx, hy) })),
  ];
  // Las letras suben desde debajo de su línea base; el punto crece desde su centro.
  const letterProps = [
    useAnimatedProps(() => ({ matrix: translateY(WORDMARK.rise * (1 - l0.value)) })),
    useAnimatedProps(() => ({ matrix: translateY(WORDMARK.rise * (1 - l1.value)) })),
    useAnimatedProps(() => ({ matrix: translateY(WORDMARK.rise * (1 - l2.value)) })),
    useAnimatedProps(() => ({ matrix: translateY(WORDMARK.rise * (1 - l3.value)) })),
    useAnimatedProps(() => ({ matrix: translateY(WORDMARK.rise * (1 - l4.value)) })),
  ];
  const dotProps = useAnimatedProps(() => ({ matrix: scaleAbout(dotIn.value, dot.cx, dot.cy) }));

  if (done) return null;

  const pagePath = (i: number) => <Path d={LOGO_PAGES[i].d} opacity={LOGO_PAGES[i].opacity} fill={`url(#g${i})`} />;
  const body = <Path d={LOGO_BODY.d} fill={LOGO_BODY.fill} />;

  return (
    <Animated.View style={[styles.root, rootStyle]} onLayout={start} pointerEvents="none">
      <Svg width={SPLASH_IMAGE_DP} height={SPLASH_IMAGE_DP} viewBox={`${F.x} ${F.y} ${F.side} ${F.side}`}>
        <Defs>
          {LOGO_PAGES.map((p, i) => (
            <LinearGradient key={i} id={`g${i}`} x1={p.gradient.x1} y1={p.gradient.y1} x2={p.gradient.x2} y2={p.gradient.y2} gradientUnits="userSpaceOnUse">
              <Stop offset="0" stopColor={p.gradient.from} />
              <Stop offset="1" stopColor={p.gradient.to} />
            </LinearGradient>
          ))}
          <RadialGradient id="glow" cx={SPLASH_GLOW.cx} cy={SPLASH_GLOW.cy} r={SPLASH_GLOW.r} gradientUnits="userSpaceOnUse">
            <Stop offset="0" stopColor={SPLASH_GLOW.color} stopOpacity={SPLASH_GLOW.opacity} />
            <Stop offset="1" stopColor={SPLASH_GLOW.color} stopOpacity={0} />
          </RadialGradient>
          {/* Recortes fijos: lo que gira solo se ve por encima del borde de la contratapa. */}
          <ClipPath id="above">
            <Path d={bookEdgePlane('above')} />
          </ClipPath>
          <ClipPath id="cover">
            <Path d={bookEdgePlane('above', COVER_OVERLAP)} />
          </ClipPath>
          <ClipPath id="below">
            <Path d={bookEdgePlane('below')} />
          </ClipPath>
          {/* Las letras solo se ven por encima de su línea base: salen de ella. */}
          <ClipPath id="baseline">
            <Rect x={F.x} y={F.y} width={F.side} height={WORDMARK.baseline - F.y} />
          </ClipPath>
        </Defs>

        <AnimatedCircle cx={SPLASH_GLOW.cx} cy={SPLASH_GLOW.cy} r={SPLASH_GLOW.r} fill="url(#glow)" animatedProps={glowProps} />

        <AnimatedG animatedProps={bookProps}>
          {/* Abriéndose: páginas, contratapa y tapa (el orden del ícono). */}
          <AnimatedG animatedProps={openingProps}>
            {LOGO_PAGES.map((_, i) => (
              <G key={i} clipPath="url(#above)">
                <AnimatedG transform={rotateAbout(BOOK_CLOSED.pages[i] * (1 - at), hx, hy)} animatedProps={pageProps[i]}>
                  {pagePath(i)}
                </AnimatedG>
              </G>
            ))}
            <G clipPath="url(#below)">{body}</G>
            <G clipPath="url(#cover)">
              <AnimatedG transform={rotateAbout(BOOK_CLOSED.cover * (1 - at), hx, hy)} animatedProps={coverProps}>
                {/* La tapa es el cuerpo por encima del borde; este recorte gira con ella. */}
                <G clipPath="url(#cover)">{body}</G>
              </AnimatedG>
            </G>
          </AnimatedG>
          {/* Abierto: el logo tal cual. */}
          <AnimatedG animatedProps={wholeProps}>
            {LOGO_PAGES.map((_, i) => (
              <G key={i}>{pagePath(i)}</G>
            ))}
            {body}
          </AnimatedG>
        </AnimatedG>

        {/* El nombre del logotipo: cada letra sube desde la línea base y al final aparece el punto. */}
        <G clipPath="url(#baseline)">
          {WORDMARK.letters.map((l, i) => (
            <AnimatedG key={l.letter} transform={translateY(WORDMARK.rise * (1 - at))} animatedProps={letterProps[i]}>
              {l.d.map((d) => (
                <Path key={d} d={d} stroke={WORDMARK.color} strokeWidth={WORDMARK.strokeWidth} strokeLinecap="round" strokeLinejoin="round" fill="none" />
              ))}
            </AnimatedG>
          ))}
        </G>
        <AnimatedG transform={scaleAbout(at, dot.cx, dot.cy)} animatedProps={dotProps}>
          <Circle cx={dot.cx} cy={dot.cy} r={dot.r} fill={dot.color} />
        </AnimatedG>
      </Svg>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: SPLASH_BACKGROUND, alignItems: 'center', justifyContent: 'center' },
});
