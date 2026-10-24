import { useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { CardMenu, type CardMenuItem } from '../CardMenu';
import { Icon, type IconName } from '../Icon';
import { ProviderIcon } from '../ProviderIcon';

export type WorkflowStatus = 'imported' | 'edited' | 'delivered';

export interface ProjectCardProps {
  name: string;
  photoCount: number;
  date: string;
  driveName: string;
  status: WorkflowStatus;
  thumbnailUrl?: string;
  /** Proveedores de nube donde se respaldó (muestra sus iconos de marca). */
  providers?: string[];
  /** Si se pasa, los iconos de proveedor son clickeables (abrir su web). */
  onProviderOpen?: (url: string, provider: string) => void;
  onClick?: () => void;
  /** Each handler, when provided, adds its item to the 3-dot menu. Omit one to hide
   *  that option (e.g. leave out onOpenFolder on Android). If none are provided the
   *  menu button is hidden entirely. */
  onOpenFolder?: () => void;
  onEdit?: () => void;
  /** Archivar / desarchivar (el texto del menú depende de `archived`). */
  onToggleArchive?: () => void;
  /** Borrar el respaldo del proyecto en la nube (solo si está respaldado). */
  onDeleteBackup?: () => void;
  onDelete?: () => void;
  /** Proyecto archivado: se muestra atenuado y con la etiqueta "Archivado". */
  archived?: boolean;
  className?: string;
  style?: CSSProperties;
}

const cardStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  backgroundColor: 'var(--lumik-glass-bg, rgba(32, 31, 31, 0.6))',
  backdropFilter: 'blur(var(--lumik-glass-blur, 12px))',
  border: '1px solid var(--lumik-glass-border, rgba(66, 70, 84, 0.3))',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  overflow: 'hidden',
  cursor: 'pointer',
  transition: 'transform var(--lumik-transition-normal, 250ms ease), box-shadow var(--lumik-transition-normal, 250ms ease)',
};

const thumbnailContainerStyles: CSSProperties = {
  position: 'relative',
  aspectRatio: '4/3',
  backgroundColor: 'var(--lumik-surface-container-lowest, #0e0e0e)',
  overflow: 'hidden',
};

const thumbnailStyles: CSSProperties = {
  width: '100%',
  height: '100%',
  objectFit: 'cover',
};

const placeholderStyles: CSSProperties = {
  width: '100%',
  height: '100%',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  color: 'var(--lumik-outline, #8c90a0)',
};

const contentStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  padding: '12px',
};

const headerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'flex-start',
  justifyContent: 'space-between',
  gap: '8px',
};

const titleStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '16px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  margin: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};

const metaContainerStyles: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: '1fr 1fr',
  gap: '4px 8px',
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const metaItemStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '4px',
  overflow: 'hidden',
};

const metaItemFullStyles: CSSProperties = {
  ...metaItemStyles,
  gridColumn: '1 / -1',
};

const archivedLabelStyles: CSSProperties = {
  position: 'absolute',
  top: '8px',
  left: '8px',
  display: 'inline-flex',
  alignItems: 'center',
  gap: '4px',
  padding: '2px 8px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '11px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  backgroundColor: 'rgba(14, 14, 14, 0.75)',
  borderRadius: 'var(--lumik-radius-full, 9999px)',
};

const statusBadgeBaseStyles: CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  gap: '4px',
  padding: '2px 8px',
  fontSize: '11px',
  fontWeight: 500,
  borderRadius: 'var(--lumik-radius-full, 9999px)',
};

// Semáforo de trabajo: amarillo = pendiente, azul = en curso, gris = terminado.
const statusBadgeConfig: Record<WorkflowStatus, { color: string; backgroundColor: string; icon: IconName }> = {
  imported: {
    color: 'var(--lumik-secondary, #e9c349)',
    backgroundColor: 'rgba(233, 195, 73, 0.15)',
    icon: 'import',
  },
  edited: {
    color: 'var(--lumik-primary, #b0c6ff)',
    backgroundColor: 'rgba(176, 198, 255, 0.15)',
    icon: 'edit',
  },
  delivered: {
    color: 'var(--lumik-outline, #8c90a0)',
    backgroundColor: 'rgba(140, 144, 160, 0.15)',
    icon: 'check',
  },
};

function formatDate(dateString: string): string {
  // Date-only strings (YYYY-MM-DD) are parsed as UTC midnight by spec, which
  // shifts the date back one day in negative-offset timezones. Appending
  // T00:00:00 forces local-time interpretation.
  const normalized = /^\d{4}-\d{2}-\d{2}$/.test(dateString)
    ? dateString + 'T00:00:00'
    : dateString;
  const date = new Date(normalized);
  if (isNaN(date.getTime())) return dateString;
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}/${month}/${day}`;
}

export function ProjectCard({
  name,
  photoCount,
  date,
  driveName,
  status,
  thumbnailUrl,
  providers,
  onProviderOpen,
  onClick,
  onOpenFolder,
  onEdit,
  onToggleArchive,
  onDeleteBackup,
  onDelete,
  archived = false,
  className,
  style,
}: ProjectCardProps) {
  const { t } = useTranslation();
  const [hovered, setHovered] = useState(false);

  const menuItems: CardMenuItem[] = [];
  if (onOpenFolder) {
    menuItems.push({ icon: 'folder-open', label: t('components.projectCard.menu.openFolder'), onSelect: onOpenFolder });
  }
  if (onEdit) {
    menuItems.push({ icon: 'edit', label: t('components.projectCard.menu.edit'), onSelect: onEdit });
  }
  if (onToggleArchive) {
    menuItems.push({
      icon: archived ? 'archive-restore' : 'archive',
      label: archived ? t('components.projectCard.menu.unarchive') : t('components.projectCard.menu.archive'),
      onSelect: onToggleArchive,
    });
  }
  if (onDeleteBackup) {
    menuItems.push({
      icon: 'cloud-off',
      label: t('components.projectCard.menu.deleteBackup'),
      onSelect: onDeleteBackup,
      danger: true,
    });
  }
  if (onDelete) {
    menuItems.push({ icon: 'trash', label: t('components.projectCard.menu.delete'), onSelect: onDelete, danger: true });
  }

  const statusLabels = {
    imported: t('components.projectCard.status.imported'),
    edited: t('components.projectCard.status.edited'),
    delivered: t('components.projectCard.status.delivered'),
  };

  return (
    <div
      style={{
        ...cardStyles,
        // Archivado: atenuado, pero legible al pasar el ratón.
        ...(archived ? { opacity: hovered ? 0.85 : 0.55 } : null),
        ...style,
      }}
      className={className}
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClick?.(); } }}
      role="button"
      tabIndex={0}
    >
      <div style={thumbnailContainerStyles}>
        {thumbnailUrl ? (
          <img src={thumbnailUrl} alt={name} style={thumbnailStyles} />
        ) : (
          <div style={placeholderStyles}>
            <Icon name="image" size={48} />
          </div>
        )}
        {archived && (
          <span style={archivedLabelStyles}>
            <Icon name="archive" size="sm" aria-hidden="true" />
            {t('components.projectCard.archived')}
          </span>
        )}
      </div>

      <div style={contentStyles}>
        <div style={headerStyles}>
          <h3 style={titleStyles} title={name}>
            {name}
          </h3>
          <CardMenu label={t('components.projectCard.menu.label')} items={menuItems} />
        </div>

        <div style={metaContainerStyles}>
          <span style={{ ...metaItemFullStyles, overflow: 'hidden' }}>
            <Icon name="drive" size="sm" />
            <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {driveName}
            </span>
          </span>
          <span style={metaItemStyles}>
            <Icon name="calendar" size="sm" />
            {formatDate(date)}
          </span>
          <span style={{ ...metaItemStyles, justifyContent: 'flex-end' }}>
            <Icon name="photo" size="sm" />
            {photoCount} {t('dashboard.statistics.photos')}
          </span>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px' }}>
          <span
            style={{
              ...statusBadgeBaseStyles,
              color: statusBadgeConfig[status].color,
              backgroundColor: statusBadgeConfig[status].backgroundColor,
            }}
          >
            <Icon name={statusBadgeConfig[status].icon} size="sm" aria-hidden="true" />
            {statusLabels[status]}
          </span>
          {providers && providers.length > 0 && (
            <span style={{ display: 'inline-flex', alignItems: 'center', gap: '6px' }}>
              {providers.map((p) => (
                <ProviderIcon key={p} provider={p} size={16} onOpen={onProviderOpen} />
              ))}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
