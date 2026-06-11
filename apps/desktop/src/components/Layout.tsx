import { useState, type ReactNode } from 'react';
import { Sidebar } from './Sidebar';

export type Section = 'projects' | 'settings';

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
};

const mainStyles: React.CSSProperties = {
  flex: 1,
  display: 'flex',
  flexDirection: 'column',
  overflow: 'hidden',
};

export function Layout({ children, activeSection: controlledSection, onSectionChange }: LayoutProps) {
  const [internalSection, setInternalSection] = useState<Section>('projects');

  const activeSection = controlledSection ?? internalSection;

  const handleSectionChange = (section: Section) => {
    if (controlledSection === undefined) setInternalSection(section);
    onSectionChange?.(section);
  };

  const renderContent = () => {
    if (typeof children === 'function') {
      return children(activeSection);
    }
    return children;
  };

  return (
    <div style={layoutStyles}>
      <Sidebar
        activeSection={activeSection}
        onSectionChange={handleSectionChange}
      />
      <main style={mainStyles}>
        {renderContent()}
      </main>
    </div>
  );
}
