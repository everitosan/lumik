import type { CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '@lumik/ui';
import type { AnalysisJob } from '../../../lib/hooks';


export interface ProjectDetailHeaderProps {
  projectName: string;
  onBack?: () => void;
  viewMode: 'grid' | 'by-date';
  onViewModeChange: (mode: 'grid' | 'by-date') => void;
  onImport?: () => void;
  onBackup?: () => void;
  backingUp?: boolean;
  backupStatus?: string | null;
  /** Análisis de las fotos (semantic-search); sin modelo instalado no se pasa. */
  analysis?: {
    /** Fotos sin analizar */
    pending: number;
    job?: AnalysisJob;
    onAnalyze: () => void;
    onStop: () => void;
  };
}

const outlineButtonStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  padding: '6px 12px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  background: 'transparent',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  cursor: 'pointer',
  whiteSpace: 'nowrap',
};

/** "Analizar fotos (N)", o el avance con "Detener" mientras se analizan. */
function AnalyzeButton({ pending, job, onAnalyze, onStop }: NonNullable<ProjectDetailHeaderProps['analysis']>) {
  const { t } = useTranslation();
  if (job?.status === 'running' || job?.status === 'queued') {
    const label =
      job.status === 'queued'
        ? t('project.analysis.queued')
        : t('project.analysis.running', { done: job.done, total: job.total });
    return (
      <button style={outlineButtonStyles} onClick={onStop} title={t('project.analysis.stop')}>
        <Icon name="x" size={14} />
        {label}
      </button>
    );
  }
  if (pending === 0) return null;
  return (
    <button style={outlineButtonStyles} onClick={onAnalyze} title={t('project.analysis.analyzeHint')}>
      <Icon name="search" size={14} />
      {t('project.analysis.analyze', { count: pending })}
    </button>
  );
}

const headerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '16px',
  height: '52px',
  padding: '0 32px',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
  flexShrink: 0,
  boxSizing: 'border-box',
};

const breadcrumbStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  flex: 1,
  minWidth: 0,
};

const breadcrumbLinkStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  background: 'none',
  border: 'none',
  padding: 0,
  cursor: 'pointer',
  flexShrink: 0,
};

const breadcrumbSepStyles: CSSProperties = {
  color: 'var(--lumik-outline, #8c90a0)',
  flexShrink: 0,
};

const breadcrumbCurrentStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};

const controlsStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  flexShrink: 0,
};


export function ProjectDetailHeader({
  projectName,
  onBack,
  viewMode,
  onViewModeChange,
  onImport,
  onBackup,
  backingUp,
  backupStatus,
  analysis,
}: ProjectDetailHeaderProps) {
  const { t } = useTranslation();

  return (
    <header style={headerStyles}>
      <nav style={breadcrumbStyles} aria-label="breadcrumb">
        <button style={breadcrumbLinkStyles} onClick={onBack}>
          {t('navigation.projects')}
        </button>
        <span style={breadcrumbSepStyles}>
          <Icon name="chevron-right" size={14} />
        </span>
        <span style={breadcrumbCurrentStyles} title={projectName}>
          {projectName}
        </span>
      </nav>

      <div style={controlsStyles}>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '0',
            border: '1px solid var(--lumik-outline-variant, #424654)',
            borderRadius: 'var(--lumik-radius-sm, 4px)',
            overflow: 'hidden',
          }}
        >
          <button
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '6px 10px',
              fontFamily: 'var(--lumik-font-primary, Inter)',
              fontSize: '13px',
              color: viewMode === 'by-date' ? 'var(--lumik-on-surface, #e5e2e1)' : 'var(--lumik-on-surface-variant, #c2c6d7)',
              background: viewMode === 'by-date' ? 'rgba(255, 255, 255, 0.1)' : 'transparent',
              border: 'none',
              borderRadius: '3px',
              cursor: 'pointer',
              transition: 'all 0.15s',
            }}
            onClick={() => onViewModeChange('by-date')}
            title={t('projectDetail.viewMode.groupByDate')}
          >
            <Icon name="calendar" size={14} />
          </button>

          <button
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '6px 10px',
              fontFamily: 'var(--lumik-font-primary, Inter)',
              fontSize: '13px',
              color: viewMode === 'grid' ? 'var(--lumik-on-surface, #e5e2e1)' : 'var(--lumik-on-surface-variant, #c2c6d7)',
              background: viewMode === 'grid' ? 'rgba(255, 255, 255, 0.1)' : 'transparent',
              border: 'none',
              borderRadius: '3px',
              cursor: 'pointer',
              transition: 'all 0.15s',
            }}
            onClick={() => onViewModeChange('grid')}
            title={t('projectDetail.viewMode.gridView')}
          >
            <Icon name="projects" size={14} />
          </button>
        </div>

        {analysis && <AnalyzeButton {...analysis} />}

        {onBackup && (
          <>
            {backupStatus && (
              <span
                style={{
                  fontSize: '12px',
                  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
                  whiteSpace: 'nowrap',
                }}
              >
                {backupStatus}
              </span>
            )}
            <button
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                padding: '6px 12px',
                fontFamily: 'var(--lumik-font-primary, Inter)',
                fontSize: '13px',
                color: 'var(--lumik-on-surface, #e5e2e1)',
                background: 'transparent',
                border: '1px solid var(--lumik-outline-variant, #424654)',
                borderRadius: 'var(--lumik-radius-sm, 4px)',
                cursor: backingUp ? 'default' : 'pointer',
                opacity: backingUp ? 0.6 : 1,
                whiteSpace: 'nowrap',
              }}
              onClick={onBackup}
              disabled={backingUp}
            >
              <Icon name="upload-cloud" size={14} />
              {backingUp ? t('backup.uploading') : t('backup.button')}
            </button>
          </>
        )}

        {onImport && (
        <button
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            padding: '6px 12px',
            fontFamily: 'var(--lumik-font-primary, Inter)',
            fontSize: '13px',
            color: 'var(--lumik-on-primary, #fff)',
            background: 'var(--lumik-primary, #1a73e8)',
            border: 'none',
            borderRadius: 'var(--lumik-radius-sm, 4px)',
            cursor: 'pointer',
            whiteSpace: 'nowrap',
          }}
          onClick={onImport}
        >
          <Icon name="import" size={14} />
          {t('import.title')}
        </button>
        )}
      </div>
    </header>
  );
}
