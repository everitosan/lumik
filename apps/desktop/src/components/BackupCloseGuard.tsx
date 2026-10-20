import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ask } from '@tauri-apps/plugin-dialog';
import { useBackup } from '../lib/BackupContext';

/** Pide confirmación al cerrar la ventana si hay respaldos subiendo archivos.
 *  Cerrar igualmente no pierde el respaldo: queda en la cola y se reanuda al
 *  arrancar, pero la subida en curso se corta. */
export function BackupCloseGuard() {
  const { t } = useTranslation();
  const { jobs } = useBackup();
  // El listener se registra una vez; lee los jobs y los textos actuales por ref.
  const runningRef = useRef(0);
  runningRef.current = Object.values(jobs).filter((j) => j.running).length;
  const tRef = useRef(t);
  tRef.current = t;

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    const appWindow = getCurrentWindow();
    appWindow
      .onCloseRequested(async (event) => {
        const count = runningRef.current;
        if (count === 0) return; // sin subidas: se cierra sin preguntar
        // Hay que cancelar antes del primer await; si se confirma, se cierra a mano.
        event.preventDefault();
        const tr = tRef.current;
        const confirmed = await ask(tr('backup.closeConfirm.message', { count }), {
          title: tr('backup.closeConfirm.title'),
          kind: 'warning',
          okLabel: tr('backup.closeConfirm.confirm'),
          cancelLabel: tr('backup.closeConfirm.cancel'),
        });
        if (confirmed) await appWindow.destroy();
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch((err) => console.error('close guard unavailable', err));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return null;
}
