import type { Meta, StoryObj } from '@storybook/react';
import { PhotoCard } from '@lumik/ui';

const meta: Meta<typeof PhotoCard> = {
  title: 'Components/PhotoCard',
  component: PhotoCard,
  parameters: {
    layout: 'centered',
    backgrounds: {
      default: 'dark',
      values: [{ name: 'dark', value: '#131313' }],
    },
  },
  tags: ['autodocs'],
  argTypes: {
    stars: {
      control: { type: 'range', min: 0, max: 5, step: 1 },
    },
    culled: { control: 'boolean' },
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    filename: 'IMG_0047.dng',
    stars: 0,
    colorLabels: [],
    culled: false,
  },
  decorators: [(Story) => <div style={{ width: 220 }}><Story /></div>],
};

export const WithThumbnail: Story = {
  args: {
    filename: 'IMG_0047.dng',
    thumbnailUrl:
      'https://images.unsplash.com/photo-1519741497674-611481863552?w=400&h=300&fit=crop',
    stars: 3,
    captureDate: '2024-03-15T10:42:33',
    colorLabels: [3],
  },
  decorators: [(Story) => <div style={{ width: 220 }}><Story /></div>],
};

export const MultipleLabels: Story = {
  args: {
    filename: 'IMG_0132.dng',
    thumbnailUrl:
      'https://images.unsplash.com/photo-1511895426328-dc8714191300?w=400&h=300&fit=crop',
    stars: 5,
    captureDate: '2024-03-15T18:05:01',
    colorLabels: [1, 3, 5],
  },
  decorators: [(Story) => <div style={{ width: 220 }}><Story /></div>],
};

export const Culled: Story = {
  args: {
    filename: 'IMG_0099.dng',
    thumbnailUrl:
      'https://images.unsplash.com/photo-1560179707-f14e90ef3623?w=400&h=300&fit=crop',
    stars: 1,
    captureDate: '2024-03-15T14:22:09',
    colorLabels: [],
    culled: true,
  },
  decorators: [(Story) => <div style={{ width: 220 }}><Story /></div>],
};

export const NoThumbnail: Story = {
  args: {
    filename: 'IMG_0300.dng',
    stars: 2,
    captureDate: '2024-03-16T09:11:45',
    colorLabels: [2],
  },
  decorators: [(Story) => <div style={{ width: 220 }}><Story /></div>],
};

export const Grid: Story = {
  render: () => (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(3, 220px)',
        gap: '16px',
        padding: '24px',
        backgroundColor: '#131313',
      }}
    >
      <PhotoCard
        filename="IMG_0047.dng"
        thumbnailUrl="https://images.unsplash.com/photo-1519741497674-611481863552?w=400&h=300&fit=crop"
        stars={3}
        captureDate="2024-03-15T10:42:33"
        colorLabels={[3]}
      />
      <PhotoCard
        filename="IMG_0048.dng"
        thumbnailUrl="https://images.unsplash.com/photo-1511895426328-dc8714191300?w=400&h=300&fit=crop"
        stars={5}
        captureDate="2024-03-15T10:43:07"
        colorLabels={[1, 4]}
      />
      <PhotoCard
        filename="IMG_0049.dng"
        thumbnailUrl="https://images.unsplash.com/photo-1472214103451-9374bd1c798e?w=400&h=300&fit=crop"
        stars={0}
        captureDate="2024-03-15T10:44:21"
        colorLabels={[]}
      />
      <PhotoCard
        filename="IMG_0050.dng"
        thumbnailUrl="https://images.unsplash.com/photo-1560179707-f14e90ef3623?w=400&h=300&fit=crop"
        stars={2}
        captureDate="2024-03-15T10:45:55"
        colorLabels={[]}
        culled
      />
      <PhotoCard
        filename="IMG_0051.dng"
        stars={4}
        captureDate="2024-03-15T10:47:02"
        colorLabels={[1, 2, 3, 4, 5]}
      />
      <PhotoCard
        filename="IMG_0052_very_long_name_truncated.dng"
        thumbnailUrl="https://images.unsplash.com/photo-1531746020798-e6953c6e8e04?w=400&h=300&fit=crop"
        stars={1}
        captureDate="2024-03-15T10:48:30"
        colorLabels={[2, 5]}
      />
    </div>
  ),
};
