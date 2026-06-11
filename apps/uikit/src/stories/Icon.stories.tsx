import type { Meta, StoryObj } from '@storybook/react';
import { Icon, type IconName } from '@lumik/ui';

const allIcons: IconName[] = [
  'search', 'projects', 'import', 'settings', 'drive', 'image', 'photo',
  'menu', 'calendar', 'aperture', 'check', 'x', 'chevron-down', 'chevron-right',
  'plus', 'star', 'star-filled', 'trash', 'edit', 'folder', 'folder-open',
];

const meta: Meta<typeof Icon> = {
  title: 'Components/Icon',
  component: Icon,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    name: {
      control: 'select',
      options: allIcons,
    },
    size: {
      control: 'select',
      options: ['sm', 'md', 'lg', 'xl'],
    },
    color: {
      control: 'color',
    },
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    name: 'aperture',
    size: 'xl',
  },
};

export const AllIcons: Story = {
  render: () => (
    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: '24px' }}>
      {allIcons.map((name) => (
        <div
          key={name}
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: '8px',
          }}
        >
          <Icon name={name} size="xl" />
          <span style={{ fontSize: '10px', color: '#8c90a0' }}>{name}</span>
        </div>
      ))}
    </div>
  ),
};

export const Sizes: Story = {
  render: () => (
    <div style={{ display: 'flex', alignItems: 'center', gap: '24px' }}>
      <div style={{ textAlign: 'center' }}>
        <Icon name="aperture" size="sm" />
        <div style={{ fontSize: '10px', color: '#8c90a0', marginTop: '4px' }}>sm (12px)</div>
      </div>
      <div style={{ textAlign: 'center' }}>
        <Icon name="aperture" size="md" />
        <div style={{ fontSize: '10px', color: '#8c90a0', marginTop: '4px' }}>md (16px)</div>
      </div>
      <div style={{ textAlign: 'center' }}>
        <Icon name="aperture" size="lg" />
        <div style={{ fontSize: '10px', color: '#8c90a0', marginTop: '4px' }}>lg (20px)</div>
      </div>
      <div style={{ textAlign: 'center' }}>
        <Icon name="aperture" size="xl" />
        <div style={{ fontSize: '10px', color: '#8c90a0', marginTop: '4px' }}>xl (24px)</div>
      </div>
      <div style={{ textAlign: 'center' }}>
        <Icon name="aperture" size={48} />
        <div style={{ fontSize: '10px', color: '#8c90a0', marginTop: '4px' }}>custom (48px)</div>
      </div>
    </div>
  ),
};

export const Colors: Story = {
  render: () => (
    <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
      <Icon name="star-filled" size="xl" color="var(--lumik-primary)" />
      <Icon name="star-filled" size="xl" color="var(--lumik-secondary)" />
      <Icon name="star-filled" size="xl" color="var(--lumik-tertiary)" />
      <Icon name="star-filled" size="xl" color="var(--lumik-error)" />
    </div>
  ),
};
