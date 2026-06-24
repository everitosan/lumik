import { useState } from 'react';
import { Layout, type Section } from './components';
import { Projects, SettingsPage, ProjectDetail, AboutPage } from './pages';
import type { ProjectDashboard } from './lib/types';

function App() {
  const [activeSection, setActiveSection] = useState<Section>('projects');
  const [selectedProject, setSelectedProject] = useState<ProjectDashboard | null>(null);

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
