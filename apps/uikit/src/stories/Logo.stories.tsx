import type { Meta, StoryObj } from '@storybook/react';
import { Logo } from '@lumik/ui';

const meta: Meta<typeof Logo> = {
  title: 'Components/Logo',
  component: Logo,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    size: {
      control: 'select',
      options: ['sm', 'md', 'lg'],
    },
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    size: 'md',
  },
};

export const Small: Story = {
  args: {
    size: 'sm',
  },
};

export const Large: Story = {
  args: {
    size: 'lg',
  },
};

export const AllSizes: Story = {
  render: () => (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: '24px' }}>
      <div>
        <div style={{ fontSize: '12px', color: '#8c90a0', marginBottom: '8px' }}>Small</div>
        <Logo size="sm" />
      </div>
      <div>
        <div style={{ fontSize: '12px', color: '#8c90a0', marginBottom: '8px' }}>Medium</div>
        <Logo size="md" />
      </div>
      <div>
        <div style={{ fontSize: '12px', color: '#8c90a0', marginBottom: '8px' }}>Large</div>
        <Logo size="lg" />
      </div>
    </div>
  ),
};
