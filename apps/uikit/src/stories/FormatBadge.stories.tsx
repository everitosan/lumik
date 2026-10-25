import { useState } from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { FormatBadge } from '@lumik/ui';

const meta: Meta<typeof FormatBadge> = {
  title: 'Components/FormatBadge',
  component: FormatBadge,
  parameters: {
    layout: 'centered',
    backgrounds: {
      default: 'dark',
      values: [{ name: 'dark', value: '#131313' }, { name: 'light', value: '#f4f4f4' }],
    },
  },
  tags: ['autodocs'],
  argTypes: {
    format: { control: 'select', options: ['RAW+JPG', 'RAW', 'JPG'] },
    showing: { control: 'select', options: [undefined, 'RAW', 'JPG'] },
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Pair: Story = { args: { format: 'RAW+JPG' } };
export const Raw: Story = { args: { format: 'RAW' } };
export const Jpg: Story = { args: { format: 'JPG' } };

/** En el visor: el par se abre en su JPG y el indicador cambia al RAW. */
export const ToggleInViewer: Story = {
  args: { format: 'RAW+JPG' },
  render: (args) => {
    const [showing, setShowing] = useState<'RAW' | 'JPG'>('JPG');
    return <FormatBadge {...args} showing={showing} onToggle={() => setShowing((s) => (s === 'JPG' ? 'RAW' : 'JPG'))} />;
  },
};
