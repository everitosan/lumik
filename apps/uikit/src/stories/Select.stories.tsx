import type { Meta, StoryObj } from '@storybook/react';
import { useState } from 'react';
import { Select, Icon, type SelectOption } from '@lumik/ui';

const meta: Meta<typeof Select> = {
  title: 'Components/Select',
  component: Select,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    disabled: {
      control: 'boolean',
    },
    fullWidth: {
      control: 'boolean',
    },
  },
  decorators: [
    (Story) => (
      <div style={{ width: '320px', padding: '24px', minHeight: '300px' }}>
        <Story />
      </div>
    ),
  ],
};

export default meta;
type Story = StoryObj<typeof meta>;

const projectOptions: SelectOption[] = [
  { value: 'sesion-otono', label: 'Sesión Exterior Otoño 2023' },
  { value: 'boda-elena', label: 'Boda Elena & Marc' },
  { value: 'new', label: '+ Crear nuevo proyecto', isAction: true },
];

export const Default: Story = {
  args: {
    options: projectOptions,
    value: 'new',
    label: 'Asignar a proyecto',
  },
};

export const WithSelection: Story = {
  args: {
    options: projectOptions,
    value: 'sesion-otono',
    label: 'Asignar a proyecto',
  },
};

export const NoSelection: Story = {
  args: {
    options: projectOptions,
    value: undefined,
    label: 'Asignar a proyecto',
    placeholder: 'Seleccionar proyecto...',
  },
};

export const Disabled: Story = {
  args: {
    options: projectOptions,
    value: 'boda-elena',
    label: 'Asignar a proyecto',
    disabled: true,
  },
};

export const WithoutLabel: Story = {
  args: {
    options: projectOptions,
    value: 'new',
  },
};

export const FullWidth: Story = {
  args: {
    options: projectOptions,
    value: 'sesion-otono',
    label: 'Asignar a proyecto',
    fullWidth: true,
  },
};

export const ManyOptions: Story = {
  args: {
    options: [
      { value: 'p1', label: 'Proyecto Alpha' },
      { value: 'p2', label: 'Proyecto Beta' },
      { value: 'p3', label: 'Proyecto Gamma' },
      { value: 'p4', label: 'Proyecto Delta' },
      { value: 'p5', label: 'Proyecto Epsilon' },
      { value: 'p6', label: 'Proyecto Zeta' },
      { value: 'p7', label: 'Proyecto Eta' },
      { value: 'p8', label: 'Proyecto Theta' },
      { value: 'new', label: '+ Crear nuevo proyecto', isAction: true },
    ],
    value: 'p1',
    label: 'Seleccionar proyecto',
  },
};

export const WithIcons: Story = {
  args: {
    options: [
      {
        value: 'folder-photos',
        label: 'Fotos',
        icon: <Icon name="folder" size="sm" color="var(--lumik-on-surface-variant)" />,
      },
      {
        value: 'folder-videos',
        label: 'Videos',
        icon: <Icon name="folder" size="sm" color="var(--lumik-on-surface-variant)" />,
      },
      {
        value: 'folder-exports',
        label: 'Exportados',
        icon: <Icon name="folder" size="sm" color="var(--lumik-on-surface-variant)" />,
      },
    ],
    value: 'folder-photos',
    label: 'Carpeta destino',
  },
};

export const Interactive: Story = {
  render: () => {
    const [value, setValue] = useState<string>('new');

    return (
      <Select
        options={projectOptions}
        value={value}
        onChange={(newValue) => setValue(newValue)}
        label="Asignar a proyecto"
      />
    );
  },
};

export const ImportFlowExample: Story = {
  render: () => {
    const [project, setProject] = useState<string>('new');

    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
        <div>
          <h3 style={{
            color: '#e5e2e1',
            margin: '0 0 8px 0',
            fontFamily: 'Inter',
            fontSize: '18px',
            fontWeight: 500,
          }}>
            Configurar importación
          </h3>
          <p style={{
            color: '#8c90a0',
            margin: 0,
            fontFamily: 'Inter',
            fontSize: '14px',
          }}>
            Selecciona o crea un proyecto para organizar las fotos
          </p>
        </div>

        <Select
          options={projectOptions}
          value={project}
          onChange={setProject}
          label="Asignar a proyecto"
          fullWidth
        />

        {project === 'new' && (
          <div style={{
            padding: '12px',
            backgroundColor: 'rgba(176, 198, 255, 0.1)',
            borderRadius: '4px',
            border: '1px solid rgba(176, 198, 255, 0.2)',
          }}>
            <p style={{
              color: '#b0c6ff',
              margin: 0,
              fontFamily: 'Inter',
              fontSize: '13px',
            }}>
              Se creará un nuevo proyecto con las fotos importadas
            </p>
          </div>
        )}
      </div>
    );
  },
};
