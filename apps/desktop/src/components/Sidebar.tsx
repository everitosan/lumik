import { Logo, SectionButton, DriveInfo } from '@lumik/ui';
import { useConnectedDevices } from '../lib/hooks';
import type { Section } from './Layout';

interface SidebarProps {
  activeSection: Section;
  onSectionChange: (section: Section) => void;
}

const sidebarStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  width: '260px',
  height: '100%',
  backgroundColor: 'var(--lumik-surface-container-low, #1c1b1b)',
  borderRight: '1px solid var(--lumik-outline-variant, #424654)',
  padding: '24px 16px',
  gap: '24px',
};

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

export function Sidebar({ activeSection, onSectionChange }: SidebarProps) {
  const { data: devices, loading } = useConnectedDevices();

  const getUsedBytes = (device: { total_bytes: number | null; available_bytes: number | null }) => {
    if (device.total_bytes === null || device.available_bytes === null) return 0;
    return device.total_bytes - device.available_bytes;
  };

  return (
    <aside style={sidebarStyles}>
      <div style={logoContainerStyles}>
        <Logo size="md" />
      </div>

      <nav style={navStyles}>
        <div style={sectionTitleStyles}>Management</div>
        <SectionButton
          icon="projects"
          label="Projects"
          active={activeSection === 'projects'}
          onClick={() => onSectionChange('projects')}
        />
        <SectionButton
          icon="settings"
          label="Settings"
          active={activeSection === 'settings'}
          onClick={() => onSectionChange('settings')}
        />
        <SectionButton
          icon="info"
          label="About"
          active={activeSection === 'about'}
          onClick={() => onSectionChange('about')}
        />
      </nav>

      <div style={spacerStyles} />

      <div style={storageContainerStyles}>
        <div style={sectionTitleStyles}>Storage</div>
        {loading && (
          <div style={emptyStorageStyles}>Loading devices...</div>
        )}
        {!loading && (!devices || devices.length === 0) && (
          <div style={emptyStorageStyles}>No devices connected</div>
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
