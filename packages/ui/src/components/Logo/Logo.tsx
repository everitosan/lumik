import { useId, type CSSProperties } from 'react';

import { LOGO_BODY, LOGO_PAGES, LOGO_SIZE } from '../../brand/logo';

export interface LogoProps {
  size?: 'sm' | 'md' | 'lg';
  className?: string;
  style?: CSSProperties;
}

const sizes = {
  sm: { iconSize: 24, fontSize: 16, totalWidth: 96 },
  md: { iconSize: 36, fontSize: 22, totalWidth: 136 },
  lg: { iconSize: 48, fontSize: 28, totalWidth: 180 },
};

export function Logo({ size = 'md', className, style }: LogoProps) {
  const { iconSize, fontSize, totalWidth } = sizes[size];
  const scale = iconSize / LOGO_SIZE;
  const uid = useId().replace(/:/g, '');
  const textX = iconSize + 10;
  const textY = iconSize * 0.7;

  return (
    <svg
      width={totalWidth}
      height={iconSize}
      viewBox={`0 0 ${totalWidth} ${iconSize}`}
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      style={style}
      aria-label="Lumik"
    >
      {/* Ícono: los datos de `@lumik/ui/brand`, escalados de su viewBox a iconSize. */}
      <defs>
        {LOGO_PAGES.map((p, i) => (
          <linearGradient key={i} id={`${uid}-g${i}`} x1={p.gradient.x1} y1={p.gradient.y1} x2={p.gradient.x2} y2={p.gradient.y2} gradientUnits="userSpaceOnUse">
            <stop stopColor={p.gradient.from} />
            <stop offset="1" stopColor={p.gradient.to} />
          </linearGradient>
        ))}
      </defs>

      {/* Como el ícono: las páginas debajo del cuerpo. */}
      <g transform={`scale(${scale})`}>
        {LOGO_PAGES.map((p, i) => (
          <path key={i} d={p.d} opacity={p.opacity} fill={`url(#${uid}-g${i})`} />
        ))}
        <path d={LOGO_BODY.d} fill={LOGO_BODY.fill} />
      </g>

      {/* Wordmark */}
      <text
        x={textX}
        y={textY}
        fontFamily="var(--lumik-font-primary, Inter)"
        fontSize={fontSize}
        fontWeight="600"
        fill="var(--lumik-on-surface, #e5e2e1)"
        letterSpacing="-0.02em"
      >
        Lumik
      </text>
    </svg>
  );
}
