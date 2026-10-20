import { useState } from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { CollapsibleSection, DriveInfo } from '@lumik/ui';

const meta: Meta<typeof CollapsibleSection> = {
  title: 'Components/CollapsibleSection',
  component: CollapsibleSection,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  decorators: [(Story) => <div style={{ width: 228 }}><Story /></div>],
};

export default meta;
type Story = StoryObj<typeof meta>;

const GB = 1073741824;

/** Estado controlado como en la barra lateral: clic o Enter/Espacio en el encabezado. */
function Interactive({ initialCollapsed, summary }: { initialCollapsed: boolean; summary?: string }) {
  const [collapsed, setCollapsed] = useState(initialCollapsed);
  return (
    <CollapsibleSection
      title="Almacenamiento"
      summary={summary}
      collapsed={collapsed}
      onToggle={() => setCollapsed((c) => !c)}
    >
      <DriveInfo name="KINGSTON" usedBytes={300 * GB} totalBytes={1000 * GB} />
    </CollapsibleSection>
  );
}

export const Expanded: Story = {
  render: () => <Interactive initialCollapsed={false} />,
};

export const CollapsedWithSummary: Story = {
  render: () => <Interactive initialCollapsed summary="1 disco" />,
};

export const CollapsedWithoutSummary: Story = {
  render: () => <Interactive initialCollapsed />,
};
