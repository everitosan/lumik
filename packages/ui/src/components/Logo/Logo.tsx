import type { CSSProperties } from 'react';

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
  const scale = iconSize / 635;
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
      <defs>
        <linearGradient id="lmk-g0" x1="85.0523" y1="592.438" x2="281.885" y2="296.751" gradientUnits="userSpaceOnUse">
          <stop stopColor="#5B55FF"/>
          <stop offset="1" stopColor="#11AAAF"/>
        </linearGradient>
        <linearGradient id="lmk-g1" x1="328.88" y1="564.393" x2="419.392" y2="303.817" gradientUnits="userSpaceOnUse">
          <stop stopColor="#E81759"/>
          <stop offset="1" stopColor="#1760E8"/>
        </linearGradient>
        <linearGradient id="lmk-g2" x1="95.8415" y1="553.69" x2="636.031" y2="382.571" gradientUnits="userSpaceOnUse">
          <stop stopColor="#EED231"/>
          <stop offset="1" stopColor="#540D70"/>
        </linearGradient>
      </defs>

      {/* Icon — original viewBox 635×635 scaled to iconSize */}
      <g transform={`scale(${scale})`}>
        <path
          d="M199.959 15C214.245 15.0001 225.827 26.5816 225.827 40.8682C225.827 42.0448 225.747 43.2029 225.595 44.3379L224.89 223.386V223.451L224.873 223.514L215.357 258.854L176.115 328.241L96.1455 467.38C96.6409 467.249 97.1385 467.123 97.6377 467.002L57.9658 537.15L57.4922 537.988L58.4502 537.895L190.915 524.842C190.967 525.072 191.018 525.302 191.067 525.533L353.067 509.55C353.042 509.324 353.019 509.097 352.995 508.871L593.278 485.194C593.297 485.41 593.318 485.626 593.335 485.843L593.512 485.826V488.368C593.656 490.8 593.731 493.251 593.731 495.72C593.731 559.761 544.45 612.222 481.956 616.826L481.856 616.938L481.637 616.938L480.186 616.943C477.812 617.083 475.42 617.157 473.012 617.157C470.916 617.157 468.832 617.102 466.763 616.996L122.024 618.352C120 618.511 117.953 618.594 115.887 618.594C73.4236 618.594 39.0001 584.171 39 541.708C39 539.661 39.0812 537.632 39.2383 535.625L40.6279 154.714C40.5021 152.949 40.4375 151.168 40.4375 149.371C40.4377 119.477 58.1614 93.7226 83.6738 82.042L182.963 21.3662C187.509 17.4014 193.453 15 199.959 15Z"
          fill="#1F252F"
        />
        <path
          opacity="0.85"
          d="M286.479 144.342C295.606 144.342 303.006 151.741 303.006 160.869C303.006 161.784 302.93 162.682 302.787 163.556V517.017L302.329 517.056L56.5803 537.894L55.6379 537.974L56.1028 537.15L270.362 157.198C272.031 149.837 278.613 144.342 286.479 144.342Z"
          fill="url(#lmk-g0)"
        />
        <path
          opacity="0.6"
          d="M497.397 203.264C510.493 203.264 521.11 213.739 521.11 226.66C521.11 227.743 521.033 228.808 520.889 229.852L520.172 495.361L520.17 495.817L519.717 495.858L60.1049 537.687L58.4047 537.841L59.7533 536.794L480.563 209.959L480.808 209.769L480.929 209.826C485.194 205.763 491 203.264 497.397 203.264Z"
          fill="url(#lmk-g1)"
        />
        <path
          opacity="0.6"
          d="M570.287 331.168C583.384 331.168 594 341.785 594 354.881C594 356.527 593.831 358.134 593.512 359.686V486.515L593.061 486.559L56.2924 538.613L56.057 537.651L555.962 335.984C559.942 332.962 564.905 331.168 570.287 331.168Z"
          fill="url(#lmk-g2)"
        />
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
