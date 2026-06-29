import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getVersion } from '@tauri-apps/api/app';
import { Logo, Icon } from '@lumik/ui';

const pageStyles: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'center',
  alignItems: 'flex-start',
  height: '100%',
  overflowY: 'auto',
  padding: '64px 32px',
};

const contentStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '48px',
  width: '100%',
  maxWidth: '560px',
};

const heroStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  gap: '16px',
};

const versionStyles: React.CSSProperties = {
  fontSize: '14px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  opacity: 0.6,
};

const descriptionStyles: React.CSSProperties = {
  fontSize: '15px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  lineHeight: 1.6,
  textAlign: 'center',
};

const sectionStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
};

const sectionTitleStyles: React.CSSProperties = {
  fontSize: '11px',
  fontWeight: 600,
  textTransform: 'uppercase',
  letterSpacing: '0.08em',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  opacity: 0.6,
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
  paddingBottom: '12px',
};

const authorNameStyles: React.CSSProperties = {
  fontSize: '16px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  textAlign: 'center',
};

const authorDetailStyles: React.CSSProperties = {
  fontSize: '14px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  gap: '4px',
};

const linkStyles: React.CSSProperties = {
  color: 'var(--lumik-primary, #b0c6ff)',
  textDecoration: 'none',
  fontSize: '14px',
};

const accordionHeaderStyles: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  cursor: 'pointer',
  userSelect: 'none',
  fontSize: '11px',
  fontWeight: 600,
  textTransform: 'uppercase',
  letterSpacing: '0.08em',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  opacity: 0.6,
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
  paddingBottom: '12px',
};

const licenseRowStyles: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  padding: '6px 0',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
};

const licenseBadgeStyles: React.CSSProperties = {
  fontSize: '12px',
  opacity: 0.6,
  fontFamily: 'var(--lumik-font-mono, monospace)',
};

const licenses = [
  { name: 'rawler',    license: 'MIT / Apache-2.0', url: 'https://github.com/dnglab/rawler' },
  { name: 'dnglab',   license: 'LGPL-2.1',          url: 'https://github.com/dnglab/dnglab' },
  { name: 'exiftool', license: 'Perl Artistic / GPL', url: 'https://exiftool.org' },
  { name: 'Tauri',    license: 'MIT / Apache-2.0',  url: 'https://tauri.app' },
  { name: 'React',    license: 'MIT',               url: 'https://react.dev' },
  { name: 'rusqlite', license: 'MIT',               url: 'https://github.com/rusqlite/rusqlite' },
];

export function AboutPage() {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string>('');
  const [licensesOpen, setLicensesOpen] = useState(false);

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);

  return (
    <div style={pageStyles}>
      <div style={contentStyles}>

        <div style={heroStyles}>
          <Logo size="lg" />
          {version && <div style={versionStyles}>{t('about.version')} {version}</div>}
          <p style={descriptionStyles}>
            {t('about.description')}
          </p>
        </div>

        <div style={sectionStyles}>
          <div style={sectionTitleStyles}>{t('about.credits')}</div>
          <div style={authorNameStyles}>Everardo Sánchez Hernández</div>
          <div style={authorDetailStyles}>
            <span>{t('about.copyright')}</span>
            <a href="https://evesan.rocks" target="_blank" rel="noreferrer" style={linkStyles}>
              evesan.rocks
            </a>
            <a href="mailto:eve.san.dev@gmail.com" style={linkStyles}>
              eve.san.dev@gmail.com
            </a>
          </div>
        </div>

        <div style={sectionStyles}>
          <div style={accordionHeaderStyles} onClick={() => setLicensesOpen(o => !o)}>
            <span>{t('about.openSourceLicenses')}</span>
            <Icon name={licensesOpen ? 'chevron-down' : 'chevron-right'} size="sm" />
          </div>
          {licensesOpen && licenses.map(({ name, license, url }) => (
            <div key={name} style={licenseRowStyles}>
              <a href={url} target="_blank" rel="noreferrer" style={linkStyles}>{name}</a>
              <span style={licenseBadgeStyles}>{license}</span>
            </div>
          ))}
        </div>

      </div>
    </div>
  );
}
