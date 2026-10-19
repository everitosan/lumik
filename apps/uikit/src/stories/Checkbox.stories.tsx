import type { Meta, StoryObj } from '@storybook/react';
import { useState } from 'react';
import { Checkbox } from '@lumik/ui';

const meta: Meta<typeof Checkbox> = {
  title: 'Components/Checkbox',
  component: Checkbox,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    disabled: {
      control: 'boolean',
    },
  },
  decorators: [
    (Story) => (
      <div style={{ width: '360px', padding: '24px' }}>
        <Story />
      </div>
    ),
  ],
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    label: 'Renombrar por fecha de captura',
    checked: true,
    onChange: () => {},
  },
};

export const WithDescription: Story = {
  args: {
    label: 'Firmar con mis datos',
    description: 'Autor, copyright y URL se escriben en cada foto',
    checked: true,
    onChange: () => {},
  },
};

export const Disabled: Story = {
  args: {
    label: 'Renombrar por fecha de captura',
    checked: false,
    disabled: true,
    onChange: () => {},
  },
};

export const Interactive: Story = {
  render: () => {
    const [checked, setChecked] = useState(false);
    return (
      <Checkbox
        label="Generar etiquetas para iPad y Mac"
        description={checked ? 'Activado' : 'Desactivado'}
        checked={checked}
        onChange={setChecked}
      />
    );
  },
};
