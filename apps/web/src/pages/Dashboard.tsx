import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ProjectCard, type WorkflowStatus } from '@lumik/ui';
import { catalogApi, type CatalogProject } from '../api/client';
import { ProjectBackupModal } from '../components/ProjectBackupModal';

function statusOf(p: CatalogProject): WorkflowStatus {
  const total = p.photo_count ?? 0;
  const up = p.backed_up ?? 0;
  if (total > 0 && up >= total) return 'delivered';
  if (up > 0) return 'edited';
  return 'imported';
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

  return (
    <div className="page">
      <h1>{t('projects.title')}</h1>
      {loading ? (
        <p className="subtitle">{t('common.loading')}</p>
      ) : projects.length === 0 ? (
        <p className="subtitle">{t('projects.empty')}</p>
      ) : (
        <div className="projects-grid">
          {projects.map((p) => (
            <ProjectCard
              key={p.id}
              name={p.name}
              photoCount={p.photo_count ?? 0}
              date={p.session_date ?? ''}
              driveName={t('projects.card.backedUp', {
                up: p.backed_up ?? 0,
                total: p.photo_count ?? 0,
              })}
              status={statusOf(p)}
              onClick={() => setActive(p)}
            />
          ))}
        </div>
      )}

      {active && <ProjectBackupModal project={active} onClose={() => setActive(null)} />}
    </div>
  );
}
