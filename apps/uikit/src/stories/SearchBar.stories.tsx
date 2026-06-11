import type { Meta, StoryObj } from '@storybook/react';
import { SearchBar } from '@lumik/ui';

const meta: Meta<typeof SearchBar> = {
  title: 'Components/SearchBar',
  component: SearchBar,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    placeholder: {
      control: 'text',
    },
    disabled: {
      control: 'boolean',
    },
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    placeholder: 'Buscar proyectos...',
  },
};

export const WithValue: Story = {
  args: {
    placeholder: 'Buscar proyectos...',
    defaultValue: 'Boda Martinez',
  },
};

export const CustomPlaceholder: Story = {
  args: {
    placeholder: 'Buscar por nombre, fecha o etiqueta...',
  },
};

export const Disabled: Story = {
  args: {
    placeholder: 'Buscar...',
    disabled: true,
  },
};

export const InContainer: Story = {
  render: () => (
    <div style={{ width: '400px', padding: '24px', backgroundColor: 'var(--lumik-surface-container)' }}>
      <SearchBar placeholder="Buscar proyectos..." />
    </div>
  ),
};
