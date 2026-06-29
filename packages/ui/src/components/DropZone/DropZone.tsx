import { useState, useRef, type CSSProperties, type DragEvent, type ChangeEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '../Icon';

export interface DropZoneFile {
  /** File name */
  name: string;
  /** File size in bytes */
  sizeBytes: number;
  /** Original File object */
  file: File;
}

export interface DropZoneProps {
  /** Callback when files are dropped or selected */
  onFilesAdded?: (files: DropZoneFile[]) => void;
  /** Accepted file extensions (e.g., ['.raw', '.cr2', '.dng']) */
  acceptedExtensions?: string[];
  /** Whether multiple files can be selected */
  multiple?: boolean;
  /** Custom title text */
  title?: string;
  /** Custom hint text showing accepted formats */
  hint?: string;
  /** Whether the drop zone is disabled */
  disabled?: boolean;
  /** Minimum height of the drop zone */
  minHeight?: number | string;
  /** Custom browse handler (e.g., for Tauri dialog). If provided, native file input is not used */
  onBrowse?: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

const containerStyles = (isDragging: boolean, disabled: boolean): CSSProperties => ({
  position: 'relative',
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  gap: '16px',
  padding: '48px 32px',
  border: `2px dashed ${
    isDragging
      ? 'var(--lumik-primary, #b0c6ff)'
      : 'var(--lumik-outline-variant, #424654)'
  }`,
  borderRadius: 'var(--lumik-radius-md, 8px)',
  backgroundColor: isDragging
    ? 'rgba(176, 198, 255, 0.05)'
    : 'transparent',
  backgroundImage: `radial-gradient(circle, var(--lumik-outline-variant, #424654) 1px, transparent 1px)`,
  backgroundSize: '24px 24px',
  transition: 'all 150ms ease',
  cursor: disabled ? 'not-allowed' : 'pointer',
  opacity: disabled ? 0.5 : 1,
  minHeight: '200px',
});

const iconContainerStyles = (isDragging: boolean): CSSProperties => ({
  width: '72px',
  height: '72px',
  borderRadius: '50%',
  backgroundColor: isDragging
    ? 'rgba(176, 198, 255, 0.15)'
    : 'var(--lumik-surface-container-high, #2a2a2a)',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  transition: 'all 150ms ease',
});

const titleStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '16px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  textAlign: 'center',
  margin: 0,
};

const hintStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  fontWeight: 500,
  color: 'var(--lumik-outline, #8c90a0)',
  textAlign: 'center',
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
};

const hiddenInputStyles: CSSProperties = {
  position: 'absolute',
  top: 0,
  left: 0,
  width: '100%',
  height: '100%',
  opacity: 0,
  cursor: 'pointer',
};

// Allowed file extensions: RAW formats from popular camera brands + JPEG
// Canon: CR2, CR3, CRW | Fuji: RAF | Nikon: NEF, NRW
// Sony: ARW, SRF, SR2 | Panasonic/Lumix: RW2 | Leica: RWL, DNG
// Olympus: ORF | Pentax: PEF | Hasselblad: 3FR, FFF | Phase One: IIQ
export const ALLOWED_RAW_EXTENSIONS = [
  // Canon
  '.cr2', '.cr3', '.crw',
  // Nikon
  '.nef', '.nrw',
  // Sony
  '.arw', '.srf', '.sr2',
  // Fuji
  '.raf',
  // Panasonic/Lumix
  '.rw2',
  // Olympus
  '.orf',
  // Pentax
  '.pef',
  // Leica
  '.rwl',
  // Hasselblad
  '.3fr', '.fff',
  // Phase One
  '.iiq',
  // Adobe DNG (universal)
  '.dng',
  // Generic RAW
  '.raw',
  // JPEG / TIFF (copied as-is, no conversion)
  '.jpg', '.jpeg', '.tif', '.tiff',
  // Video (copied as-is to _video/)
  '.mp4', '.mov', '.avi', '.mts', '.m2ts', '.mkv', '.mxf',
];

export const VIDEO_EXTENSIONS = ['.mp4', '.mov', '.avi', '.mts', '.m2ts', '.mkv', '.mxf'];

export function isVideoFile(filename: string): boolean {
  const lower = filename.toLowerCase();
  return VIDEO_EXTENSIONS.some((ext) => lower.endsWith(ext));
}

/**
 * Validates if a filename has an allowed RAW extension
 * @param filename - The filename to validate
 * @param extensions - Optional custom extensions array (defaults to ALLOWED_RAW_EXTENSIONS)
 */
export function isAllowedRawFile(filename: string, extensions: string[] = ALLOWED_RAW_EXTENSIONS): boolean {
  const lowerName = filename.toLowerCase();
  return extensions.some((ext) => lowerName.endsWith(ext.toLowerCase()));
}

const DEFAULT_HINT = 'Supports CR2, CR3, NEF, ARW, RAF, ORF, RW2, DNG, JPEG, TIFF, MP4, MOV and more';

export function DropZone({
  onFilesAdded,
  acceptedExtensions = ALLOWED_RAW_EXTENSIONS,
  multiple = true,
  title = 'Drag files here or click to browse',
  hint = DEFAULT_HINT,
  disabled = false,
  minHeight,
  onBrowse,
}: DropZoneProps) {
  const [isDragging, setIsDragging] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const dragCounter = useRef(0);

  const isValidFile = (file: File): boolean => {
    const fileName = file.name.toLowerCase();
    return acceptedExtensions.some((ext) => fileName.endsWith(ext.toLowerCase()));
  };

  const processFiles = (files: FileList | File[]) => {
    const validFiles: DropZoneFile[] = Array.from(files)
      .filter(isValidFile)
      .map((file) => ({
        name: file.name,
        sizeBytes: file.size,
        file,
      }));

    if (validFiles.length > 0) {
      onFilesAdded?.(validFiles);
    }
  };

  const handleDragEnter = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (disabled) return;

    dragCounter.current += 1;
    if (e.dataTransfer.items && e.dataTransfer.items.length > 0) {
      setIsDragging(true);
    }
  };

  const handleDragLeave = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (disabled) return;

    dragCounter.current -= 1;
    if (dragCounter.current === 0) {
      setIsDragging(false);
    }
  };

  const handleDragOver = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDrop = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (disabled) return;

    setIsDragging(false);
    dragCounter.current = 0;

    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      processFiles(e.dataTransfer.files);
    }
  };

  const handleInputChange = (e: ChangeEvent<HTMLInputElement>) => {
    if (disabled) return;

    if (e.target.files && e.target.files.length > 0) {
      processFiles(e.target.files);
      // Reset input so the same file can be selected again
      e.target.value = '';
    }
  };

  const handleClick = () => {
    if (disabled) return;
    if (onBrowse) {
      onBrowse();
    } else {
      inputRef.current?.click();
    }
  };

  const acceptString = acceptedExtensions.join(',');

  return (
    <div
      style={{ ...containerStyles(isDragging, disabled), ...(minHeight && { minHeight }) }}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
      onClick={handleClick}
      role="button"
      tabIndex={disabled ? -1 : 0}
      onKeyDown={(e) => {
        if (!disabled && (e.key === 'Enter' || e.key === ' ')) {
          e.preventDefault();
          handleClick();
        }
      }}
    >
      {/* Only render native input when onBrowse is not provided */}
      {!onBrowse && (
        <input
          ref={inputRef}
          type="file"
          accept={acceptString}
          multiple={multiple}
          onChange={handleInputChange}
          style={hiddenInputStyles}
          tabIndex={-1}
          disabled={disabled}
        />
      )}

      <div style={iconContainerStyles(isDragging)}>
        <Icon
          name="sd-card"
          size={32}
          color={
            isDragging
              ? 'var(--lumik-primary, #b0c6ff)'
              : 'var(--lumik-on-surface-variant, #c2c6d7)'
          }
        />
      </div>

      <p style={titleStyles}>{title}</p>

      <span style={hintStyles}>{hint}</span>
    </div>
  );
}

// Helper component for displaying selected files summary
export interface DropZoneSummaryProps {
  /** Number of files selected */
  fileCount: number;
  /** Total size in bytes */
  totalBytes: number;
}

const summaryStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: '24px',
  padding: '12px 16px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
};

const summaryItemStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
};

const summaryLabelStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  fontWeight: 500,
  color: 'var(--lumik-outline, #8c90a0)',
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
};

const summaryValueStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '14px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

export function DropZoneSummary({
  fileCount,
  totalBytes
}: DropZoneSummaryProps) {
  const { t } = useTranslation();

  return (
    <div style={summaryStyles}>
      <div style={summaryItemStyles}>
        <span style={summaryValueStyles}>{fileCount}</span>
        <span style={summaryLabelStyles}>{t('components.dropzone.filesSelected')}</span>
      </div>
      <div style={summaryItemStyles}>
        <span style={summaryValueStyles}>{formatBytes(totalBytes)}</span>
        <span style={summaryLabelStyles}>{t('components.dropzone.total')}</span>
      </div>
    </div>
  );
}
