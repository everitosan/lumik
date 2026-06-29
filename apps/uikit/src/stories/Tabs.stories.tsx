import type { Meta, StoryObj } from '@storybook/react';
import { useState } from 'react';
import { Tabs, type TabItem } from '@lumik/ui';

const meta: Meta<typeof Tabs> = {
  title: 'Components/Tabs',
  component: Tabs,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    variant: {
      control: 'select',
      options: ['line', 'pill'],
    },
    size: {
      control: 'select',
      options: ['sm', 'md', 'lg'],
    },
    fullWidth: {
      control: 'boolean',
    },
  },
  decorators: [
    (Story) => (
      <div style={{ width: '520px', padding: '24px' }}>
        <Story />
      </div>
    ),
  ],
};

export default meta;
type Story = StoryObj<typeof meta>;

const settingsTabs: TabItem[] = [
  { id: 'appearance', label: 'Apariencia', icon: 'settings' },
  { id: 'import', label: 'Importar', icon: 'import' },
  { id: 'shortcuts', label: 'Shortcuts', icon: 'aperture' },
];

const plainTabs: TabItem[] = [
  { id: 'appearance', label: 'Apariencia' },
  { id: 'import', label: 'Importar' },
  { id: 'shortcuts', label: 'Shortcuts' },
];

export const Default: Story = {
  args: {
    tabs: settingsTabs,
    activeTab: 'appearance',
  },
  render: (args) => {
    const [active, setActive] = useState(args.activeTab);
    return <Tabs {...args} activeTab={active} onChange={setActive} />;
  },
};

export const Pill: Story = {
  args: {
    tabs: settingsTabs,
    activeTab: 'appearance',
    variant: 'pill',
  },
  render: (args) => {
    const [active, setActive] = useState(args.activeTab);
    return <Tabs {...args} activeTab={active} onChange={setActive} />;
  },
};

export const WithoutIcons: Story = {
  args: {
    tabs: plainTabs,
    activeTab: 'appearance',
  },
  render: (args) => {
    const [active, setActive] = useState(args.activeTab);
    return <Tabs {...args} activeTab={active} onChange={setActive} />;
  },
};

export const FullWidth: Story = {
  args: {
    tabs: settingsTabs,
    activeTab: 'appearance',
    fullWidth: true,
  },
  render: (args) => {
    const [active, setActive] = useState(args.activeTab);
    return <Tabs {...args} activeTab={active} onChange={setActive} />;
  },
};

export const WithDisabled: Story = {
  args: {
    tabs: [
      { id: 'appearance', label: 'Apariencia', icon: 'settings' },
      { id: 'import', label: 'Importar', icon: 'import' },
      { id: 'shortcuts', label: 'Shortcuts', icon: 'aperture', disabled: true },
    ],
    activeTab: 'appearance',
  },
  render: (args) => {
    const [active, setActive] = useState(args.activeTab);
    return <Tabs {...args} activeTab={active} onChange={setActive} />;
  },
};

export const Sizes: Story = {
  render: () => {
    const [active, setActive] = useState('appearance');
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '32px' }}>
        {(['sm', 'md', 'lg'] as const).map((size) => (
          <div key={size}>
            <p style={{ color: '#8c90a0', marginBottom: '12px', fontSize: '12px' }}>{size}</p>
            <Tabs tabs={settingsTabs} activeTab={active} onChange={setActive} size={size} />
          </div>
        ))}
      </div>
    );
  },
};

export const WithPanel: Story = {
  render: () => {
    const [active, setActive] = useState('appearance');
    const content: Record<string, string> = {
      appearance: 'Configura el tema, idioma y densidad de la interfaz.',
      import: 'Define carpetas de destino y reglas de organización al importar.',
      shortcuts: 'Personaliza los atajos de teclado del culling.',
    };
    return (
      <div>
        <Tabs tabs={settingsTabs} activeTab={active} onChange={setActive} />
        <div
          role="tabpanel"
          id={`tabpanel-${active}`}
          aria-labelledby={`tab-${active}`}
          style={{
            marginTop: '20px',
            color: 'var(--lumik-on-surface, #e5e2e1)',
            fontFamily: 'var(--lumik-font-primary, Inter)',
            fontSize: '14px',
          }}
        >
          {content[active]}
        </div>
      </div>
    );
  },
};
