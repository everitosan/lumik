// Ajustes de la tablet que deben sobrevivir al cierre: los del import y la
// firma del fotógrafo, si ya se vio la guía de gestos, cómo se dejó el visor y
// el orden y los archivados del inicio (en el desktop van en la base global). Un JSON en la sandbox.
import { type ImportSettings, readText, writeText } from '@lumik/core-native';
import { FILES_DIR } from './paths';

const PATH = `${FILES_DIR}/prefs.json`;

export type SortDirection = 'asc' | 'desc';

type Prefs = {
  /** Firma del fotógrafo y renombrado al importar (spec `tablet-settings`). */
  importSettings?: ImportSettings;
  /** Entrega al aprobar con que se crean los proyectos (`none`, `pairs`, `all_camera_jpg`). */
  defaultDelivery?: string;
  guideSeen?: boolean;
  /** Modo limpio y panel de información (histograma) del visor, como se dejaron. */
  viewer?: { clean: boolean; showInfo: boolean };
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

/** Ajustes del import; firmar y renombrar están activos de fábrica. */
export function importSettings(): ImportSettings {
  return { sign: true, rename: true, author: undefined, copyright: undefined, url: undefined, ...readPrefs().importSettings };
}

export function savePrefs(change: Prefs) {
  try {
    writeText(PATH, JSON.stringify({ ...readPrefs(), ...change }));
  } catch (e) {
    console.warn('No se pudieron guardar los ajustes', e);
  }
}
