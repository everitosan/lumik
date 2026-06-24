import { useState, useEffect, useCallback, type ChangeEvent } from 'react';
import { Input, Button, Icon } from '@lumik/ui';
import { useActivePhotographer } from '../lib/hooks';
import {
  getPhotographerMetadata,
  updatePhotographerMetadata,
  getAppSettings,
  updateAppSettings,
  getKeybindings,
  updateKeybinding,
} from '../lib/api';
import type { PhotographerMetadata, AppSettings, Keybinding } from '../lib/types';

const containerStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  height: '100%',
  padding: '24px 32px',
  gap: '32px',
  overflow: 'auto',
};

const headerStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  flexShrink: 0,
};

const titleStyles: React.CSSProperties = {
  fontSize: '28px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  margin: 0,
};

const sectionStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '20px',
  maxWidth: '560px',
};

const sectionHeaderStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
};

const sectionTitleStyles: React.CSSProperties = {
  fontSize: '18px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  margin: 0,
};

const sectionDescriptionStyles: React.CSSProperties = {
  fontSize: '14px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  margin: 0,
};

const formStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '16px',
};

const actionsStyles: React.CSSProperties = {
  display: 'flex',
  gap: '12px',
  marginTop: '8px',
};

const successStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '12px 16px',
  backgroundColor: 'var(--lumik-success-container, #1a3d1a)',
  color: 'var(--lumik-on-success-container, #a8f0a8)',
  borderRadius: '8px',
  fontSize: '14px',
};

const errorStyles: React.CSSProperties = {
  padding: '12px 16px',
  backgroundColor: 'var(--lumik-error-container, #93000a)',
  color: 'var(--lumik-on-error-container, #ffdad6)',
  borderRadius: '8px',
  fontSize: '14px',
};

const loadingStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  height: '200px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const checkboxContainerStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  padding: '12px 0',
};

const checkboxStyles: React.CSSProperties = {
  width: '20px',
  height: '20px',
  accentColor: 'var(--lumik-primary, #a8c7fa)',
  cursor: 'pointer',
};

const checkboxLabelStyles: React.CSSProperties = {
  fontSize: '14px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  cursor: 'pointer',
  userSelect: 'none',
};

const kbRowStyles: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '10px 0',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
};

const kbDescStyles: React.CSSProperties = {
  fontSize: '14px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const kbBadgeBaseStyles: React.CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '12px',
  fontWeight: 600,
  padding: '4px 10px',
  borderRadius: '6px',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  background: 'var(--lumik-surface-container, #201f1f)',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  cursor: 'pointer',
  minWidth: '80px',
  textAlign: 'center' as const,
  transition: 'border-color 0.15s, color 0.15s',
};

const MODIFIER_KEYS = new Set([
  'Shift', 'Control', 'Alt', 'Meta',
  'CapsLock', 'Tab', 'NumLock', 'ScrollLock',
]);

function formatKey(key: string): string {
  if (key.startsWith('Ctrl+')) return `Ctrl+${formatKey(key.slice(5))}`;
  const labels: Record<string, string> = {
    ArrowLeft: '←', ArrowRight: '→', ArrowUp: '↑', ArrowDown: '↓',
    Escape: 'Esc', Enter: '↵', ' ': 'Space', Backspace: '⌫',
  };
  return labels[key] ?? key;
}

function KeybindingRow({
  binding,
  onSaved,
}: {
  binding: Keybinding;
  onSaved: (action: string, key: string) => void;
}) {
  const [listening, setListening] = useState(false);
  const [rowState, setRowState] = useState<'idle' | 'saved' | 'error'>('idle');

  const startListening = () => {
    setListening(true);
    setRowState('idle');
  };

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!listening) return;
      e.preventDefault();
      e.stopPropagation();

      if (e.key === 'Escape') { setListening(false); return; }
      if (MODIFIER_KEYS.has(e.key)) return;

      setListening(false);
      updateKeybinding(binding.action, e.key)
        .then(() => {
          setRowState('saved');
          onSaved(binding.action, e.key);
          setTimeout(() => setRowState('idle'), 1500);
        })
        .catch(() => setRowState('error'));
    },
    [listening, binding.action, onSaved],
  );

  useEffect(() => {
    if (!listening) return;
    window.addEventListener('keydown', handleKeyDown, { capture: true });
    return () => window.removeEventListener('keydown', handleKeyDown, { capture: true });
  }, [listening, handleKeyDown]);

  const badgeStyle: React.CSSProperties = {
    ...kbBadgeBaseStyles,
    borderColor: listening
      ? 'var(--lumik-primary, #b0c6ff)'
      : rowState === 'saved'
      ? '#27AE60'
      : rowState === 'error'
      ? 'var(--lumik-error, #ffb4ab)'
      : 'var(--lumik-outline-variant, #424654)',
    color: listening
      ? 'var(--lumik-primary, #b0c6ff)'
      : rowState === 'saved'
      ? '#27AE60'
      : rowState === 'error'
      ? 'var(--lumik-error, #ffb4ab)'
      : 'var(--lumik-on-surface-variant, #c2c6d7)',
  };

  return (
    <div style={kbRowStyles}>
      <span style={kbDescStyles}>{binding.description}</span>
      <button onClick={startListening} style={badgeStyle}>
        {listening
          ? 'Presiona una tecla…'
          : rowState === 'error'
          ? '✕ Error'
          : formatKey(binding.key)}
      </button>
    </div>
  );
}

export function SettingsPage() {
  const { data: photographer, loading: photographerLoading } = useActivePhotographer();

  const [metadata, setMetadata] = useState<PhotographerMetadata | null>(null);
  const [appSettings, setAppSettings] = useState<AppSettings | null>(null);
  const [keybindings, setKeybindings] = useState<Keybinding[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  // Form fields
  const [embedMetadata, setEmbedMetadata] = useState(true);
  const [artist, setArtist] = useState('');
  const [copyright, setCopyright] = useState('');
  const [creatorUrl, setCreatorUrl] = useState('');

  // Load metadata and settings when photographer is available
  useEffect(() => {
    if (!photographer) return;

    const loadData = async () => {
      try {
        setLoading(true);

        const [metadataData, settingsData, keybindingsData] = await Promise.all([
          getPhotographerMetadata(photographer.id),
          getAppSettings(),
          getKeybindings(),
        ]);

        setMetadata(metadataData);
        setAppSettings(settingsData);
        setKeybindings(keybindingsData);

        // Initialize form fields
        if (metadataData) {
          setArtist(metadataData.artist ?? '');
          setCopyright(metadataData.copyright ?? '');
          setCreatorUrl(metadataData.contact_url ?? '');
        }

        setEmbedMetadata(settingsData.embed_metadata_on_import);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Error loading settings');
      } finally {
        setLoading(false);
      }
    };

    loadData();
  }, [photographer]);

  const handleSave = async () => {
    if (!photographer) return;

    setSaving(true);
    setError(null);
    setSuccess(false);

    try {
      const [updatedMetadata, updatedSettings] = await Promise.all([
        updatePhotographerMetadata(photographer.id, {
          artist: artist || undefined,
          copyright: copyright || undefined,
          contact_url: creatorUrl || undefined,
        }),
        updateAppSettings({
          embed_metadata_on_import: embedMetadata,
        }),
      ]);

      setMetadata(updatedMetadata);
      setAppSettings(updatedSettings);
      setSuccess(true);

      // Hide success message after 3 seconds
      setTimeout(() => setSuccess(false), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Error saving settings');
    } finally {
      setSaving(false);
    }
  };

  const hasChanges = () => {
    const metadataChanged = metadata
      ? artist !== (metadata.artist ?? '') ||
        copyright !== (metadata.copyright ?? '') ||
        creatorUrl !== (metadata.contact_url ?? '')
      : artist !== '' || copyright !== '' || creatorUrl !== '';

    const settingsChanged = appSettings
      ? embedMetadata !== appSettings.embed_metadata_on_import
      : false;

    return metadataChanged || settingsChanged;
  };

  if (photographerLoading || loading) {
    return (
      <div style={containerStyles}>
        <div style={loadingStyles}>Loading settings...</div>
      </div>
    );
  }

  return (
    <div style={containerStyles}>
      <div style={headerStyles}>
        <h1 style={titleStyles}>Settings</h1>
      </div>

      <section style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
        <div style={sectionHeaderStyles}>
          <h2 style={sectionTitleStyles}>Metadata</h2>
          <p style={sectionDescriptionStyles}>
            Configure the metadata to embed in your photos during import.
          </p>
        </div>

        {error && <div style={errorStyles}>{error}</div>}
        {success && (
          <div style={successStyles}>
            <Icon name="check" size="sm" />
            Settings saved successfully
          </div>
        )}

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '16px' }}>
          <label style={{ ...checkboxContainerStyles, gridColumn: '1 / -1' }}>
            <input
              type="checkbox"
              checked={embedMetadata}
              onChange={(e) => setEmbedMetadata(e.target.checked)}
              style={checkboxStyles}
              disabled={saving}
            />
            <span style={checkboxLabelStyles}>
              Embed metadata in photos on import
            </span>
          </label>

          <Input
            label="Artist"
            placeholder="Your name or studio name"
            value={artist}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setArtist(e.target.value)}
            fullWidth
            disabled={saving || !embedMetadata}
          />

          <Input
            label="Copyright"
            placeholder="e.g. © 2024 Your Name. All rights reserved."
            value={copyright}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setCopyright(e.target.value)}
            fullWidth
            disabled={saving || !embedMetadata}
          />

          <div style={{ gridColumn: '1 / -1' }}>
            <Input
              label="Creator URL"
              placeholder="https://yourwebsite.com"
              value={creatorUrl}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setCreatorUrl(e.target.value)}
              fullWidth
              disabled={saving || !embedMetadata}
            />
          </div>

          <div style={{ gridColumn: '1 / -1' }}>
            <Input
              label="Image description format"
              value="{Project description}@{Year}"
              placeholder="{Project description}@{Year}"
              fullWidth
              disabled
            />
          </div>

          <div style={{ gridColumn: '1 / -1', ...actionsStyles, justifyContent: 'flex-end' }}>
            <Button
              variant="primary"
              onClick={handleSave}
              disabled={saving || !hasChanges()}
            >
              {saving ? 'Saving...' : 'Save changes'}
            </Button>
          </div>
        </div>
      </section>

      {keybindings.length > 0 && (() => {
        const CONTEXT_LABELS: Record<string, string> = {
          photo_detail: 'Photo Detail',
          project: 'Project',
          projects: 'Projects',
        };

        const groups = keybindings.reduce<Record<string, typeof keybindings>>((acc, kb) => {
          const ctx = kb.action.split('.')[0];
          (acc[ctx] ??= []).push(kb);
          return acc;
        }, {});

        const onSaved = (action: string, key: string) =>
          setKeybindings((prev) => prev.map((b) => (b.action === action ? { ...b, key } : b)));

        return (
          <section style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <div style={sectionHeaderStyles}>
              <h2 style={sectionTitleStyles}>Keyboard shortcuts</h2>
              <p style={sectionDescriptionStyles}>
                Click on a key badge and press the new key to reassign it. Press Esc to cancel.
              </p>
            </div>

            <div style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(3, 1fr)',
              gap: '0 48px',
              alignItems: 'start',
            }}>
              {(['projects', 'project', 'photo_detail'] as const)
                .filter((ctx) => groups[ctx])
                .map((ctx) => { const rows = groups[ctx]; return (
                <div key={ctx} style={{ display: 'flex', flexDirection: 'column' }}>
                  <span style={{
                    fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
                    fontSize: '10px',
                    fontWeight: 600,
                    letterSpacing: '0.1em',
                    textTransform: 'uppercase' as const,
                    color: 'var(--lumik-outline, #8c90a0)',
                    padding: '16px 0 4px',
                  }}>
                    {CONTEXT_LABELS[ctx] ?? ctx}
                  </span>
                  {rows.map((kb) => (
                    <KeybindingRow key={kb.action} binding={kb} onSaved={onSaved} />
                  ))}
                </div>
              ); })}
            </div>
          </section>
        );
      })()}
    </div>
  );
}
