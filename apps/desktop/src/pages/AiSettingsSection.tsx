import { useEffect, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Modal } from '@lumik/ui';
import {
  cancelContentModelDownload,
  deleteContentModel,
  downloadContentModel,
  getAppSettings,
  getContentModelState,
  onContentModelState,
  updateAppSettings,
} from '../lib/api';
import type { AnalysisMode, ContentModelState } from '../lib/types';
import { AnalysisModeSelect } from '../components/AnalysisModeSelect';

const sectionStyles: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '20px' };
/** Dos columnas (explicación | modelo) que pasan a una cuando no caben. */
const columnsStyles: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))',
  gap: '24px',
  alignItems: 'start',
};
const columnStyles: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '12px' };
const titleStyles: CSSProperties = { fontSize: '18px', fontWeight: 600, color: 'var(--lumik-on-surface, #e5e2e1)', margin: 0 };
const textStyles: CSSProperties = { fontSize: '14px', lineHeight: 1.5, color: 'var(--lumik-on-surface-variant, #c2c6d7)', margin: 0 };
const mutedStyles: CSSProperties = { fontSize: '13px', lineHeight: 1.5, color: 'var(--lumik-on-surface-variant, #c2c6d7)', margin: 0 };
const errorStyles: CSSProperties = { fontSize: '13px', color: 'var(--lumik-error, #ffb4ab)', margin: 0 };
const cardStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
  padding: '16px',
  borderRadius: '8px',
  background: 'var(--lumik-surface-container-low, #1c1b1b)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
};
const barTrackStyles: CSSProperties = { height: '6px', borderRadius: '3px', background: 'var(--lumik-surface-container-highest, #353534)', overflow: 'hidden' };

const mb = (bytes: number) => Math.round(bytes / 1_000_000);

/**
 * Ajustes → IA: funciones que usan modelos en el equipo. Hoy, la búsqueda por
 * contenido (spec `semantic-search`): el modelo opcional (descargar, cancelar,
 * borrar) y, con el modelo instalado, el modo de análisis de los proyectos nuevos.
 */
export function AiSettingsSection() {
  const { t } = useTranslation();
  const [model, setModel] = useState<ContentModelState | null>(null);
  const [mode, setMode] = useState<AnalysisMode>('on_demand');
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let active = true;
    getContentModelState().then((s) => active && setModel(s)).catch((e) => setError(String(e)));
    getAppSettings().then((s) => active && setMode(s.default_analysis_mode)).catch(() => undefined);
    onContentModelState((s) => setModel(s)).then((fn) => {
      if (active) unlisten = fn;
      else fn();
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  const run = (action: () => Promise<void>) => {
    setError(null);
    action().catch((e) => setError(String(e)));
  };

  const changeMode = (next: AnalysisMode) => {
    setMode(next);
    run(async () => {
      const current = await getAppSettings();
      await updateAppSettings({ ...current, default_analysis_mode: next });
    });
  };

  /** `código:detalle` de la última descarga fallida → texto. */
  const downloadError = model?.state === 'not_installed' && model.error
    ? t(`settings.contentSearch.errors.${model.error.split(':')[0]}`, { defaultValue: model.error })
    : null;

  const installed = model?.state === 'installed';

  return (
    <section style={sectionStyles}>
      <h2 style={titleStyles}>{t('settings.contentSearch.title')}</h2>

      <div style={columnsStyles}>
        <div style={columnStyles}>
          <p style={textStyles}>{t('settings.contentSearch.description')}</p>
          <p style={mutedStyles}>{t('settings.contentSearch.privacy')}</p>
        </div>

        <div style={cardStyles}>
          {model === null && <p style={mutedStyles}>{t('common.loading')}</p>}

          {model?.state === 'not_installed' && (
            <>
              <p style={textStyles}>{t('settings.contentSearch.notInstalled', { size: mb(model.size) })}</p>
              <div>
                <Button variant="primary" size="sm" onClick={() => run(downloadContentModel)}>
                  {downloadError ? t('settings.contentSearch.retry') : t('settings.contentSearch.download')}
                </Button>
              </div>
              {downloadError && <p style={errorStyles}>{downloadError}</p>}
            </>
          )}

          {model?.state === 'downloading' && (
            <>
              <p style={textStyles}>
                {t('settings.contentSearch.downloading', { done: mb(model.downloaded), total: mb(model.total) })}
              </p>
              <div style={barTrackStyles}>
                <div
                  style={{
                    height: '100%',
                    width: `${model.total > 0 ? (model.downloaded / model.total) * 100 : 0}%`,
                    background: 'var(--lumik-primary, #aac0f0)',
                    transition: 'width 0.2s',
                  }}
                />
              </div>
              <div>
                <Button variant="secondary" size="sm" onClick={() => run(cancelContentModelDownload)}>
                  {t('settings.contentSearch.cancel')}
                </Button>
              </div>
            </>
          )}

          {installed && (
            <>
              <p style={textStyles}>{t('settings.contentSearch.installed', { size: mb(model.size) })}</p>
              <div>
                <Button variant="secondary" size="sm" onClick={() => setConfirmDelete(true)}>
                  {t('settings.contentSearch.delete')}
                </Button>
              </div>
            </>
          )}
        </div>
      </div>

      {installed && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          <AnalysisModeSelect labelKey="settings.contentSearch.modeLabel" value={mode} onChange={changeMode} />
          <p style={mutedStyles}>{t('settings.contentSearch.modeHint')}</p>
        </div>
      )}

      {error && <p style={errorStyles}>{error}</p>}

      <Modal
        title={t('settings.contentSearch.deleteTitle')}
        open={confirmDelete}
        onClose={() => setConfirmDelete(false)}
        closable
        style={{ width: '440px' }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          <p style={textStyles}>{t('settings.contentSearch.deleteBody')}</p>
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '12px' }}>
            <Button variant="ghost" onClick={() => setConfirmDelete(false)}>{t('settings.contentSearch.cancel')}</Button>
            <Button
              variant="primary"
              onClick={() => {
                setConfirmDelete(false);
                run(deleteContentModel);
              }}
            >
              {t('settings.contentSearch.delete')}
            </Button>
          </div>
        </div>
      </Modal>
    </section>
  );
}
