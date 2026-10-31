// Configuración de Expo. La versión sale del tag del monorepo (`LUMIK_VERSION`,
// p. ej. `0.2.0-beta.3`) y el código de versión de Android se deriva de ella
// para que crezca con cada AAB (spec `tablet-distribution`).
import type { ConfigContext, ExpoConfig } from 'expo/config';

const version = process.env.LUMIK_VERSION?.replace(/^v/, '') ?? '0.0.0';

/** `major*1 000 000 + minor*10 000 + patch*100 + beta`; una estable usa 99. */
export function versionCode(v: string): number {
  const m = /^(\d+)\.(\d+)\.(\d+)(?:-beta\.(\d+))?$/.exec(v);
  if (!m) throw new Error(`LUMIK_VERSION no es X.Y.Z ni X.Y.Z-beta.N: ${v}`);
  const [, major, minor, patch, beta] = m.map(Number);
  const build = m[4] === undefined ? 99 : beta;
  if (build > 98 && m[4] !== undefined) throw new Error(`Beta demasiado alta para el código de versión: ${v}`);
  return Math.max(1, major * 1_000_000 + minor * 10_000 + patch * 100 + build);
}

export default ({ config }: ConfigContext): ExpoConfig => ({
  ...config,
  name: 'Lumik',
  slug: 'lumik-tablet',
  version,
  orientation: 'default',
  icon: './assets/icon.png',
  userInterfaceStyle: 'dark',
  ios: {
    supportsTablet: true,
    bundleIdentifier: 'com.lumik.tablet',
  },
  android: {
    adaptiveIcon: {
      backgroundColor: '#E6F4FE',
      foregroundImage: './assets/android-icon-foreground.png',
      backgroundImage: './assets/android-icon-background.png',
      monochromeImage: './assets/android-icon-monochrome.png',
    },
    predictiveBackGestureEnabled: false,
    package: 'com.lumik.tablet',
    versionCode: versionCode(version),
    // Solo SAF para discos y tarjetas; ninguna dependencia debe colar permisos
    // de almacenamiento amplio ni de dibujar sobre otras apps (spec `tablet-distribution`).
    blockedPermissions: [
      'android.permission.MANAGE_EXTERNAL_STORAGE',
      'android.permission.READ_EXTERNAL_STORAGE',
      'android.permission.WRITE_EXTERNAL_STORAGE',
      'android.permission.SYSTEM_ALERT_WINDOW',
    ],
  },
  web: {
    favicon: './assets/favicon.png',
  },
  plugins: ['expo-image', './plugins/withReleaseSigning'],
});
