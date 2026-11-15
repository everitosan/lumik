import { useState } from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { ProjectCard } from '@lumik/ui';

const meta: Meta<typeof ProjectCard> = {
  title: 'Components/ProjectCard',
  component: ProjectCard,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    status: {
      control: 'select',
      options: ['imported', 'edited', 'delivered'],
    },
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    name: 'Boda Martinez-Lopez',
    photoCount: 324,
    date: '2024-03-15',
    driveName: 'WD_Photos',
    status: 'edited',
  },
};

export const Importada: Story = {
  args: {
    name: 'Sesión Familia Ruiz',
    photoCount: 156,
    date: '2024-03-20',
    driveName: 'Backup_SSD',
    status: 'imported',
  },
};

export const Entregada: Story = {
  args: {
    name: 'Corporativo TechCorp',
    photoCount: 89,
    date: '2024-02-28',
    driveName: 'WD_Photos',
    status: 'delivered',
  },
};

export const Archived: Story = {
  args: {
    name: 'Corporativo TechCorp',
    photoCount: 89,
    date: '2024-02-28',
    driveName: 'WD_Photos',
    status: 'delivered',
    archived: true,
    thumbnailUrl: 'https://images.unsplash.com/photo-1560179707-f14e90ef3623?w=400&h=300&fit=crop',
    onEdit: () => {},
    onToggleArchive: () => {},
    onDeleteBackup: () => {},
    onTrash: () => {},
  },
};

export const WithThumbnail: Story = {
  args: {
    name: 'Boda Martinez-Lopez',
    photoCount: 324,
    date: '2024-03-15',
    driveName: 'WD_Photos',
    status: 'edited',
    thumbnailUrl: 'https://images.unsplash.com/photo-1519741497674-611481863552?w=400&h=300&fit=crop',
  },
};

/** Abierto en otro dispositivo (candado del proyecto). */
export const LockedByOtherDevice: Story = {
  args: {
    name: 'XV Ana',
    photoCount: 1204,
    date: '2026-10-18',
    driveName: 'Samsung T7',
    status: 'edited',
    lockedBy: 'iPad de Ana',
    thumbnailUrl: 'https://images.unsplash.com/photo-1519741497674-611481863552?w=400&h=300&fit=crop',
    onEdit: () => {},
    onTrash: () => {},
  },
};

/** En la sección "Papelera": fecha en que se mandó, restaurar y eliminar definitivamente. */
export const InTrash: Story = {
  args: {
    name: 'Risespray',
    photoCount: 3,
    date: '2026-09-18',
    driveName: 'KINGSTON',
    status: 'delivered',
    trashedAt: '2026-11-10T19:54:03',
    thumbnailUrl: 'https://images.unsplash.com/photo-1519741497674-611481863552?w=400&h=300&fit=crop',
    onRestore: () => console.log('[ProjectCard] restore'),
    onPurge: () => console.log('[ProjectCard] purge'),
  },
};

/** Marcable en la sección "Papelera" (spec photo-marking). */
export const Markable: Story = {
  render: function Render() {
    const [marked, setMarked] = useState(false);
    return (
      <div style={{ width: 280 }}>
        <ProjectCard
          name="Risespray"
          photoCount={3}
          date="2026-09-18"
          driveName="KINGSTON"
          status="delivered"
          trashedAt="2026-11-10T19:54:03"
          marked={marked}
          onToggleMark={() => setMarked((m) => !m)}
          onRestore={() => {}}
          onPurge={() => {}}
        />
      </div>
    );
  },
};

/** Marcado: borde primario y casilla visible. */
export const Marked: Story = {
  args: {
    name: 'Sony a600',
    photoCount: 19,
    date: '2026-09-18',
    driveName: 'KINGSTON',
    status: 'edited',
    trashedAt: '2026-11-10T19:54:03',
    marked: true,
    onToggleMark: () => {},
  },
};

export const LongName: Story = {
  args: {
    name: 'Sesión de fotos para el catálogo de primavera-verano 2024',
    photoCount: 1250,
    date: '2024-01-10',
    driveName: 'Archive_Drive',
    status: 'delivered',
  },
};

export const Grid: Story = {
  render: () => (
    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 280px)', gap: '24px' }}>
      <ProjectCard
        name="Boda Martinez-Lopez"
        photoCount={324}
        date="2024-03-15"
        driveName="WD_Photos"
        status="edited"
        thumbnailUrl="https://images.unsplash.com/photo-1519741497674-611481863552?w=400&h=300&fit=crop"
      />
      <ProjectCard
        name="Sesión Familia Ruiz"
        photoCount={156}
        date="2024-03-20"
        driveName="Backup_SSD"
        status="imported"
        thumbnailUrl="https://images.unsplash.com/photo-1511895426328-dc8714191300?w=400&h=300&fit=crop"
      />
      <ProjectCard
        name="Corporativo TechCorp"
        photoCount={89}
        date="2024-02-28"
        driveName="WD_Photos"
        status="delivered"
        thumbnailUrl="https://images.unsplash.com/photo-1560179707-f14e90ef3623?w=400&h=300&fit=crop"
      />
      <ProjectCard
        name="XV Años Sofia"
        photoCount={412}
        date="2024-03-01"
        driveName="Archive"
        status="delivered"
      />
      <ProjectCard
        name="Producto Joyería Luna"
        photoCount={78}
        date="2024-03-22"
        driveName="WD_Photos"
        status="imported"
      />
      <ProjectCard
        name="Retrato Ejecutivo"
        photoCount={24}
        date="2024-03-25"
        driveName="Backup_SSD"
        status="edited"
      />
    </div>
  ),
};
