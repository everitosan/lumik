import { useState, useEffect } from 'react';
import { Layout, type Section } from './components';
import { Projects, SettingsPage, ProjectDetail, AboutPage } from './pages';
import { useConnectedDevices } from './lib/hooks';
import type { ProjectDashboard } from './lib/types';

function App() {
  const [activeSection, setActiveSection] = useState<Section>('projects');
  const [selectedProject, setSelectedProject] = useState<ProjectDashboard | null>(null);
  const { data: devices } = useConnectedDevices();

  // If the open project's device is ejected/disconnected, leave the detail view
  // and return to the dashboard — its data is no longer reachable.
  useEffect(() => {
    if (!selectedProject || !devices) return;
    const stillConnected = devices.some((d) => d.uuid === selectedProject.device_uuid);
    if (!stillConnected) setSelectedProject(null);
  }, [devices, selectedProject]);

  const handleSectionChange = (section: Section) => {
    setActiveSection(section);
    setSelectedProject(null);
  };

  const renderPage = (section: Section) => {
    if (section === 'projects' && selectedProject) {
      return (
        <ProjectDetail
          projectId={selectedProject.id}
          projectName={selectedProject.name}
          deviceUuid={selectedProject.device_uuid}
          coverPhotoPath={selectedProject.cover_photo_path}
          onBack={() => setSelectedProject(null)}
          onCoverPhotoChange={(photoId) =>
            setSelectedProject((p) => p ? { ...p, cover_photo_path: photoId ? `.thumbs/${photoId}.jpg` : null } : p)
          }
        />
      );
    }

    switch (section) {
      case 'projects':
        return <Projects onProjectClick={setSelectedProject} />;
      case 'settings':
        return <SettingsPage />;
      case 'about':
        return <AboutPage />;
      default:
        return <Projects onProjectClick={setSelectedProject} />;
    }
  };

  return (
    <Layout activeSection={activeSection} onSectionChange={handleSectionChange}>
      {renderPage}
    </Layout>
  );
}

export default App;
