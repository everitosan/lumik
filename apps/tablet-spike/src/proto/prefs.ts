// Ajustes del prototipo que deben sobrevivir al cierre: si ya se vio la guía de
// gestos y el orden y los archivados del inicio (en el desktop van en la base
// global). Un JSON en la sandbox.
import { readText, writeText } from '@lumik/core-native';

const PATH = '/data/user/0/com.lumik.tabletspike/files/prefs.json';

export type SortDirection = 'asc' | 'desc';

type Prefs = {
  guideSeen?: boolean;
  dashboardSort?: { years: SortDirection; projects: SortDirection };
  showArchived?: boolean;
};

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
