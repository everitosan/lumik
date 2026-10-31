// Firma de release (spec `tablet-distribution`): si el entorno trae la clave de
// subida de Google Play, el build de release la usa; si no, queda la de debug
// (builds locales para probar en la tablet). Nada secreto se versiona.
//   LUMIK_UPLOAD_STORE_FILE      ruta del keystore (.jks)
//   LUMIK_UPLOAD_STORE_PASSWORD  contraseña del keystore
//   LUMIK_UPLOAD_KEY_ALIAS       alias de la clave
//   LUMIK_UPLOAD_KEY_PASSWORD    contraseña de la clave
const { withAppBuildGradle } = require('expo/config-plugins');

const MARK = '// lumik: release signing';

module.exports = function withReleaseSigning(config) {
  return withAppBuildGradle(config, (cfg) => {
    let gradle = cfg.modResults.contents;
    if (gradle.includes(MARK)) return cfg;
    gradle = gradle.replace(
      /signingConfigs \{\n/,
      `signingConfigs {
        ${MARK}
        if (System.getenv('LUMIK_UPLOAD_STORE_FILE')) {
            release {
                storeFile file(System.getenv('LUMIK_UPLOAD_STORE_FILE'))
                storePassword System.getenv('LUMIK_UPLOAD_STORE_PASSWORD')
                keyAlias System.getenv('LUMIK_UPLOAD_KEY_ALIAS')
                keyPassword System.getenv('LUMIK_UPLOAD_KEY_PASSWORD')
            }
        }
`,
    );
    gradle = gradle.replace(
      /(release \{\n(?:\s*\/\/[^\n]*\n)*)(\s*)signingConfig signingConfigs\.debug/,
      `$1$2signingConfig System.getenv('LUMIK_UPLOAD_STORE_FILE') ? signingConfigs.release : signingConfigs.debug`,
    );
    if (!gradle.includes("signingConfigs.release : signingConfigs.debug")) {
      throw new Error('withReleaseSigning: no se encontró la firma del build de release en build.gradle');
    }
    cfg.modResults.contents = gradle;
    return cfg;
  });
};
