import type { Meta, StoryObj } from '@storybook/react';
import { useState } from 'react';
import { DropZone, DropZoneSummary, type DropZoneFile } from '@lumik/ui';

const meta: Meta<typeof DropZone> = {
  title: 'Components/DropZone',
  component: DropZone,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  decorators: [
    (Story) => (
      <div style={{ width: '600px', padding: '24px' }}>
        <Story />
      </div>
    ),
  ],
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    title: 'Arrastra tus fotos aquí o haz clic para explorar',
    hint: 'Compatible con RAW, CR2, CR3, NEF, ARW, RAF, ORF, RW2, DNG, TIFF',
    multiple: true,
  },
};

export const CustomTitle: Story = {
  args: {
    title: 'Selecciona los archivos RAW de tu sesión',
    hint: 'Formatos soportados: CR2, CR3, NEF, ARW, DNG',
    acceptedExtensions: ['.cr2', '.cr3', '.nef', '.arw', '.dng'],
  },
};

export const SingleFile: Story = {
  args: {
    title: 'Selecciona un archivo',
    hint: 'Solo se permite un archivo a la vez',
    multiple: false,
  },
};

export const Disabled: Story = {
  args: {
    title: 'Arrastra tus fotos aquí o haz clic para explorar',
    hint: 'Compatible con RAW, CR2, CR3, NEF, ARW, RAF, ORF, RW2, DNG, TIFF',
    disabled: true,
  },
};

export const CompactHeight: Story = {
  args: {
    title: 'Arrastra archivos aquí',
    hint: 'RAW, DNG, TIFF',
    minHeight: '150px',
  },
};

export const Interactive: Story = {
  render: () => {
    const [files, setFiles] = useState<DropZoneFile[]>([]);

    const handleFilesAdded = (newFiles: DropZoneFile[]) => {
      setFiles((prev) => [...prev, ...newFiles]);
    };

    const totalBytes = files.reduce((sum, f) => sum + f.sizeBytes, 0);

    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <DropZone
          onFilesAdded={handleFilesAdded}
          title="Arrastra tus fotos aquí o haz clic para explorar"
          hint="Compatible con RAW, CR2, CR3, NEF, ARW, RAF, ORF, RW2, DNG, TIFF"
        />

        {files.length > 0 && (
          <DropZoneSummary fileCount={files.length} totalBytes={totalBytes} />
        )}
      </div>
    );
  },
};

export const WithFileList: Story = {
  render: () => {
    const [files, setFiles] = useState<DropZoneFile[]>([]);

    const handleFilesAdded = (newFiles: DropZoneFile[]) => {
      setFiles((prev) => [...prev, ...newFiles]);
    };

    const handleRemove = (index: number) => {
      setFiles((prev) => prev.filter((_, i) => i !== index));
    };

    const totalBytes = files.reduce((sum, f) => sum + f.sizeBytes, 0);

    const formatBytes = (bytes: number): string => {
      if (bytes === 0) return '0 B';
      const k = 1024;
      const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
    };

    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <DropZone
          onFilesAdded={handleFilesAdded}
          title="Arrastra tus fotos aquí o haz clic para explorar"
          hint="Compatible con RAW, CR2, CR3, NEF, ARW, RAF, ORF, RW2, DNG, TIFF"
        />

        {files.length > 0 && (
          <>
            <div style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '8px',
              maxHeight: '200px',
              overflowY: 'auto',
            }}>
              {files.map((file, index) => (
                <div
                  key={index}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '10px 12px',
                    backgroundColor: 'var(--lumik-surface-container, #201f1f)',
                    borderRadius: '4px',
                    border: '1px solid var(--lumik-outline-variant, #424654)',
                  }}
                >
                  <span style={{
                    fontFamily: 'Inter',
                    fontSize: '13px',
                    color: '#e5e2e1',
                  }}>
                    {file.name}
                  </span>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                    <span style={{
                      fontFamily: 'JetBrains Mono',
                      fontSize: '12px',
                      color: '#8c90a0',
                    }}>
                      {formatBytes(file.sizeBytes)}
                    </span>
                    <button
                      onClick={() => handleRemove(index)}
                      style={{
                        background: 'transparent',
                        border: 'none',
                        color: '#8c90a0',
                        cursor: 'pointer',
                        padding: '4px',
                        fontSize: '16px',
                        lineHeight: 1,
                      }}
                    >
                      ×
                    </button>
                  </div>
                </div>
              ))}
            </div>

            <DropZoneSummary fileCount={files.length} totalBytes={totalBytes} />
          </>
        )}
      </div>
    );
  },
};

export const ImportFlowStep1: Story = {
  render: () => {
    const [files, setFiles] = useState<DropZoneFile[]>([]);

    const handleFilesAdded = (newFiles: DropZoneFile[]) => {
      setFiles((prev) => [...prev, ...newFiles]);
    };

    const totalBytes = files.reduce((sum, f) => sum + f.sizeBytes, 0);

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
            Paso 1: Origen
          </h3>
          <p style={{
            color: '#8c90a0',
            margin: 0,
            fontFamily: 'Inter',
            fontSize: '14px',
          }}>
            Selecciona los archivos RAW que deseas importar
          </p>
        </div>

        <DropZone
          onFilesAdded={handleFilesAdded}
          title="Arrastra tus fotos aquí o haz clic para explorar"
          hint="Compatible con RAW, CR2, CR3, NEF, ARW, RAF, ORF, RW2, DNG, TIFF"
        />

        {files.length > 0 && (
          <DropZoneSummary fileCount={files.length} totalBytes={totalBytes} />
        )}

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
            disabled={files.length === 0}
            style={{
              padding: '10px 20px',
              backgroundColor: files.length > 0 ? '#558dff' : '#2a2a2a',
              color: files.length > 0 ? '#002d6e' : '#8c90a0',
              border: 'none',
              borderRadius: '4px',
              cursor: files.length > 0 ? 'pointer' : 'not-allowed',
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
