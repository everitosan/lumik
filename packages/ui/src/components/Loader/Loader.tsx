import type { CSSProperties } from 'react';

export interface LoaderProps {
  /** Final diameter in pixels. Default 16. */
  size?: number;
  /** Color of the static outer ring. Defaults to a themed token. */
  trackColor?: string;
  /** Color of the spinning arc. Defaults to a themed token. */
  color?: string;
  className?: string;
  style?: CSSProperties;
  /** Accessible label for screen readers. */
  label?: string;
}

/**
 * Spinning ring loader. The animation and `::after` arc live in `tokens.css`
 * under `.lumik-loader` (keyframes/pseudo-elements can't be inlined). Size and
 * colors are driven by CSS custom properties so they can be set per instance.
 */
export function Loader({
  size = 16,
  trackColor,
  color,
  className,
  style,
  label,
}: LoaderProps) {
  // The base CSS sizes everything as a multiple of `--size`, where the overall
  // diameter is `48 * --size`. Derive it from the requested pixel diameter.
  const vars: Record<string, string> = { '--size': `${size / 48}px` };
  if (trackColor) vars['--color-1'] = trackColor;
  if (color) vars['--color-2'] = color;

  return (
    <span
      className={className ? `lumik-loader ${className}` : 'lumik-loader'}
      style={{ ...vars, ...style } as CSSProperties}
      role="status"
      aria-label={label}
    />
  );
}
