import type { Meta, StoryObj } from '@storybook/react';
import { useState } from 'react';
import { FilterToggle } from '@lumik/ui';

const meta: Meta<typeof FilterToggle> = {
  title: 'Components/FilterToggle',
  component: FilterToggle,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Off: Story = {
  args: {
    label: 'Mostrar solo aprobadas',
    checked: false,
    onChange: () => {},
  },
};

export const On: Story = {
  args: {
    label: 'Mostrar solo aprobadas',
    checked: true,
    onChange: () => {},
  },
};

export const WithCount: Story = {
  render: () => {
    const [checked, setChecked] = useState(false);
    return <FilterToggle label="Mostrar archivados (3)" checked={checked} onChange={setChecked} />;
  },
};
