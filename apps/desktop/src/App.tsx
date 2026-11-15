import { useState, useEffect } from 'react';
import { Layout, type Section } from './components';
import { Projects, SettingsPage, ProjectDetail, AboutPage, AlbumsPage, AlbumDetail, TrashPage, type SettingsTab } from './pages';
import { useConnectedDevices } from './lib/hooks';
import { BackupProvider } from './lib/BackupContext';
import { BackupSessionProvider } from './lib/BackupSessionContext';
import { BackupModalHost } from './components/BackupModalHost';
import { BackupCloseGuard } from './components/BackupCloseGuard';
import { TrashProvider, useTrash } from './lib/TrashContext';
import type { ProjectDashboard, SearchSnapshot } from './lib/types';

function AppContent() {
  const [activeSection, setActiveSection] = useState<Section>('projects');
  const [selectedProject, setSelectedProject] = useState<ProjectDashboard | null>(null);
  // Álbum abierto en la sección Álbumes.
  const [selectedAlbumId, setSelectedAlbumId] = useState<string | null>(null);
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
    if (!stillConnected) setSelectedProject(null);
  }, [devices, selectedProject]);

  // La papelera solo existe si hay algo: al vaciarse estando en ella, se vuelve a Proyectos.
  const { disks: trashDisks, count: trashItems } = useTrash();
  useEffect(() => {
    if (activeSection === 'trash' && trashDisks !== null && trashItems === 0) setActiveSection('projects');
  }, [activeSection, trashDisks, trashItems]);

  const handleSectionChange = (section: Section) => {
    setActiveSection(section);
    setSelectedProject(null);
    setSelectedAlbumId(null);
    setSettingsTab(undefined);
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
          onBack={() => setSelectedProject(null)}
          onCoverPhotoChange={(photoId) =>
            setSelectedProject((p) => p ? { ...p, cover_photo_path: photoId ? `.thumbs/${photoId}.jpg` : null } : p)
          }
        />
      );
    }

    switch (section) {
      case 'projects':
        return <Projects onProjectClick={setSelectedProject} photoSearch={searchSnapshot} onPhotoSearch={setSearchSnapshot} />;
      case 'albums':
        return selectedAlbumId ? (
          <AlbumDetail key={selectedAlbumId} albumId={selectedAlbumId} onBack={() => setSelectedAlbumId(null)} />
        ) : (
          <AlbumsPage onOpenAlbum={setSelectedAlbumId} />
        );
      case 'trash':
        return <TrashPage />;
      case 'settings':
        return <SettingsPage initialTab={settingsTab} />;
      case 'about':
        return <AboutPage />;
      default:
        return <Projects onProjectClick={setSelectedProject} photoSearch={searchSnapshot} onPhotoSearch={setSearchSnapshot} />;
    }
  };

  return (
    <>
      <Layout activeSection={activeSection} onSectionChange={handleSectionChange}>
        {renderPage}
      </Layout>
      <BackupModalHost onOpenBackupSettings={() => openSettings('backup')} />
      <BackupCloseGuard />
    </>
  );
}

function App() {
  return (
    <BackupSessionProvider>
      <BackupProvider>
        <TrashProvider>
          <AppContent />
        </TrashProvider>
      </BackupProvider>
    </BackupSessionProvider>
  );
}

export default App;
