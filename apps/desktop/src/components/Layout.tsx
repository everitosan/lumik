import { useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Sidebar } from './Sidebar';
import { usePlatform } from '../lib/hooks';

export type Section = 'projects' | 'settings' | 'about';

interface LayoutProps {
  children: ReactNode | ((section: Section) => ReactNode);
  activeSection?: Section;
  onSectionChange?: (section: Section) => void;
}

const layoutStyles: React.CSSProperties = {
  display: 'flex',
  height: '100vh',
  width: '100vw',
  overflow: 'hidden',
  position: 'relative',
};

const mainStyles: React.CSSProperties = {
  flex: 1,
  display: 'flex',
  flexDirection: 'column',
  overflow: 'hidden',
};

const sidebarTabStyles: React.CSSProperties = {
  position: 'absolute',
  left: 0,
  top: '60px',
  zIndex: 60,
  width: '20px',
  height: '56px',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderLeft: 'none',
  borderRadius: '0 10px 10px 0',
  background: 'var(--lumik-surface-container-low, #1c1b1b)',
  color: 'var(--lumik-on-surface, #e3e2e9)',
  cursor: 'pointer',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  fontSize: '14px',
  padding: 0,
};

export function Layout({ children, activeSection: controlledSection, onSectionChange }: LayoutProps) {
  const { t } = useTranslation();
  const platform = usePlatform();
  const isMobile = platform === 'android' || platform === 'ios';

  const [internalSection, setInternalSection] = useState<Section>('projects');
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean | null>(null);

  // Default: collapsed on mobile, expanded on desktop
  const collapsed = sidebarCollapsed ?? isMobile;

  const activeSection = controlledSection ?? internalSection;

  const handleSectionChange = (section: Section) => {
    if (controlledSection === undefined) setInternalSection(section);
    onSectionChange?.(section);
    // Auto-collapse sidebar after navigation on mobile
    if (isMobile) setSidebarCollapsed(true);
  };

  const renderContent = () => {
    if (typeof children === 'function') return children(activeSection);
    return children;
  };

  return (
    <div style={layoutStyles}>
      {/* Sidebar — on mobile overlays content when expanded */}
      <Sidebar
        activeSection={activeSection}
        onSectionChange={handleSectionChange}
        collapsed={collapsed}
        onToggle={() => setSidebarCollapsed(c => !(c ?? isMobile))}
        isMobile={isMobile}
      />

      {/* Backdrop to close sidebar on mobile */}
      {isMobile && !collapsed && (
        <div
          onClick={() => setSidebarCollapsed(true)}
          style={{
            position: 'absolute', inset: 0, zIndex: 49,
            background: 'rgba(0,0,0,0.5)',
          }}
        />
      )}

      {/* Tab to open sidebar when collapsed — glued to the left edge */}
      {collapsed && (
        <button
          style={sidebarTabStyles}
          onClick={() => setSidebarCollapsed(false)}
          aria-label={t('navigation.openMenu')}
        >
          ›
        </button>
      )}

      <main style={mainStyles}>
        {renderContent()}
      </main>
    </div>
  );
}
