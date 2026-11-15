import type { Meta, StoryObj } from '@storybook/react';
import { MarkedBar } from '@lumik/ui';

const meta: Meta<typeof MarkedBar> = {
  title: 'Components/MarkedBar',
  component: MarkedBar,
  parameters: { layout: 'padded' },
};

export default meta;
type Story = StoryObj<typeof MarkedBar>;

/** Grid del proyecto con 3 fotos marcadas (spec photo-marking). */
export const Photos: Story = {
  args: {
    count: 3,
    noun: 'photos',
    onClear: () => console.log('[MarkedBar] clear'),
    actions: [
      { icon: 'album', label: 'Agregar a álbum', onClick: () => console.log('[MarkedBar] album') },
      { icon: 'trash', label: 'Mover a la papelera', danger: true, onClick: () => console.log('[MarkedBar] trash') },
    ],
  },
};

/** Sección "Papelera" con una acción en curso. */
export const ItemsBusy: Story = {
  args: {
    count: 7,
    noun: 'items',
    busy: 'Eliminando…',
    onClear: () => {},
    actions: [
      { icon: 'archive-restore', label: 'Restaurar', onClick: () => {} },
      { icon: 'trash', label: 'Eliminar definitivamente', danger: true, onClick: () => {} },
    ],
  },
};
