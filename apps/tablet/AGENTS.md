This is an Expo/React Native mobile application. Prioritize mobile-first patterns, performance, and cross-platform compatibility.

## Expo has changed — do not trust your training data

Expo ships breaking changes every SDK release. APIs you remember are likely renamed, moved, or removed. Before writing any code that touches an Expo, EAS, or React Native API:

1. Read the major version of the `expo` package in `package.json`.
2. Fetch the matching versioned docs: `https://docs.expo.dev/versions/v<major>.0.0/`
3. For anything else, fetch https://docs.expo.dev/llms.txt — an index of all Expo docs with corrections to common LLM misconceptions. Follow its links to the specific page you need; never answer from memory.

## Commands

Use `bunx` instead of `npx` if the project uses bun (`bun.lock` present).

```bash
pnpm exec expo install <package>  # ALWAYS use instead of npm/yarn/pnpm/bun add — resolves SDK-compatible versions
pnpm exec expo start              # start the dev server
pnpm exec expo lint               # lint
pnpm exec tsc --noEmit            # typecheck
pnpm exec expo-doctor             # diagnose dependency and config issues
pnpm exec expo install --fix      # fix incompatible package versions
```

Run lint and typecheck before declaring any task done.

## Navigation

- Lumik tablet does **not** use Expo Router. Navigation is state-based, like the desktop app: `src/AppScreen.tsx` switches between home → project → photo (plus import), and Android's back button goes up one level through `BackHandler`. Keep it that way unless a change decides otherwise.

## Building

The Rust core (`crates/lumik-mobile`) is compiled with uniffi-bindgen-react-native, so builds run locally or in GitHub Actions, not on EAS:

```bash
# Env: ANDROID_HOME, NDK_HOME/ANDROID_NDK_HOME (NDK r26), JAVA_HOME (full JDK), cmake/ninja from the SDK on PATH
pnpm --dir modules/core-native ubrn:android --release   # Rust core + bindings
pnpm exec expo prebuild --platform android               # android/ (CNG, not committed)
cd android && ./gradlew assembleRelease -PreactNativeArchitectures=arm64-v8a
```

Performance numbers only count on release builds: in debug, each uniffi callback is ~35x slower.

## Disk access (SAF)

- Disks and cards are reached only through SAF (`modules/saf`); never request broad storage permissions.
- Never overwrite an existing file on the disk in place: on Android it can end up empty after unmount. Use `replaceFile` (`src/safWrite.ts`).
- Create files as `application/octet-stream`, or Android appends an extension to the name.

## Rules

- If `ios/` and `android/` directories do not exist, they are generated (Continuous Native Generation). Never create or edit them by hand — configure native behavior in `app.config.ts` and config plugins (`plugins/`).
- Expo Go only includes its bundled native modules. After adding a library with native code, the app needs a development build: `pnpm exec expo run:ios|android` locally, or `eas build --profile development`.
- Prefer recommended Expo modules over third-party libraries, and check your available skills before adding dependencies. Docs: https://docs.expo.dev/versions/latest/index.md
