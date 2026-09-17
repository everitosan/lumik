import type { CSSProperties } from 'react';
import type { IconType } from 'react-icons';
import { SiGoogledrive, SiDropbox, SiMega, SiNextcloud } from 'react-icons/si';
import { FaCloud } from 'react-icons/fa6';

export interface ProviderMeta {
  label: string;
  /** Color de marca (para el icono). */
  color: string;
  /** URL del proveedor (raíz web). */
  url: string;
  Icon: IconType;
}

const META: Record<string, ProviderMeta> = {
  gdrive: { label: 'Google Drive', color: '#1FA463', url: 'https://drive.google.com', Icon: SiGoogledrive },
  dropbox: { label: 'Dropbox', color: '#0061FF', url: 'https://www.dropbox.com/home', Icon: SiDropbox },
  onedrive: { label: 'OneDrive', color: '#0364B8', url: 'https://onedrive.live.com', Icon: FaCloud },
  mega: { label: 'MEGA', color: '#D9272E', url: 'https://mega.nz/fm', Icon: SiMega },
  nextcloud: { label: 'Nextcloud', color: '#0082C9', url: 'https://nextcloud.com', Icon: SiNextcloud },
};

/** Metadatos (label/color/url/icon) de un proveedor, o `undefined` si es desconocido. */
export function providerMeta(provider: string): ProviderMeta | undefined {
  return META[provider];
}

export interface ProviderIconProps {
  provider: string;
  size?: number;
  /** Si se pasa, el icono es un botón que llama `onOpen(url, provider)`. */
  onOpen?: (url: string, provider: string) => void;
}

const buttonStyles: CSSProperties = {
  background: 'transparent',
  border: 'none',
  padding: 2,
  cursor: 'pointer',
  display: 'inline-flex',
  lineHeight: 0,
};

/** Icono de marca (a color) de un proveedor de nube. Opcionalmente clickeable. */
export function ProviderIcon({ provider, size = 16, onOpen }: ProviderIconProps) {
  const meta = providerMeta(provider);
  if (!meta) return null;
  const { Icon, color, label, url } = meta;
  const icon = <Icon size={size} color={color} title={label} />;

  if (!onOpen) {
    return (
      <span title={label} style={{ display: 'inline-flex', lineHeight: 0 }}>
        {icon}
      </span>
    );
  }
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      style={buttonStyles}
      onClick={(e) => {
        e.stopPropagation(); // no dispares el onClick de la card
        onOpen(url, provider);
      }}
    >
      {icon}
    </button>
  );
}
