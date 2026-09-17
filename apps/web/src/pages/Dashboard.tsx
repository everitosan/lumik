import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ProjectCard, type WorkflowStatus } from '@lumik/ui';
import {
  catalogApi,
  backupApi,
  getProjectCoverUrl,
  type CatalogProject,
} from '../api/client';

/** Abre la carpeta del backup: enlace expirable si el proveedor lo soporta; si no, su web. */
async function openBackupLocation(projectId: string, homeUrl: string, provider: string) {
  try {
    const { url } = await backupApi.folderLink(projectId, provider);
    window.open(url ?? homeUrl, '_blank', 'noopener,noreferrer');
  } catch {
    window.open(homeUrl, '_blank', 'noopener,noreferrer');
  }
}
import { ProjectBackupModal } from '../components/ProjectBackupModal';

const VALID_STATUS = ['imported', 'edited', 'delivered'] as const;

function statusOf(p: CatalogProject): WorkflowStatus {
  // Estado fijado por la desktop (tras respaldar) tiene prioridad.
  if (p.workflow_status && (VALID_STATUS as readonly string[]).includes(p.workflow_status)) {
    return p.workflow_status as WorkflowStatus;
  }
  const total = p.photo_count ?? 0;
  const up = p.backed_up ?? 0;
  if (total > 0 && up >= total) return 'delivered';
  if (up > 0) return 'edited';
  return 'imported';
}

const providersOf = (p: CatalogProject) =>
  (p.providers ?? '').split(',').map((s) => s.trim()).filter(Boolean);

const dateOf = (p: CatalogProject) => p.session_date ?? p.created_at ?? '';
const yearOf = (p: CatalogProject) => dateOf(p).slice(0, 4) || '—';

/** Card que carga su propia portada (fetch con auth → objectURL). */
function ProjectCardWithCover({
  project,
  onClick,
}: {
  project: CatalogProject;
  onClick: () => void;
}) {
  const { t } = useTranslation();
  const [cover, setCover] = useState<string | undefined>(undefined);

  useEffect(() => {
    let url: string | null = null;
    getProjectCoverUrl(project.id).then((u) => {
      url = u;
      setCover(u ?? undefined);
    });
    return () => {
      if (url) URL.revokeObjectURL(url);
    };
  }, [project.id]);

  return (
    <ProjectCard
      name={project.name}
      photoCount={project.photo_count ?? 0}
      date={project.session_date ?? ''}
      driveName={t('projects.card.backedUp', {
        up: project.backed_up ?? 0,
        total: project.photo_count ?? 0,
      })}
      status={statusOf(project)}
      thumbnailUrl={cover}
      providers={providersOf(project)}
      onProviderOpen={(url, provider) => openBackupLocation(project.id, url, provider)}
      onClick={onClick}
    />
  );
}

export default function Dashboard() {
  const { t } = useTranslation();
  const [projects, setProjects] = useState<CatalogProject[]>([]);
  const [loading, setLoading] = useState(true);
  const [active, setActive] = useState<CatalogProject | null>(null);

  useEffect(() => {
    catalogApi
      .listProjects()
      .then(setProjects)
      .catch(() => setProjects([]))
      .finally(() => setLoading(false));
  }, []);

  // Agrupado por año: años DESC; dentro de cada año, ASC por fecha (como la desktop).
  const byYear = useMemo(() => {
    const groups = new Map<string, CatalogProject[]>();
    for (const p of projects) {
      const y = yearOf(p);
      if (!groups.has(y)) groups.set(y, []);
      groups.get(y)!.push(p);
    }
    const years = [...groups.keys()].sort((a, b) => b.localeCompare(a));
    return years.map((year) => ({
      year,
      items: groups.get(year)!.sort((a, b) => dateOf(a).localeCompare(dateOf(b))),
    }));
  }, [projects]);

  return (
    <div className="page">
      <h1>{t('projects.title')}</h1>
      {loading ? (
        <p className="subtitle">{t('common.loading')}</p>
      ) : projects.length === 0 ? (
        <p className="subtitle">{t('projects.empty')}</p>
      ) : (
        byYear.map(({ year, items }) => (
          <section key={year} className="year-group">
            <h2 className="year-title">{year}</h2>
            <div className="projects-grid">
              {items.map((p) => (
                <ProjectCardWithCover key={p.id} project={p} onClick={() => setActive(p)} />
              ))}
            </div>
          </section>
        ))
      )}

      {active && <ProjectBackupModal project={active} onClose={() => setActive(null)} />}
    </div>
  );
}
