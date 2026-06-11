import type { Meta, StoryObj } from '@storybook/react';
import { DriveInfo } from '@lumik/ui';

const meta: Meta<typeof DriveInfo> = {
  title: 'Components/DriveInfo',
  component: DriveInfo,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    connected: {
      control: 'boolean',
    },
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

// 1TB = 1099511627776 bytes
const TB = 1099511627776;
const GB = 1073741824;

export const Default: Story = {
  args: {
    name: 'WD_Photos',
    uuid: 'a3f2b1c4-5678-90ab-cdef-1234567890ab',
    usedBytes: 800 * GB,
    totalBytes: 2 * TB,
    connected: true,
  },
};

export const AlmostFull: Story = {
  args: {
    name: 'Backup_Drive',
    uuid: 'b4c3d2e1-9876-54ab-cdef-abcdef123456',
    usedBytes: 1.8 * TB,
    totalBytes: 2 * TB,
    connected: true,
  },
};

export const Critical: Story = {
  args: {
    name: 'Old_Drive',
    uuid: 'c5d4e3f2-1234-56ab-cdef-fedcba654321',
    usedBytes: 1.95 * TB,
    totalBytes: 2 * TB,
    connected: true,
  },
};

export const Disconnected: Story = {
  args: {
    name: 'External_SSD',
    uuid: 'd6e5f4a3-abcd-ef12-3456-789012345678',
    usedBytes: 500 * GB,
    totalBytes: 1 * TB,
    connected: false,
  },
};

export const MultipleDisks: Story = {
  render: () => (
    <div style={{ width: '240px', display: 'flex', flexDirection: 'column', gap: '12px' }}>
      <DriveInfo
        name="WD_Photos"
        uuid="a3f2b1c4-5678-90ab"
        usedBytes={800 * GB}
        totalBytes={2 * TB}
        connected={true}
      />
      <DriveInfo
        name="Backup_SSD"
        uuid="b4c3d2e1-9876-54ab"
        usedBytes={450 * GB}
        totalBytes={1 * TB}
        connected={true}
      />
      <DriveInfo
        name="Archive"
        uuid="c5d4e3f2-1234-56ab"
        usedBytes={3.5 * TB}
        totalBytes={4 * TB}
        connected={false}
      />
    </div>
  ),
};
