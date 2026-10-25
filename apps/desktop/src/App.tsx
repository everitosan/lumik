import { useState, useEffect } from 'react';
import { Layout, type Section } from './components';
import { Projects, SettingsPage, ProjectDetail, AboutPage, type SettingsTab } from './pages';
import { getProjectsDashboard } from './lib/api';
import { useConnectedDevices } from './lib/hooks';
import { BackupProvider } from './lib/BackupContext';
import { BackupSessionProvider } from './lib/BackupSessionContext';
import { BackupModalHost } from './components/BackupModalHost';
import { BackupCloseGuard } from './components/BackupCloseGuard';
import type { ProjectDashboard, SearchSnapshot } from './lib/types';

function App() {
  const [activeSection, setActiveSection] = useState<Section>('projects');
  const [selectedProject, setSelectedProject] = useState<ProjectDashboard | null>(null);
  // Foto con la que se abre el proyecto (al venir de la búsqueda).
  const [initialPhotoId, setInitialPhotoId] = useState<string | null>(null);
  // Búsqueda guardada: al cerrar una foto abierta desde ella se vuelve igual.
  const [searchSnapshot, setSearchSnapshot] = useState<SearchSnapshot | null>(null);
  // Pestaña con la que se abre Ajustes; solo se fija al saltar a una concreta.
  const [settingsTab, setSettingsTab] = useState<SettingsTab | undefined>(undefined);
  const { data: devices } = useConnectedDevices();

  // If the open project's device is ejected/disconnected, leave the detail view
  // and return to the dashboard — its data is no longer reachable.
  useEffect(() => {
    if (!selectedProject || !devices) return;
    const stillConnected = devices.some((d) => d.uuid === selectedProject.device_uuid);
    if (!stillConnected) {
      setSelectedProject(null);
      setInitialPhotoId(null);
    }
  }, [devices, selectedProject]);

  const handleSectionChange = (section: Section) => {
    setActiveSection(section);
    setSelectedProject(null);
    setInitialPhotoId(null);
    setSettingsTab(undefined);
  };

  // Desde el dashboard el proyecto se abre en su grid, sin volver a la búsqueda.
  const openFromDashboard = (project: ProjectDashboard) => {
    setInitialPhotoId(null);
    setSelectedProject(project);
  };

  const backToSearch = () => {
    setSelectedProject(null);
    setInitialPhotoId(null);
    setActiveSection('projects');
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
          onCloseInitialPhoto={initialPhotoId ? backToSearch : undefined}
          onBack={() => { setSelectedProject(null); setInitialPhotoId(null); }}
          onCoverPhotoChange={(photoId) =>
            setSelectedProject((p) => p ? { ...p, cover_photo_path: photoId ? `.thumbs/${photoId}.jpg` : null } : p)
          }
        />
      );
    }

    switch (section) {
      case 'projects':
        return <Projects onProjectClick={openFromDashboard} photoSearch={searchSnapshot} onPhotoSearch={setSearchSnapshot} onOpenPhoto={openSearchResult} />;
      case 'settings':
        return <SettingsPage initialTab={settingsTab} />;
      case 'about':
        return <AboutPage />;
      default:
        return <Projects onProjectClick={openFromDashboard} photoSearch={searchSnapshot} onPhotoSearch={setSearchSnapshot} onOpenPhoto={openSearchResult} />;
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
