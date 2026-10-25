import type { ComponentType, CSSProperties, SVGProps } from 'react';
import {
  Aperture,
  Archive,
  ArchiveRestore,
  ArrowUpDown,
  Calendar,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CloudOff,
  CloudUpload,
  Database,
  Eject,
  EllipsisVertical,
  Folder,
  FolderOpen,
  Image,
  Import,
  Info,
  LayoutGrid,
  Lock,
  Plus,
  RotateCcw,
  RotateCw,
  Search,
  Settings,
  SquarePen,
  Star,
  Tag,
  Trash2,
  X,
  ZoomIn,
  ZoomOut,
} from 'lucide-react';

export type IconName =
  | 'search'
  | 'projects'
  | 'import'
  | 'settings'
  | 'drive'
  | 'image'
  | 'photo'
  | 'menu'
  | 'calendar'
  | 'aperture'
  | 'check'
  | 'x'
  | 'chevron-down'
  | 'chevron-right'
  | 'chevron-left'
  | 'plus'
  | 'star'
  | 'star-filled'
  | 'trash'
  | 'edit'
  | 'folder'
  | 'folder-open'
  | 'sd-card'
  | 'upload-cloud'
  | 'cloud-off'
  | 'zoom-in'
  | 'zoom-out'
  | 'rotate-ccw'
  | 'rotate-cw'
  | 'info'
  | 'tags'
  | 'eject'
  | 'sort'
  | 'archive'
  | 'lock'
  | 'archive-restore';


export interface IconProps extends Omit<SVGProps<SVGSVGElement>, 'name'> {
  name: IconName;
  size?: number | 'sm' | 'md' | 'lg' | 'xl';
  color?: string;
}

const sizeMap = {
  sm: 12,
  md: 16,
  lg: 20,
  xl: 24,
};

type IconComponent = ComponentType<SVGProps<SVGSVGElement>>;

// Lucide no tiene tarjeta SD: se dibuja a mano con los mismos atributos de trazo.
const SdCard: IconComponent = (props) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" {...props}>
    <path d="M18 2h-8L4 8v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2z" />
    <path d="M10 2v4" />
    <path d="M14 2v4" />
  </svg>
);

const StarFilled: IconComponent = (props) => <Star fill="currentColor" {...props} />;

// Los nombres de IconName se conservan aunque no coincidan con los de Lucide
// (p. ej. `drive` siempre fue el cilindro de base de datos).
const icons: Record<IconName, IconComponent> = {
  info: Info,
  search: Search,
  projects: LayoutGrid,
  import: Import,
  settings: Settings,
  drive: Database,
  image: Image,
  photo: Image,
  menu: EllipsisVertical,
  calendar: Calendar,
  aperture: Aperture,
  check: Check,
  x: X,
  'chevron-down': ChevronDown,
  'chevron-right': ChevronRight,
  'chevron-left': ChevronLeft,
  plus: Plus,
  star: Star,
  'star-filled': StarFilled,
  trash: Trash2,
  edit: SquarePen,
  folder: Folder,
  'folder-open': FolderOpen,
  'sd-card': SdCard,
  'upload-cloud': CloudUpload,
  'cloud-off': CloudOff,
  'zoom-in': ZoomIn,
  'zoom-out': ZoomOut,
  'rotate-ccw': RotateCcw,
  'rotate-cw': RotateCw,
  tags: Tag,
  eject: Eject,
  sort: ArrowUpDown,
  archive: Archive,
  lock: Lock,
  'archive-restore': ArchiveRestore,
};

export const iconNames = Object.keys(icons) as IconName[];

export function Icon({ name, size = 'md', color, style, className, ...props }: IconProps) {
  const resolvedSize = typeof size === 'number' ? size : sizeMap[size];
  const IconComponent = icons[name];

  const combinedStyle: CSSProperties = {
    width: resolvedSize,
    height: resolvedSize,
    color: color ?? 'currentColor',
    flexShrink: 0,
    ...style,
  };

  return <IconComponent style={combinedStyle} className={className} {...props} />;
}
