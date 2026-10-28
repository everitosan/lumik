// Ajustes del prototipo que deben sobrevivir al cierre: el disco elegido y si
// ya se vio la guía de gestos. Un JSON en la sandbox.
import { readText, writeText } from '@lumik/core-native';

const PATH = '/data/user/0/com.lumik.tabletspike/files/prefs.json';

type Prefs = { guideSeen?: boolean };

export function readPrefs(): Prefs {
  try {
    return JSON.parse(readText(PATH) ?? '{}');
  } catch {
    return {};
  }
}

export function savePrefs(change: Prefs) {
  try {
    writeText(PATH, JSON.stringify({ ...readPrefs(), ...change }));
  } catch (e) {
    console.warn('No se pudieron guardar los ajustes', e);
  }
}
