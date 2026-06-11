import type { Meta, StoryObj } from '@storybook/react';
import { useState } from 'react';
import { DriveSelector, type Drive } from '@lumik/ui';

const meta: Meta<typeof DriveSelector> = {
  title: 'Components/DriveSelector',
  component: DriveSelector,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  decorators: [
    (Story) => (
      <div style={{ width: '480px', padding: '24px' }}>
        <Story />
      </div>
    ),
  ],
};

export default meta;
type Story = StoryObj<typeof meta>;

const sampleDrives: Drive[] = [
  {
    id: 'wd-photos',
    name: 'WD_Photos',
    uuid: 'a1b2c3d4-e5f6-7890-abcd-ef1234567890',
    totalBytes: 2 * 1024 * 1024 * 1024 * 1024, // 2TB
    usedBytes: 200 * 1024 * 1024 * 1024, // 200GB
    connected: true,
  },
  {
    id: 'macintosh-hd',
    name: 'Macintosh HD',
    uuid: 'f0e1d2c3-b4a5-9687-fedc-ba0987654321',
    totalBytes: 500 * 1024 * 1024 * 1024, // 500GB
    usedBytes: 450 * 1024 * 1024 * 1024, // 450GB (casi lleno)
    connected: true,
  },
  {
    id: 'backup-drive',
    name: 'Backup_2024',
    uuid: '12345678-90ab-cdef-1234-567890abcdef',
    totalBytes: 4 * 1024 * 1024 * 1024 * 1024, // 4TB
    usedBytes: 1.5 * 1024 * 1024 * 1024 * 1024, // 1.5TB
    connected: false,
  },
];

export const Default: Story = {
  args: {
    drives: sampleDrives,
    selectedId: 'wd-photos',
    requiredBytes: 50 * 1024 * 1024 * 1024, // 50GB
  },
};

export const NoSelection: Story = {
  args: {
    drives: sampleDrives,
    selectedId: undefined,
    requiredBytes: 50 * 1024 * 1024 * 1024,
  },
};

export const InsufficientSpace: Story = {
  args: {
    drives: sampleDrives,
    requiredBytes: 100 * 1024 * 1024 * 1024, // 100GB requeridos, Macintosh HD queda disabled
  },
};

export const CustomLabel: Story = {
  args: {
    drives: sampleDrives,
    selectedId: 'wd-photos',
    label: 'Disco de respaldo',
    requiredBytes: 50 * 1024 * 1024 * 1024,
  },
};

export const SingleDrive: Story = {
  args: {
    drives: [sampleDrives[0]],
    selectedId: 'wd-photos',
    requiredBytes: 50 * 1024 * 1024 * 1024,
  },
};

export const Interactive: Story = {
  render: () => {
    const [selectedId, setSelectedId] = useState<string | undefined>(undefined);

    return (
      <DriveSelector
        drives={sampleDrives}
        selectedId={selectedId}
        onSelect={(drive) => setSelectedId(drive.id)}
        requiredBytes={50 * 1024 * 1024 * 1024}
        label="Seleccionar destino"
      />
    );
  },
};

export const ImportFlow: Story = {
  render: () => {
    const [selectedId, setSelectedId] = useState<string>('wd-photos');

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
            Paso 2: Destino
          </h3>
          <p style={{
            color: '#8c90a0',
            margin: 0,
            fontFamily: 'Inter',
            fontSize: '14px',
          }}>
            Selecciona el disco donde se guardarán las fotos importadas
          </p>
        </div>

        <DriveSelector
          drives={sampleDrives}
          selectedId={selectedId}
          onSelect={(drive) => setSelectedId(drive.id)}
          requiredBytes={75 * 1024 * 1024 * 1024}
          label="Discos disponibles"
        />

        <div style={{ display: 'flex', gap: '12px', justifyContent: 'flex-end' }}>
          <button
            style={{
              padding: '10px 20px',
              backgroundColor: 'transparent',
              color: '#e5e2e1',
              border: '1px solid #424654',
              borderRadius: '4px',
              cursor: 'pointer',
              fontFamily: 'Inter',
              fontSize: '14px',
            }}
          >
            Cancelar
          </button>
          <button
            style={{
              padding: '10px 20px',
              backgroundColor: '#558dff',
              color: '#002d6e',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontFamily: 'Inter',
              fontSize: '14px',
              fontWeight: 500,
            }}
          >
            Siguiente
          </button>
        </div>
      </div>
    );
  },
};
