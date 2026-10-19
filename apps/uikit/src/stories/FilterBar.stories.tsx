import type { Meta, StoryObj } from '@storybook/react';
import { useState, type CSSProperties } from 'react';
import { FilterBar, FilterBarDivider, FilterToggle, Icon } from '@lumik/ui';

const meta: Meta<typeof FilterBar> = {
  title: 'Components/FilterBar',
  component: FilterBar,
  parameters: {
    layout: 'fullscreen',
  },
  tags: ['autodocs'],
};

export default meta;
type Story = StoryObj<typeof meta>;

const statsStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '12px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const valueStyles: CSSProperties = { color: 'var(--lumik-on-surface, #e5e2e1)', fontWeight: 500 };
const sepStyles: CSSProperties = { color: 'var(--lumik-outline-variant, #424654)' };

const dropdownStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  padding: '6px 10px',
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  background: 'transparent',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
};

function Stats() {
  return (
    <div style={statsStyles}>
      <span style={valueStyles}>12</span> proyectos <span style={sepStyles}>•</span>
      <span style={valueStyles}>3,420</span> fotos
    </div>
  );
}

export const Dashboard: Story = {
  render: () => {
    const [showArchived, setShowArchived] = useState(false);
    const archivedStyles: CSSProperties = showArchived
      ? { ...dropdownStyles, color: 'var(--lumik-tertiary, #ffb690)', borderColor: 'var(--lumik-tertiary, #ffb690)' }
      : dropdownStyles;
    return (
      <div style={{ marginTop: '200px' }}>
        <FilterBar
          end={
            <>
              <button type="button" style={archivedStyles} onClick={() => setShowArchived(!showArchived)}>
                <Icon name="archive" size="sm" /> Archivados (3)
              </button>
              <FilterBarDivider />
              <button type="button" style={dropdownStyles}>
                <Icon name="sort" size="sm" /> Ordenar <Icon name="chevron-down" size="sm" />
              </button>
            </>
          }
        />
      </div>
    );
  },
};

export const WithDividers: Story = {
  render: () => (
    <div style={{ marginTop: '200px' }}>
      <FilterBar
        start={<Stats />}
        end={
          <>
            <button type="button" style={dropdownStyles}>Estrellas</button>
            <FilterBarDivider />
            <button type="button" style={dropdownStyles}>Color</button>
            <FilterBarDivider />
            <button type="button" style={dropdownStyles}>Tags</button>
          </>
        }
      />
    </div>
  ),
};

export const Narrow: Story = {
  render: () => (
    <div style={{ marginTop: '200px', width: '420px' }}>
      <FilterBar
        start={
          <>
            <Stats />
            <FilterToggle label="Mostrar solo aprobadas" checked={false} onChange={() => {}} />
          </>
        }
        end={
          <button type="button" style={dropdownStyles}>
            <Icon name="sort" size="sm" /> Ordenar
          </button>
        }
      />
    </div>
  ),
};
