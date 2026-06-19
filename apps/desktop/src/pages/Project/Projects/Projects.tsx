import { useState, useMemo, useEffect, useRef } from 'react';
import { SearchBar, ProjectCard, Button, Icon } from '@lumik/ui';
import { useProjectsDashboard, useActivePhotographer, useConnectedDevices, useCoverThumbnails, useContextKeybindings, matchesKey } from '../../../lib/hooks';
import { createProject } from '../../../lib/api';
import { CreateProjectModal, type ProjectFormData } from '../../../components/CreateProjectModal';
import type { ProjectDashboard } from '../../../lib/types';

const containerStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  height: '100%',
  padding: '24px 32px',
  gap: '24px',
  overflow: 'hidden',
};

const headerStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  flexShrink: 0,
};

const headerSectionStyles: React.CSSProperties = {
  flex: 1,
  display: 'flex',
  alignItems: 'center',
};

const headerLeftStyles: React.CSSProperties = {
  ...headerSectionStyles,
  flexDirection: 'column',
  alignItems: 'flex-start',
  gap: '4px',
};

const headerCenterStyles: React.CSSProperties = {
  ...headerSectionStyles,
  justifyContent: 'center',
};

const headerRightStyles: React.CSSProperties = {
  ...headerSectionStyles,
  justifyContent: 'flex-end',
};

const titleStyles: React.CSSProperties = {
  fontSize: '28px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  margin: 0,
};

const statsStyles: React.CSSProperties = {
  display: 'flex',
  gap: '24px',
  fontSize: '14px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const statItemStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
};

const statValueStyles: React.CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontWeight: 500,
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const gridContainerStyles: React.CSSProperties = {
  flex: 1,
  overflow: 'auto',
  paddingRight: '8px',
};

const gridStyles: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
  gap: '24px',
  paddingBottom: '24px',
};

const yearSeparatorStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'flex-end',
  paddingBottom: '20px',
  borderBottom: '1px solid rgba(255, 255, 255, 0.2)',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '18px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  marginBottom: '24px',
};

const yearSectionStyles: React.CSSProperties = {
  marginBottom: '40px',
};

const loadingStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  height: '200px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const emptyStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  height: '200px',
  gap: '16px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

interface ProjectsProps {
  onProjectClick?: (project: ProjectDashboard) => void;
}

export function Projects({ onProjectClick }: ProjectsProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  const kb = useContextKeybindings('projects');

  useEffect(() => {
    if (showCreateModal) return;
    function onKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (matchesKey(e, kb['projects.new_project'])) {
        e.preventDefault();
        setShowCreateModal(true);
      }
      if (matchesKey(e, kb['projects.focus_search'])) {
        e.preventDefault();
        searchRef.current?.focus();
      }
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [showCreateModal, kb]);

  const { data: projects, loading, error, refetch } = useProjectsDashboard();
  const { data: photographer } = useActivePhotographer();
  const { data: devices } = useConnectedDevices();
  const coverThumbnails = useCoverThumbnails(projects);

  // Refetch projects whenever a new device UUID appears in the connected list.
  // scan_connected_devices already called refresh_open_projects on the Rust side,
  // so we just need to re-query get_projects_dashboard to pick up the new entries.
  const knownDeviceUuids = useRef<Set<string>>(new Set());
  useEffect(() => {
    if (!devices) return;
    const hasNew = devices.some((d) => !knownDeviceUuids.current.has(d.uuid));
    knownDeviceUuids.current = new Set(devices.map((d) => d.uuid));
    if (hasNew) refetch();
  }, [devices, refetch]);

  const filteredProjects = useMemo(() => {
    if (!projects) return [];
    return projects.filter((project) =>
      project.name.toLowerCase().includes(searchQuery.toLowerCase())
    );
  }, [projects, searchQuery]);

  const projectsByYear = useMemo(() => {
    const groups = new Map<string, ProjectDashboard[]>();
    for (const project of filteredProjects) {
      const raw = project.session_date ?? project.created_at;
      const year = raw ? raw.slice(0, 4) : 'sin-fecha';
      const bucket = groups.get(year);
      if (bucket) bucket.push(project);
      else groups.set(year, [project]);
    }
    return [...groups.entries()]
      .sort(([a], [b]) => {
        if (a === 'sin-fecha') return 1;
        if (b === 'sin-fecha') return -1;
        return b.localeCompare(a); // newest year first
      })
      .map(([year, list]) => [
        year,
        [...list].sort((a, b) => {
          const da = a.session_date ?? a.created_at;
          const db = b.session_date ?? b.created_at;
          return da.localeCompare(db); // oldest first within the year
        }),
      ] as [string, typeof list]);
  }, [filteredProjects]);

  const totalPhotos = useMemo(() => {
    if (!projects) return 0;
    return projects.reduce((sum, p) => sum + p.photo_count, 0);
  }, [projects]);

  const handleProjectClick = (project: ProjectDashboard) => {
    onProjectClick?.(project);
  };

  const handleProjectMenu = (project: ProjectDashboard) => {
    console.log('Open menu for:', project.name);
    // TODO: Show context menu
  };

  const handleCreateProject = async (data: ProjectFormData) => {
    setIsCreating(true);
    setCreateError(null);

    try {
      await createProject({
        name: data.name,
        description: data.description || undefined,
        session_date: data.sessionDate || undefined,
        creator_id: photographer!.id,
        device_uuid: data.deviceUuid,
      });

      setShowCreateModal(false);
      refetch();
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : 'Error al crear el proyecto');
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <div style={containerStyles}>
      <div style={headerStyles}>
        <div style={headerLeftStyles}>
          <h1 style={titleStyles}>Projects</h1>
          <div style={statsStyles}>
            <span style={statItemStyles}>
              <span style={statValueStyles}>{projects?.length ?? 0}</span> projects
            </span>
            <span style={statItemStyles}>
              <span style={statValueStyles}>{totalPhotos.toLocaleString()}</span> photos
            </span>
          </div>
        </div>
        <div style={headerCenterStyles}>
          <SearchBar
            ref={searchRef}
            placeholder="Search projects..."
            onSearch={setSearchQuery}
          />
        </div>
        <div style={headerRightStyles}>
          <Button
            variant="primary"
            leftIcon={<Icon name="plus" size="sm" />}
            onClick={() => setShowCreateModal(true)}
          >
            New project
          </Button>
        </div>
      </div>

      <div style={gridContainerStyles}>
        {loading && (
          <div style={loadingStyles}>Loading projects...</div>
        )}

        {error && (
          <div style={loadingStyles}>Error: {error}</div>
        )}

        {!loading && !error && filteredProjects.length === 0 && (
          <div style={emptyStyles}>
            <span>No projects found</span>
            <span style={{ fontSize: '14px' }}>
              Create a new project to get started
            </span>
          </div>
        )}

        {!loading && !error && projectsByYear.map(([year, yearProjects]) => (
          <div key={year} style={yearSectionStyles}>
            <div style={yearSeparatorStyles}>
              {year === 'sin-fecha' ? 'Sin fecha' : year}
            </div>
            <div style={gridStyles}>
              {yearProjects.map((project) => (
                <ProjectCard
                  key={project.id}
                  name={project.name}
                  photoCount={project.photo_count}
                  date={project.session_date ?? project.created_at}
                  status={project.workflow_status}
                  driveName={devices?.find((d) => d.uuid === project.device_uuid)?.name ?? project.device_uuid}
                  thumbnailUrl={coverThumbnails[project.id]}
                  onClick={() => handleProjectClick(project)}
                  onMenuClick={() => handleProjectMenu(project)}
                />
              ))}
            </div>
          </div>
        ))}
      </div>

      <CreateProjectModal
        open={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        onSubmit={handleCreateProject}
        devices={devices ?? []}
        loading={isCreating}
        error={createError}
      />
    </div>
  );
}
