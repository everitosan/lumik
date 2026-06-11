import type { Meta, StoryObj } from '@storybook/react';
import { SectionButton } from '@lumik/ui';

const meta: Meta<typeof SectionButton> = {
  title: 'Components/SectionButton',
  component: SectionButton,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    icon: {
      control: 'select',
      options: ['projects', 'import', 'settings', 'folder', 'drive'],
    },
    active: {
      control: 'boolean',
    },
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    icon: 'projects',
    label: 'Proyectos',
  },
};

export const Active: Story = {
  args: {
    icon: 'projects',
    label: 'Proyectos',
    active: true,
  },
};

export const WithoutIcon: Story = {
  args: {
    label: 'Solo texto',
  },
};

export const Navigation: Story = {
  render: () => (
    <div style={{ width: '240px', display: 'flex', flexDirection: 'column', gap: '4px' }}>
      <SectionButton icon="projects" label="Proyectos" active />
      <SectionButton icon="import" label="Importar" />
      <SectionButton icon="settings" label="Configuración" />
    </div>
  ),
};
