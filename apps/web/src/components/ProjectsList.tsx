import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { catalogApi, type CatalogProject } from '../api/client';

export function ProjectsList() {
  const { t } = useTranslation();
  const [projects, setProjects] = useState<CatalogProject[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    catalogApi
      .listProjects()
      .then(setProjects)
      .catch(() => setProjects([]))
      .finally(() => setLoading(false));
  }, []);

  return (
    <section className="projects-panel">
      <h2>{t('projects.title')}</h2>
      {loading ? (
        <p className="subtitle">{t('common.loading')}</p>
      ) : projects.length === 0 ? (
        <p className="subtitle">{t('projects.empty')}</p>
      ) : (
        <ul className="projects-list">
          {projects.map((p) => (
            <li key={p.id} className="project-row">
              <span className="project-name">{p.name}</span>
              <span className="subtitle">{p.session_date ?? ''}</span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
