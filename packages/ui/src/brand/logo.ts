// Geometría del logo de Lumik y de su animación de arranque (spec `app-splash`).
// Son datos sin dependencias: los usan `Logo` y `LogoSplash` de este paquete y
// el splash de la tablet (React Native), que importa solo `@lumik/ui/brand`.
//
// El logo es un libro visto desde su borde inferior: el cuerpo oscuro es una L
// (la tapa de pie, a la izquierda, y la contratapa, abajo) y las tres páginas de
// color salen del lomo en abanico. Para animarlo, el cuerpo se parte con la recta
// del borde superior de la contratapa: lo de arriba es la tapa, lo de abajo la
// contratapa. La tapa y las páginas giran sobre el lomo y solo se dibujan por
// encima de esa recta, así que cerradas quedan escondidas en el libro.

/** Lado del viewBox cuadrado del logo. */
export const LOGO_SIZE = 635;

/** Cuerpo del libro (la L oscura), de `apps/landing/public/icon.svg`. */
export const LOGO_BODY = {
  d: 'M199.959 15C214.245 15.0001 225.827 26.5816 225.827 40.8682C225.827 42.0448 225.747 43.2029 225.595 44.3379L224.89 223.386V223.451L224.873 223.514L215.357 258.854L176.115 328.241L96.1455 467.38C96.6409 467.249 97.1385 467.123 97.6377 467.002L57.9658 537.15L57.4922 537.988L58.4502 537.895L190.915 524.842C190.967 525.072 191.018 525.302 191.067 525.533L353.067 509.55C353.042 509.324 353.019 509.097 352.995 508.871L593.278 485.194C593.297 485.41 593.318 485.626 593.335 485.843L593.512 485.826V488.368C593.656 490.8 593.731 493.251 593.731 495.72C593.731 559.761 544.45 612.222 481.956 616.826L481.856 616.938L481.637 616.938L480.186 616.943C477.812 617.083 475.42 617.157 473.012 617.157C470.916 617.157 468.832 617.102 466.763 616.996L122.024 618.352C120 618.511 117.953 618.594 115.887 618.594C73.4236 618.594 39.0001 584.171 39 541.708C39 539.661 39.0812 537.632 39.2383 535.625L40.6279 154.714C40.5021 152.949 40.4375 151.168 40.4375 149.371C40.4377 119.477 58.1614 93.7226 83.6738 82.042L182.963 21.3662C187.509 17.4014 193.453 15 199.959 15Z',
  fill: '#1F252F',
};

export type LogoPage = {
  d: string;
  opacity: number;
  /** Degradado lineal en coordenadas del viewBox (`userSpaceOnUse`). */
  gradient: { x1: number; y1: number; x2: number; y2: number; from: string; to: string };
};

/** Las tres páginas, de la más empinada a la más baja. En el ícono van debajo del cuerpo. */
export const LOGO_PAGES: LogoPage[] = [
  {
    d: 'M286.479 144.342C295.606 144.342 303.006 151.741 303.006 160.869C303.006 161.784 302.93 162.682 302.787 163.556V517.017L302.329 517.056L56.5803 537.894L55.6379 537.974L56.1028 537.15L270.362 157.198C272.031 149.837 278.613 144.342 286.479 144.342Z',
    opacity: 0.85,
    gradient: { x1: 85.0523, y1: 592.438, x2: 281.885, y2: 296.751, from: '#5B55FF', to: '#11AAAF' },
  },
  {
    d: 'M497.397 203.264C510.493 203.264 521.11 213.739 521.11 226.66C521.11 227.743 521.033 228.808 520.889 229.852L520.172 495.361L520.17 495.817L519.717 495.858L60.1049 537.687L58.4047 537.841L59.7533 536.794L480.563 209.959L480.808 209.769L480.929 209.826C485.194 205.763 491 203.264 497.397 203.264Z',
    opacity: 0.6,
    gradient: { x1: 328.88, y1: 564.393, x2: 419.392, y2: 303.817, from: '#E81759', to: '#1760E8' },
  },
  {
    d: 'M570.287 331.168C583.384 331.168 594 341.785 594 354.881C594 356.527 593.831 358.134 593.512 359.686V486.515L593.061 486.559L56.2924 538.613L56.057 537.651L555.962 335.984C559.942 332.962 564.905 331.168 570.287 331.168Z',
    opacity: 0.6,
    gradient: { x1: 95.8415, y1: 553.69, x2: 636.031, y2: 382.571, from: '#EED231', to: '#540D70' },
  },
];

/** Lomo: el eje sobre el que giran la tapa y las páginas (vértice común de las páginas). */
export const BOOK_HINGE = { x: 57.5, y: 537.5 };

/** Inclinación del borde superior de la contratapa, en grados (negativo: sube a la derecha). */
export const BOOK_EDGE_ANGLE = -5.5;

/**
 * Giro (grados, horario) que lleva cada pieza del logo al libro cerrado: la tapa
 * acostada sobre la contratapa y cada página con su borde superior bajo la recta.
 * Abierto, todos valen 0.
 */
export const BOOK_CLOSED = { cover: 84.5, pages: [57, 34, 18.5] };

/**
 * Semiplano por encima (`'above'`) o por debajo (`'below'`) de la recta del borde
 * de la contratapa, como trazo para un `clipPath`. `offset` baja la recta esas
 * unidades: la tapa se recorta 4 más abajo para solaparse con la contratapa y
 * que no quede una línea entre las dos.
 */
export function bookEdgePlane(side: 'above' | 'below', offset = 0): string {
  const a = (BOOK_EDGE_ANGLE * Math.PI) / 180;
  const [ux, uy] = [Math.cos(a), Math.sin(a)];
  // Normal hacia arriba de la pantalla.
  const [nx, ny] = [uy, -ux];
  const bx = BOOK_HINGE.x - nx * offset;
  const by = BOOK_HINGE.y - ny * offset;
  const far = LOGO_SIZE * 4;
  const s = side === 'above' ? 1 : -1;
  const pts = [
    [bx - ux * far, by - uy * far],
    [bx + ux * far, by + uy * far],
    [bx + ux * far + s * nx * far, by + uy * far + s * ny * far],
    [bx - ux * far + s * nx * far, by - uy * far + s * ny * far],
  ];
  return `M${pts.map(([x, y]) => `${x.toFixed(2)} ${y.toFixed(2)}`).join('L')}Z`;
}

/** Solape de la tapa bajo la recta del borde (ver `bookEdgePlane`). */
export const COVER_OVERLAP = 4;

/**
 * Nombre "lumik" del logotipo (`apps/landing/public/logotipo.png`), rehecho como
 * trazos de grosor uniforme con extremos redondos, en coordenadas del logo:
 * centrado bajo el libro, con la proporción y la separación del logotipo.
 */
export const WORDMARK = {
  strokeWidth: 24.41,
  /** Texto claro de la app, sobre el fondo oscuro. */
  color: '#E5E2E1',
  /** Línea base: las letras salen de debajo de esta recta horizontal (borde inferior de los trazos). */
  baseline: 884.8,
  /** Cuánto suben las letras desde debajo de la línea base. */
  rise: 200,
  letters: [
    { letter: 'l', d: ['M-82.4 706.4V872.6'] },
    { letter: 'u', d: ['M17.6 764.5V811.6A64.52 64.52 0 0 0 146.6 811.6V764.5'] },
    { letter: 'm', d: ['M243.1 872.6V809.5A47.37 47.37 0 0 1 337.9 809.5V872.6', 'M337.9 809.5A47.37 47.37 0 0 1 432.6 809.5V872.6'] },
    { letter: 'i', d: ['M535.5 763.9V872.6'] },
    { letter: 'k', d: ['M631.4 706.4V872.6', 'M643.6 827.9L708.7 762.8', 'M671.5 826.7L708.7 873.2'] },
  ],
  /** Punto de la "i", en el dorado de la marca. */
  dot: { cx: 535.5, cy: 712.2, r: 18.0, color: '#E9C349' },
};

/**
 * Cuadro del splash, en coordenadas del logo: el libro con el nombre (y=15..885)
 * centrado, con margen para el resplandor. La imagen del splash del sistema de la
 * tablet es este mismo cuadro, para que la animación siga sin saltos.
 */
export const SPLASH_FRAME = { x: -157, y: -20, side: 940 };

/** Curvas de la línea de tiempo. Cada plataforma usa su equivalente. */
export type SplashEasing = 'linear' | 'out-cubic' | 'out-back';

export type SplashStep = { from: number; to: number; easing: SplashEasing };

/**
 * Línea de tiempo del splash, en segundos (design D3). Al terminar las páginas se
 * dibuja el logo sin recortes (idéntico al ícono); después sale el nombre.
 */
export const SPLASH_TIMELINE = {
  /** Resplandor: opacidad 0 → 1. */
  glow: { from: 0, to: 0.3, easing: 'linear' },
  /** Libro cerrado: opacidad 0 → 1. */
  book: { from: 0.15, to: 0.35, easing: 'linear' },
  /** Tapa: de `BOOK_CLOSED.cover` a 0. */
  cover: { from: 0.4, to: 0.85, easing: 'out-cubic' },
  /** Cada página: de su ángulo en `BOOK_CLOSED.pages` a 0, con un leve rebote. */
  pages: [
    { from: 0.5, to: 0.95, easing: 'out-back' },
    { from: 0.6, to: 1.05, easing: 'out-back' },
    { from: 0.7, to: 1.15, easing: 'out-back' },
  ],
  /** Cada letra de `WORDMARK.letters`: sube desde la línea base, con un leve rebote. */
  letters: [0, 1, 2, 3, 4].map((i) => ({ from: 1.0 + 0.06 * i, to: 1.3 + 0.06 * i, easing: 'out-back' as const })),
  /** Punto de la "i": escala 0 → 1, con un leve rebote. */
  dot: { from: 1.45, to: 1.7, easing: 'out-back' },
  /** Todo (logo, nombre y resplandor): opacidad 1 → 0. */
  fade: { from: 1.75, to: 2.05, easing: 'linear' },
  total: 2.05,
} satisfies Record<string, SplashStep | SplashStep[] | number>;

/** Rebote de `out-back` (el `s` de la curva clásica; Reanimated: `Easing.back(1.4)`). */
export const OUT_BACK_OVERSHOOT = 1.4;

/** Resplandor detrás del libro: círculo con degradado radial, en coordenadas del logo. */
export const SPLASH_GLOW = { cx: 317, cy: 380, r: 420, color: '#C7D6FF', opacity: 0.28 };

/** Fondo de las apps detrás del splash. */
export const SPLASH_BACKGROUND = '#131313';
