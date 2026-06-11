import type { Meta, StoryObj } from '@storybook/react';
import { FileList } from '@lumik/ui';

const meta: Meta<typeof FileList> = {
  title: 'Components/FileList',
  component: FileList,
  parameters: {
    layout: 'padded',
  },
  tags: ['autodocs'],
  argTypes: {
    onRemove: { action: 'remove' },
  },
  decorators: [
    (Story) => (
      <div style={{ width: '100%', maxWidth: '900px' }}>
        <Story />
      </div>
    ),
  ],
};

export default meta;
type Story = StoryObj<typeof meta>;

const MB = 1024 * 1024;

const sampleFiles = [
  { name: 'IMG_0001.RAF', sizeBytes: 52.3 * MB },
  { name: 'IMG_0002.RAF', sizeBytes: 48.7 * MB },
  { name: 'IMG_0003.RAF', sizeBytes: 55.1 * MB },
  { name: 'IMG_0004.RAF', sizeBytes: 51.2 * MB },
  { name: 'IMG_0005.RAF', sizeBytes: 49.8 * MB },
  { name: 'IMG_0006.RAF', sizeBytes: 47.5 * MB },
];

const manyFiles = [
  ...sampleFiles,
  { name: 'IMG_0007.RAF', sizeBytes: 53.2 * MB },
  { name: 'IMG_0008.RAF', sizeBytes: 50.9 * MB },
  { name: 'IMG_0009.RAF', sizeBytes: 46.8 * MB },
  { name: 'IMG_0010.RAF', sizeBytes: 54.1 * MB },
  { name: 'IMG_0011.RAF', sizeBytes: 49.3 * MB },
  { name: 'IMG_0012.RAF', sizeBytes: 52.7 * MB },
  { name: 'DSC_1234.NEF', sizeBytes: 45.2 * MB },
  { name: 'DSC_1235.NEF', sizeBytes: 48.9 * MB },
  { name: 'DSC_1236.NEF', sizeBytes: 51.3 * MB },
];

export const Default: Story = {
  args: {
    title: 'Archivos seleccionados',
    files: sampleFiles,
    onRemove: (index: number) => console.log('Remove file at index:', index),
  },
};

export const WithSearch: Story = {
  args: {
    title: 'Archivos seleccionados',
    files: manyFiles,
    showSearch: true,
    onRemove: (index: number) => console.log('Remove file at index:', index),
  },
};

export const NoSearch: Story = {
  args: {
    title: 'Archivos seleccionados',
    files: manyFiles,
    showSearch: false,
    onRemove: (index: number) => console.log('Remove file at index:', index),
  },
};

export const FewFiles: Story = {
  args: {
    title: 'Archivos seleccionados',
    files: sampleFiles.slice(0, 3),
    onRemove: (index: number) => console.log('Remove file at index:', index),
  },
};

export const ManyFiles: Story = {
  args: {
    title: 'Archivos seleccionados',
    files: [
      ...manyFiles,
      { name: 'IMG_0016.RAF', sizeBytes: 50.1 * MB },
      { name: 'IMG_0017.RAF', sizeBytes: 52.4 * MB },
      { name: 'IMG_0018.RAF', sizeBytes: 49.7 * MB },
      { name: 'IMG_0019.RAF', sizeBytes: 53.8 * MB },
      { name: 'IMG_0020.RAF', sizeBytes: 47.2 * MB },
    ],
    maxHeight: '350px',
    onRemove: (index: number) => console.log('Remove file at index:', index),
  },
};

export const SingleFile: Story = {
  args: {
    title: 'Archivo seleccionado',
    files: [{ name: 'DSC_1234.NEF', sizeBytes: 45.2 * MB }],
    onRemove: (index: number) => console.log('Remove file at index:', index),
  },
};

export const LongFileNames: Story = {
  args: {
    title: 'Archivos con nombres largos',
    files: [
      { name: 'FUJI_SESSION_2024_01_15_WEDDING_CEREMONY_0001.RAF', sizeBytes: 52.3 * MB },
      { name: 'FUJI_SESSION_2024_01_15_WEDDING_CEREMONY_0002.RAF', sizeBytes: 48.7 * MB },
      { name: 'FUJI_SESSION_2024_01_15_WEDDING_RECEPTION_0001.RAF', sizeBytes: 55.1 * MB },
      { name: 'FUJI_SESSION_2024_01_15_WEDDING_RECEPTION_0002.RAF', sizeBytes: 51.2 * MB },
      { name: 'FUJI_SESSION_2024_01_15_PORTRAITS_0001.RAF', sizeBytes: 49.8 * MB },
      { name: 'FUJI_SESSION_2024_01_15_PORTRAITS_0002.RAF', sizeBytes: 47.5 * MB },
    ],
    onRemove: (index: number) => console.log('Remove file at index:', index),
  },
};

export const Empty: Story = {
  args: {
    title: 'Archivos seleccionados',
    files: [],
  },
};

export const ReadOnly: Story = {
  args: {
    title: 'Archivos (solo lectura)',
    files: sampleFiles,
  },
};

export const CustomSearchPlaceholder: Story = {
  args: {
    title: 'Archivos RAW',
    files: manyFiles,
    searchPlaceholder: 'Filtrar por nombre...',
    onRemove: (index: number) => console.log('Remove file at index:', index),
  },
};
