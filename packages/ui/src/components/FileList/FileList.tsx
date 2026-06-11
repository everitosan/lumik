import { useState, useMemo, type CSSProperties } from 'react';
import { Button } from '../Button';
import { Icon } from '../Icon';

export interface FileListItem {
  name: string;
  sizeBytes: number;
}

export interface FileListProps {
  title?: string;
  files: FileListItem[];
  onRemove?: (index: number) => void;
  maxHeight?: number | string;
  showSearch?: boolean;
  searchPlaceholder?: string;
  className?: string;
  style?: CSSProperties;
}

const containerStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '16px',
};

const headerStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
};

const titleStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
};

const searchContainerStyles: CSSProperties = {
  display: 'flex',
  justifyContent: 'center',
};

const searchInputStyles: CSSProperties = {
  width: '100%',
  maxWidth: '400px',
  padding: '8px 12px 8px 36px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  outline: 'none',
  transition: 'border-color 200ms ease',
};

const searchWrapperStyles: CSSProperties = {
  position: 'relative',
  width: '100%',
  maxWidth: '400px',
};

const searchIconStyles: CSSProperties = {
  position: 'absolute',
  left: '10px',
  top: '50%',
  transform: 'translateY(-50%)',
  color: 'var(--lumik-outline, #8c90a0)',
  pointerEvents: 'none',
};

const gridStyles = (maxHeight?: number | string): CSSProperties => ({
  display: 'grid',
  gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
  gap: '8px',
  maxHeight: maxHeight ?? '400px',
  overflow: 'auto',
  paddingRight: '4px',
});

const itemStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '10px 12px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  minWidth: 0,
};

const fileNameStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
  flex: 1,
  minWidth: 0,
};

const fileInfoStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  flexShrink: 0,
  marginLeft: '8px',
};

const fileSizeStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '11px',
  color: 'var(--lumik-outline, #8c90a0)',
};

const noResultsStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  padding: '24px',
  color: 'var(--lumik-outline, #8c90a0)',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
};

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export function FileList({
  title,
  files,
  onRemove,
  maxHeight,
  showSearch = true,
  searchPlaceholder = 'Buscar archivos...',
  className,
  style,
}: FileListProps) {
  const [searchQuery, setSearchQuery] = useState('');

  const filteredFiles = useMemo(() => {
    if (!searchQuery.trim()) return files;
    const query = searchQuery.toLowerCase();
    return files.filter((file) => file.name.toLowerCase().includes(query));
  }, [files, searchQuery]);

  // Get original indices for filtered files (needed for onRemove)
  const filteredIndices = useMemo(() => {
    if (!searchQuery.trim()) return files.map((_, i) => i);
    const query = searchQuery.toLowerCase();
    return files
      .map((file, index) => ({ file, index }))
      .filter(({ file }) => file.name.toLowerCase().includes(query))
      .map(({ index }) => index);
  }, [files, searchQuery]);

  if (files.length === 0) {
    return null;
  }

  return (
    <div style={{ ...containerStyles, ...style }} className={className}>
      <div style={headerStyles}>
        {title && (
          <span style={titleStyles}>
            {title} ({files.length})
          </span>
        )}
        {showSearch && files.length > 5 && (
          <div style={searchContainerStyles}>
            <div style={searchWrapperStyles}>
              <Icon name="search" size="sm" style={searchIconStyles} />
              <input
                type="text"
                placeholder={searchPlaceholder}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                style={searchInputStyles}
              />
            </div>
          </div>
        )}
      </div>

      {filteredFiles.length === 0 ? (
        <div style={noResultsStyles}>
          No se encontraron archivos que coincidan con "{searchQuery}"
        </div>
      ) : (
        <div style={gridStyles(maxHeight)}>
          {filteredFiles.map((file, idx) => {
            const originalIndex = filteredIndices[idx];
            return (
              <div key={originalIndex} style={itemStyles}>
                <span style={fileNameStyles} title={file.name}>
                  {file.name}
                </span>
                <div style={fileInfoStyles}>
                  <span style={fileSizeStyles}>{formatBytes(file.sizeBytes)}</span>
                  {onRemove && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => onRemove(originalIndex)}
                    >
                      <Icon name="x" size="sm" />
                    </Button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
