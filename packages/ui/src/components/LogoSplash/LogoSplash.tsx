import { useEffect, useId, useRef, useState, type CSSProperties } from 'react';

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
} from '../../brand/logo';

export interface LogoSplashProps {
  /** Lado del cuadro del splash (el logo y el nombre, con su resplandor). Por defecto, 45 % del lado menor de la ventana, hasta 400 px. */
  size?: string;
  /** Al terminar el desvanecimiento, cuando el splash ya se quitó. */
  onDone?: () => void;
}

const VIEW = SPLASH_FRAME;
const { x: hx, y: hy } = BOOK_HINGE;
const { dot } = WORDMARK;

const easings: Record<SplashStep['easing'], (x: number) => number> = {
  linear: (x) => x,
  'out-cubic': (x) => 1 - (1 - x) ** 3,
  'out-back': (x) => {
    const s = OUT_BACK_OVERSHOOT;
    const k = x - 1;
    return 1 + (s + 1) * k ** 3 + s * k ** 2;
  },
};

/** Avance (0 → 1) de un paso de la línea de tiempo en el segundo `t`. */
function progress(step: SplashStep, t: number) {
  const x = Math.min(1, Math.max(0, (t - step.from) / (step.to - step.from)));
  return easings[step.easing](x);
}

const reducedMotion = () =>
  typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;

/**
 * Splash de arranque (spec `app-splash`): el libro del logo se abre sobre un
 * resplandor, el nombre sale debajo letra por letra y todo se desvanece dejando
 * ver la app. Se dibuja encima de
 * todo y se quita solo al terminar. Cada cuadro escribe los atributos del SVG
 * (sin renders de React) y gira con `transform="rotate(a cx cy)"`, que se
 * comporta igual en los webviews de Tauri.
 */
export function LogoSplash({ size = 'min(45vmin, 400px)', onDone }: LogoSplashProps) {
  const uid = useId().replace(/:/g, '');
  const [done, setDone] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const glow = useRef<SVGCircleElement>(null);
  const book = useRef<SVGGElement>(null);
  const opening = useRef<SVGGElement>(null);
  const finished = useRef<SVGGElement>(null);
  const cover = useRef<SVGGElement>(null);
  const pages = useRef<(SVGGElement | null)[]>([]);
  const letters = useRef<(SVGGElement | null)[]>([]);
  const dotRef = useRef<SVGGElement>(null);
  const onDoneRef = useRef(onDone);
  onDoneRef.current = onDone;

  useEffect(() => {
    const still = reducedMotion();
    const T = SPLASH_TIMELINE;
    const pagesDone = T.pages[T.pages.length - 1].to;
    let frame = 0;
    const start = performance.now();

    const draw = (now: number) => {
      const t = (now - start) / 1000;
      if (still) {
        // Movimiento reducido: el logo y el nombre quietos con su resplandor, y el mismo desvanecimiento.
        glow.current?.setAttribute('opacity', '1');
        book.current?.setAttribute('opacity', '1');
        letters.current.forEach((l) => l?.setAttribute('transform', 'translate(0 0)'));
        dotRef.current?.setAttribute('transform', 'translate(0 0)');
      } else {
        glow.current?.setAttribute('opacity', String(progress(T.glow, t)));
        book.current?.setAttribute('opacity', String(progress(T.book, t)));
        cover.current?.setAttribute('transform', `rotate(${BOOK_CLOSED.cover * (1 - progress(T.cover, t))} ${hx} ${hy})`);
        T.pages.forEach((step, i) =>
          pages.current[i]?.setAttribute('transform', `rotate(${BOOK_CLOSED.pages[i] * (1 - progress(step, t))} ${hx} ${hy})`),
        );
        // Las letras suben desde debajo de su línea base; el punto crece desde su centro.
        T.letters.forEach((step, i) =>
          letters.current[i]?.setAttribute('transform', `translate(0 ${WORDMARK.rise * (1 - progress(step, t))})`),
        );
        const k = progress(T.dot, t);
        dotRef.current?.setAttribute('transform', `translate(${dot.cx} ${dot.cy}) scale(${k}) translate(${-dot.cx} ${-dot.cy})`);
      }
      // Con las piezas en su lugar, el logo sin recortes (idéntico al ícono).
      const whole = still || t >= pagesDone;
      opening.current?.setAttribute('display', whole ? 'none' : 'inline');
      finished.current?.setAttribute('display', whole ? 'inline' : 'none');
      if (root.current) root.current.style.opacity = String(1 - progress(T.fade, t));

      if (t < T.total) {
        frame = requestAnimationFrame(draw);
      } else {
        setDone(true);
        onDoneRef.current?.();
      }
    };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  }, []);

  if (done) return null;

  const id = (name: string) => `${uid}-${name}`;
  const pagePath = (i: number) => (
    <path d={LOGO_PAGES[i].d} opacity={LOGO_PAGES[i].opacity} fill={`url(#${id(`g${i}`)})`} />
  );
  const body = <path d={LOGO_BODY.d} fill={LOGO_BODY.fill} />;

  return (
    <div ref={root} style={overlayStyles} aria-hidden="true">
      <svg
        width={size}
        height={size}
        viewBox={`${VIEW.x} ${VIEW.y} ${VIEW.side} ${VIEW.side}`}
        style={{ width: size, height: size }}
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <defs>
          {LOGO_PAGES.map((p, i) => (
            <linearGradient key={i} id={id(`g${i}`)} x1={p.gradient.x1} y1={p.gradient.y1} x2={p.gradient.x2} y2={p.gradient.y2} gradientUnits="userSpaceOnUse">
              <stop stopColor={p.gradient.from} />
              <stop offset="1" stopColor={p.gradient.to} />
            </linearGradient>
          ))}
          <radialGradient id={id('glow')} cx={SPLASH_GLOW.cx} cy={SPLASH_GLOW.cy} r={SPLASH_GLOW.r} gradientUnits="userSpaceOnUse">
            <stop stopColor={SPLASH_GLOW.color} stopOpacity={SPLASH_GLOW.opacity} />
            <stop offset="1" stopColor={SPLASH_GLOW.color} stopOpacity={0} />
          </radialGradient>
          {/* Recortes fijos (no giran): lo que gira solo se ve por encima del borde de la contratapa. */}
          <clipPath id={id('above')}>
            <path d={bookEdgePlane('above')} />
          </clipPath>
          <clipPath id={id('cover')}>
            <path d={bookEdgePlane('above', COVER_OVERLAP)} />
          </clipPath>
          <clipPath id={id('below')}>
            <path d={bookEdgePlane('below')} />
          </clipPath>
          {/* Las letras solo se ven por encima de su línea base: salen de ella. */}
          <clipPath id={id('baseline')}>
            <rect x={VIEW.x} y={VIEW.y} width={VIEW.side} height={WORDMARK.baseline - VIEW.y} />
          </clipPath>
        </defs>

        <circle ref={glow} cx={SPLASH_GLOW.cx} cy={SPLASH_GLOW.cy} r={SPLASH_GLOW.r} fill={`url(#${id('glow')})`} opacity={0} />

        <g ref={book} opacity={0}>
          {/* Abriéndose: páginas, contratapa y tapa (el orden del ícono). */}
          <g ref={opening}>
            {LOGO_PAGES.map((_, i) => (
              <g key={i} clipPath={`url(#${id('above')})`}>
                <g
                  ref={(el) => {
                    pages.current[i] = el;
                  }}
                  transform={`rotate(${BOOK_CLOSED.pages[i]} ${hx} ${hy})`}
                >
                  {pagePath(i)}
                </g>
              </g>
            ))}
            <g clipPath={`url(#${id('below')})`}>{body}</g>
            <g clipPath={`url(#${id('cover')})`}>
              <g ref={cover} transform={`rotate(${BOOK_CLOSED.cover} ${hx} ${hy})`}>
                {/* La tapa es el cuerpo por encima del borde; este recorte gira con ella. */}
                <g clipPath={`url(#${id('cover')})`}>{body}</g>
              </g>
            </g>
          </g>
          {/* Abierto: el logo tal cual. */}
          <g ref={finished} display="none">
            {LOGO_PAGES.map((_, i) => (
              <g key={i}>{pagePath(i)}</g>
            ))}
            {body}
          </g>
        </g>

        {/* El nombre del logotipo: cada letra sube desde la línea base y al final aparece el punto. */}
        <g clipPath={`url(#${id('baseline')})`}>
          {WORDMARK.letters.map((l, i) => (
            <g
              key={l.letter}
              ref={(el) => {
                letters.current[i] = el;
              }}
              transform={`translate(0 ${WORDMARK.rise})`}
            >
              {l.d.map((d) => (
                <path key={d} d={d} stroke={WORDMARK.color} strokeWidth={WORDMARK.strokeWidth} strokeLinecap="round" strokeLinejoin="round" />
              ))}
            </g>
          ))}
        </g>
        <g ref={dotRef} transform={`translate(${dot.cx} ${dot.cy}) scale(0) translate(${-dot.cx} ${-dot.cy})`}>
          <circle cx={dot.cx} cy={dot.cy} r={dot.r} fill={dot.color} />
        </g>
      </svg>
    </div>
  );
}

const overlayStyles: CSSProperties = {
  position: 'fixed',
  inset: 0,
  zIndex: 2147483647,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  background: SPLASH_BACKGROUND,
};
