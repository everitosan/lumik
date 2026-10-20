import type { Meta, StoryObj } from '@storybook/react';
import { BackupJobCard } from '@lumik/ui';

const meta: Meta<typeof BackupJobCard> = {
  title: 'Components/BackupJobCard',
  component: BackupJobCard,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  decorators: [(Story) => <div style={{ width: 228 }}><Story /></div>],
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Running: Story = {
  args: {
    name: 'Boda Ana',
    done: 1,
    total: 20,
    running: true,
    onClick: () => console.log('[BackupJobCard] open detail'),
  },
};

/** Terminó con errores: "Reintentar" y × no deben abrir el detalle. */
export const WithErrors: Story = {
  args: {
    name: 'Risespray',
    done: 21,
    total: 23,
    failed: 2,
    running: false,
    onClick: () => console.log('[BackupJobCard] open detail'),
    onRetry: () => console.log('[BackupJobCard] retry'),
    onDismiss: () => console.log('[BackupJobCard] dismiss'),
  },
};

export const RetryNeedsDrive: Story = {
  args: {
    ...WithErrors.args,
    message: 'Conecta el disco de «Risespray» para reintentar.',
  },
};

export const LongName: Story = {
  args: {
    name: 'Pero mira si no es un buen día para fotografiar la ciudad',
    done: 12,
    total: 48,
    running: true,
    onClick: () => console.log('[BackupJobCard] open detail'),
  },
};
