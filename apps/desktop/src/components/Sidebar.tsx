import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Logo, SectionButton, DriveInfo, CollapsibleSection, BackupJobCard, Icon } from '@lumik/ui';
import { useConnectedDevices, useAnalysisJobs, type AnalysisJob } from '../lib/hooks';
import * as api from '../lib/api';
import { useBackup } from '../lib/BackupContext';
import type { Section } from './Layout';

interface SidebarProps {
  activeSection: Section;
  onSectionChange: (section: Section) => void;
  collapsed?: boolean;
}

const sidebarStyles = (collapsed: boolean): React.CSSProperties => ({
  display: 'flex',
  flexDirection: 'column',
  width: collapsed ? '0px' : '260px',
  minWidth: collapsed ? '0px' : '260px',
  height: '100%',
  backgroundColor: 'var(--lumik-surface-container-low, #1c1b1b)',
  borderRight: collapsed ? 'none' : '1px solid var(--lumik-outline-variant, #424654)',
  padding: collapsed ? '0' : '24px 16px',
  gap: '24px',
  overflow: 'hidden',
  transition: 'width 0.2s ease, min-width 0.2s ease, padding 0.2s ease',
});

const logoContainerStyles: React.CSSProperties = {
  padding: '0 8px',
};

const navStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
};

const sectionTitleStyles: React.CSSProperties = {
  fontSize: '11px',
  fontWeight: 600,
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  padding: '8px 16px 4px',
};

const spacerStyles: React.CSSProperties = {
  flex: 1,
};

const emptyStorageStyles: React.CSSProperties = {
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  padding: '8px 16px',
  opacity: 0.7,
};

// La lista de respaldos crece hasta este alto y luego hace scroll propio, para
// que Almacenamiento siga visible debajo.
const backupListStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  maxHeight: '40vh',
  overflowY: 'auto',
};

const analysisCardStyles = (hasError: boolean): React.CSSProperties => ({
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
  padding: '10px 12px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  border: `1px solid ${hasError ? 'var(--lumik-error, #ffb4ab)' : 'var(--lumik-outline-variant, #424654)'}`,
  fontFamily: 'var(--lumik-font-primary, Inter)',
});

const analysisIconButtonStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: '22px',
  height: '22px',
  padding: 0,
  background: 'transparent',
  border: 'none',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  cursor: 'pointer',
  flexShrink: 0,
};

/**
 * Análisis de las fotos de un proyecto (semantic-search): en espera, avance con
 * "Detener", o cómo terminó con una × para quitarlo.
 */
function AnalysisJobCard({ name, job, onStop, onDismiss }: { name: string; job: AnalysisJob; onStop: () => void; onDismiss: () => void }) {
  const { t } = useTranslation();
  const active = job.status === 'running' || job.status === 'queued';
  const hasError = job.status === 'failed' || job.status === 'locked' || job.status === 'no_model';
  const line = (() => {
    switch (job.status) {
      case 'queued':
        return t('navigation.analysis.queued');
      case 'running':
        return job.total > 0 ? t('navigation.analysis.running', { done: job.done, total: job.total }) : t('navigation.analysis.starting');
      case 'done':
        return job.failed > 0
          ? t('navigation.analysis.doneWithFailures', { count: job.analyzed, failed: job.failed })
          : t('navigation.analysis.done', { count: job.analyzed });
      case 'cancelled':
        return t('navigation.analysis.cancelled', { count: job.analyzed });
      default:
        return t(`navigation.analysis.${job.status}`);
    }
  })();
  return (
    <div style={analysisCardStyles(hasError)}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
        <span style={{ flex: 1, minWidth: 0, fontSize: '13px', fontWeight: 500, color: 'var(--lumik-on-surface, #e5e2e1)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={name}>
          {name}
        </span>
        <button
          type="button"
          style={analysisIconButtonStyles}
          onClick={active ? onStop : onDismiss}
          title={active ? t('navigation.analysis.stop') : t('navigation.analysis.dismiss')}
          aria-label={active ? t('navigation.analysis.stop') : t('navigation.analysis.dismiss')}
        >
          <Icon name="x" size={14} />
        </button>
      </div>
      {job.status === 'running' && job.total > 0 && (
        <div style={{ height: '4px', borderRadius: '2px', background: 'var(--lumik-surface-container-highest, #353534)', overflow: 'hidden' }}>
          <div style={{ height: '100%', width: `${(job.done / job.total) * 100}%`, background: 'var(--lumik-primary, #aac0f0)', transition: 'width 0.2s' }} />
        </div>
      )}
      <span style={{ fontSize: '12px', color: hasError ? 'var(--lumik-error, #ffb4ab)' : 'var(--lumik-on-surface-variant, #c2c6d7)' }}>{line}</span>
    </div>
  );
}


export function Sidebar({ activeSection, onSectionChange, collapsed = false }: SidebarProps) {
  const { t } = useTranslation();
  const { data: devices, loading, eject } = useConnectedDevices();

  const { jobs, retryBackup, dismissBackup, userLaunches, openBackupDetail } = useBackup();

  // Análisis de fotos (semantic-search): nombres de los proyectos que aparecen.
  const { jobs: analysisJobs, dismiss: dismissAnalysis } = useAnalysisJobs();
  const [projectNames, setProjectNames] = useState<Record<string, string>>({});
  const [analysisCollapsed, setAnalysisCollapsed] = useState(false);
  const analysisIds = Object.keys(analysisJobs);
  useEffect(() => {
    if (analysisIds.every((id) => id in projectNames)) return;
    api
      .getProjectsDashboard()
      .then((list) => setProjectNames(Object.fromEntries(list.map((p) => [p.id, p.name]))))
      .catch(() => undefined);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [analysisIds.join(',')]);
  // Motivo por el que no se pudo reintentar, por proyecto (p. ej. disco ausente).
  const [retryMessages, setRetryMessages] = useState<Record<string, string>>({});

  // Secciones colapsables: el estado vive en la configuración de la instalación.
  const [sections, setSections] = useState<api.SidebarSections>({
    backups_collapsed: false,
    storage_collapsed: false,
  });
  useEffect(() => {
    api.getSidebarSections().then(setSections).catch(() => {});
  }, []);
  const setSectionCollapsed = (section: api.SidebarSection, value: boolean) => {
    setSections((s) => ({ ...s, [`${section}_collapsed`]: value }));
    api.setSidebarSectionCollapsed(section, value).catch((err) =>
      console.error('saving sidebar section failed', err)
    );
  };

  // Un respaldo lanzado por el usuario expande Respaldos (los reanudados al
  // arrancar no). Si luego la colapsa, se respeta.
  const seenLaunches = useRef(userLaunches);
  useEffect(() => {
    if (userLaunches === seenLaunches.current) return;
    seenLaunches.current = userLaunches;
    if (sections.backups_collapsed) setSectionCollapsed('backups', false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [userLaunches]);

  const handleEject = async (uuid: string) => {
    try {
      await eject(uuid);
    } catch (err) {
      // Surface the failure; the device stays listed so the user can retry.
      console.error('eject failed', err);
      alert(t('navigation.ejectError', { error: String(err) }));
    }
  };

  const getUsedBytes = (device: { total_bytes: number | null; available_bytes: number | null }) => {
    if (device.total_bytes === null || device.available_bytes === null) return 0;
    return device.total_bytes - device.available_bytes;
  };

  const jobEntries = Object.entries(jobs);
  const running = jobEntries.filter(([, job]) => job.running);
  const withErrors = jobEntries.filter(
    ([, job]) => !job.running && job.files.some((f) => f.status === 'error')
  );

  const backupsSummary = (() => {
    if (running.length > 0) {
      const done = running.reduce((n, [, j]) => n + j.files.filter((f) => f.status === 'done').length, 0);
      const total = running.reduce((n, [, j]) => n + j.files.length, 0);
      return t('navigation.backups.summaryRunning', { count: running.length, done, total });
    }
    if (withErrors.length > 0) return t('navigation.backups.summaryFailed', { count: withErrors.length });
    return undefined;
  })();

  const handleRetry = async (projectId: string, projectName: string) => {
    setRetryMessages(({ [projectId]: _old, ...rest }) => rest);
    const result = await retryBackup(projectId);
    if (!result.ok && result.reason === 'not_mounted') {
      setRetryMessages((m) => ({ ...m, [projectId]: t('navigation.backups.notMounted', { name: projectName }) }));
    }
  };

  const handleDismiss = (projectId: string) => {
    setRetryMessages(({ [projectId]: _old, ...rest }) => rest);
    dismissBackup(projectId);
  };

  return (
    <aside style={sidebarStyles(collapsed)}>
      <div style={logoContainerStyles}>
        <Logo size="md" />
      </div>

      <nav style={navStyles}>
        <div style={sectionTitleStyles}>{t('navigation.management')}</div>
        <SectionButton
          icon="projects"
          label={t('navigation.projects')}
          active={activeSection === 'projects'}
          onClick={() => onSectionChange('projects')}
        />
        <SectionButton
          icon="album"
          label={t('navigation.albums')}
          active={activeSection === 'albums'}
          onClick={() => onSectionChange('albums')}
        />
        <SectionButton
          icon="settings"
          label={t('navigation.settings')}
          active={activeSection === 'settings'}
          onClick={() => onSectionChange('settings')}
        />
        <SectionButton
          icon="info"
          label={t('navigation.about')}
          active={activeSection === 'about'}
          onClick={() => onSectionChange('about')}
        />
      </nav>

      <div style={spacerStyles} />

      {/* Respaldos solo aparece mientras hay jobs (en curso o con errores); la
          cuenta se gestiona en Ajustes → Respaldo. */}
      {jobEntries.length > 0 && (
        <CollapsibleSection
          title={t('navigation.backups.title')}
          summary={sections.backups_collapsed ? backupsSummary : undefined}
          collapsed={sections.backups_collapsed}
          onToggle={() => setSectionCollapsed('backups', !sections.backups_collapsed)}
        >
          <div style={backupListStyles}>
            {jobEntries.map(([projectId, job]) => (
              <BackupJobCard
                key={projectId}
                name={job.projectName}
                done={job.files.filter((f) => f.status === 'done').length}
                total={job.files.length}
                failed={job.files.filter((f) => f.status === 'error').length}
                running={job.running}
                message={retryMessages[projectId]}
                onClick={() => openBackupDetail(projectId, job.projectName)}
                onRetry={() => handleRetry(projectId, job.projectName)}
                onDismiss={() => handleDismiss(projectId)}
              />
            ))}
          </div>
        </CollapsibleSection>
      )}

      {analysisIds.length > 0 && (
        <CollapsibleSection
          title={t('navigation.analysis.title')}
          summary={analysisCollapsed ? t('navigation.analysis.summary', { count: analysisIds.length }) : undefined}
          collapsed={analysisCollapsed}
          onToggle={() => setAnalysisCollapsed((c) => !c)}
        >
          <div style={backupListStyles}>
            {analysisIds.map((id) => (
              <AnalysisJobCard
                key={id}
                name={projectNames[id] ?? '…'}
                job={analysisJobs[id]}
                onStop={() => void api.stopProjectAnalysis(id)}
                onDismiss={() => dismissAnalysis(id)}
              />
            ))}
          </div>
        </CollapsibleSection>
      )}

      <CollapsibleSection
        title={t('navigation.storage')}
        summary={
          sections.storage_collapsed && devices
            ? t('navigation.storageSummary', { count: devices.length })
            : undefined
        }
        collapsed={sections.storage_collapsed}
        onToggle={() => setSectionCollapsed('storage', !sections.storage_collapsed)}
      >
        {loading && (
          <div style={emptyStorageStyles}>{t('navigation.loadingDevices')}</div>
        )}
        {!loading && (!devices || devices.length === 0) && (
          <div style={emptyStorageStyles}>{t('navigation.noDevices')}</div>
        )}
        {!loading && devices && devices.map((device) => (
          <DriveInfo
            key={device.uuid}
            name={device.name}
            uuid={device.uuid}
            usedBytes={getUsedBytes(device)}
            totalBytes={device.total_bytes ?? 0}
            connected={true}
            readOnly={device.read_only}
            onEject={() => handleEject(device.uuid)}
          />
        ))}
      </CollapsibleSection>
    </aside>
  );
}
