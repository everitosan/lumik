// Motivos de los elementos que fallan en un lote (spec photo-marking), como los
// devuelven los comandos `trash_photos`, `restore_trash_items` y `purge_trash_items`.
import type { TFunction } from 'i18next';
import { projectLockHolder } from './api';

/** `collision` → "su lugar está ocupado"; `project_locked:X` → "en uso en X"; el resto, tal cual. */
export function describeBatchReason(t: TFunction, reason: string): string {
  if (reason === 'collision') return t('batch.collision');
  const holder = projectLockHolder(reason);
  if (holder !== null) return t('trash.locked', { device: holder });
  return reason;
}
