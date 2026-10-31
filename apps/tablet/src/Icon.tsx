// Íconos de la tablet: Lucide, como el `Icon` de `@lumik/ui` en el desktop, con
// los mismos nombres. Ese componente dibuja SVG del DOM y no sirve en React
// Native; aquí se usa `lucide-react-native` (sobre `react-native-svg`).
// Al agregar un nombre en uno, agregarlo también en el otro.
import type { ComponentType } from 'react';
import type { StyleProp, ViewStyle } from 'react-native';
import Svg, { Path } from 'react-native-svg';
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
  CircleHelp,
  CloudOff,
  CloudUpload,
  Database,
  Eject,
  EllipsisVertical,
  Folder,
  FolderOpen,
  Image,
  Images,
  Import,
  Info,
  LayoutGrid,
  Lock,
  type LucideProps,
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
} from 'lucide-react-native';

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
  | 'help'
  | 'tags'
  | 'eject'
  | 'sort'
  | 'archive'
  | 'lock'
  | 'album'
  | 'archive-restore';

const sizeMap = { sm: 12, md: 16, lg: 20, xl: 24 };

type IconComponent = ComponentType<LucideProps>;

// Lucide no tiene tarjeta SD: se dibuja a mano con los mismos atributos de trazo.
const SdCard: IconComponent = ({ size = 24, color = 'currentColor', strokeWidth = 2, style }) => (
  <Svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth={strokeWidth} strokeLinecap="round" strokeLinejoin="round" style={style}>
    <Path d="M18 2h-8L4 8v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2z" />
    <Path d="M10 2v4" />
    <Path d="M14 2v4" />
  </Svg>
);

const StarFilled: IconComponent = (props) => <Star fill={props.color} {...props} />;

const icons: Record<IconName, IconComponent> = {
  info: Info,
  help: CircleHelp,
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
  album: Images,
  'archive-restore': ArchiveRestore,
};

export type IconProps = {
  name: IconName;
  size?: number | keyof typeof sizeMap;
  color?: string;
  strokeWidth?: number;
  style?: StyleProp<ViewStyle>;
};

export function Icon({ name, size = 'md', color = '#fff', strokeWidth, style }: IconProps) {
  const IconComponent = icons[name];
  return <IconComponent size={typeof size === 'number' ? size : sizeMap[size]} color={color} strokeWidth={strokeWidth} style={style} />;
}
