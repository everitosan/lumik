import { useTranslation } from 'react-i18next';
import { Logo, SectionButton, DriveInfo } from '@lumik/ui';
import { useConnectedDevices } from '../lib/hooks';
import type { Section } from './Layout';

interface SidebarProps {
  activeSection: Section;
  onSectionChange: (section: Section) => void;
  collapsed?: boolean;
  onToggle?: () => void;
  isMobile?: boolean;
}

const sidebarStyles = (collapsed: boolean, isMobile: boolean): React.CSSProperties => ({
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
  // On mobile, sidebar overlays the content
  ...(isMobile && !collapsed ? {
    position: 'absolute',
    left: 0,
    top: 0,
    zIndex: 50,
    height: '100vh',
  } : {}),
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

const storageContainerStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
};

const emptyStorageStyles: React.CSSProperties = {
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  padding: '8px 16px',
  opacity: 0.7,
};

const closeBtnStyles: React.CSSProperties = {
  alignSelf: 'flex-end',
  width: '36px',
  height: '36px',
  borderRadius: '8px',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  background: 'transparent',
  color: 'var(--lumik-on-surface, #e3e2e9)',
  cursor: 'pointer',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  fontSize: '18px',
  flexShrink: 0,
};

export function Sidebar({ activeSection, onSectionChange, collapsed = false, onToggle, isMobile = false }: SidebarProps) {
  const { t } = useTranslation();
  const { data: devices, loading } = useConnectedDevices();

  const getUsedBytes = (device: { total_bytes: number | null; available_bytes: number | null }) => {
    if (device.total_bytes === null || device.available_bytes === null) return 0;
    return device.total_bytes - device.available_bytes;
  };

  return (
    <aside style={sidebarStyles(collapsed, isMobile)}>
      <div style={{ display: 'flex', alignItems: 'center' }}>
        <div style={{ ...logoContainerStyles, flex: 1 }}>
          <Logo size="md" />
        </div>
        {isMobile && onToggle && (
          <button style={closeBtnStyles} onClick={onToggle} aria-label={t('navigation.closeMenu')}>✕</button>
        )}
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

      <div style={storageContainerStyles}>
        <div style={sectionTitleStyles}>{t('navigation.storage')}</div>
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
          />
        ))}
      </div>
    </aside>
  );
}
