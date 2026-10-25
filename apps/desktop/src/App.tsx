import { useState, useEffect } from 'react';
import { Layout, type Section } from './components';
import { Projects, SettingsPage, ProjectDetail, AboutPage, SearchPage, type SettingsTab } from './pages';
import { getProjectsDashboard } from './lib/api';
import { useConnectedDevices } from './lib/hooks';
import { BackupProvider } from './lib/BackupContext';
import { BackupSessionProvider } from './lib/BackupSessionContext';
import { BackupModalHost } from './components/BackupModalHost';
import { BackupCloseGuard } from './components/BackupCloseGuard';
import type { ProjectDashboard } from './lib/types';

function App() {
  const [activeSection, setActiveSection] = useState<Section>('projects');
  const [selectedProject, setSelectedProject] = useState<ProjectDashboard | null>(null);
  // Foto con la que se abre el proyecto (al venir de la búsqueda).
  const [initialPhotoId, setInitialPhotoId] = useState<string | null>(null);
  // Pestaña con la que se abre Ajustes; solo se fija al saltar a una concreta.
  const [settingsTab, setSettingsTab] = useState<SettingsTab | undefined>(undefined);
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
    setInitialPhotoId(null);
    setSettingsTab(undefined);
  };

  // Desde la búsqueda: abre el proyecto en esa foto (su disco está conectado).
  const openSearchResult = async (projectId: string, photoId: string) => {
    const project = (await getProjectsDashboard()).find((p) => p.id === projectId);
    if (!project) return;
    setActiveSection('projects');
    setInitialPhotoId(photoId);
    setSelectedProject(project);
  };

  const openSettings = (tab: SettingsTab) => {
    setActiveSection('settings');
    setSelectedProject(null);
    setSettingsTab(tab);
  };

  const renderPage = (section: Section) => {
    if (section === 'projects' && selectedProject) {
      return (
        <ProjectDetail
          projectId={selectedProject.id}
          projectName={selectedProject.name}
          deviceUuid={selectedProject.device_uuid}
          coverPhotoPath={selectedProject.cover_photo_path}
          initialPhotoId={initialPhotoId}
          onBack={() => { setSelectedProject(null); setInitialPhotoId(null); }}
          onCoverPhotoChange={(photoId) =>
            setSelectedProject((p) => p ? { ...p, cover_photo_path: photoId ? `.thumbs/${photoId}.jpg` : null } : p)
          }
        />
      );
    }

    switch (section) {
      case 'projects':
        return <Projects onProjectClick={setSelectedProject} />;
      case 'search':
        return <SearchPage onOpenPhoto={openSearchResult} />;
      case 'settings':
        return <SettingsPage initialTab={settingsTab} />;
      case 'about':
        return <AboutPage />;
      default:
        return <Projects onProjectClick={setSelectedProject} />;
    }
  };

  return (
    <BackupSessionProvider>
      <BackupProvider>
        <Layout activeSection={activeSection} onSectionChange={handleSectionChange}>
          {renderPage}
        </Layout>
        <BackupModalHost onOpenBackupSettings={() => openSettings('backup')} />
        <BackupCloseGuard />
      </BackupProvider>
    </BackupSessionProvider>
  );
}

export default App;
