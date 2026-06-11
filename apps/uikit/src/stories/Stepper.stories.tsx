import type { Meta, StoryObj } from '@storybook/react';
import { useState } from 'react';
import { Stepper } from '@lumik/ui';

const meta: Meta<typeof Stepper> = {
  title: 'Components/Stepper',
  component: Stepper,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    orientation: {
      control: 'select',
      options: ['horizontal', 'vertical'],
    },
    size: {
      control: 'select',
      options: ['sm', 'md', 'lg'],
    },
    currentStep: {
      control: { type: 'number', min: 0, max: 4 },
    },
  },
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

const importSteps = [
  { id: 'origen', label: 'Origen' },
  { id: 'destino', label: 'Destino' },
  { id: 'confirmar', label: 'Confirmar' },
];

export const Default: Story = {
  args: {
    steps: importSteps,
    currentStep: 1,
  },
};

export const FirstStep: Story = {
  args: {
    steps: importSteps,
    currentStep: 0,
  },
};

export const LastStep: Story = {
  args: {
    steps: importSteps,
    currentStep: 2,
  },
};

export const AllCompleted: Story = {
  args: {
    steps: importSteps,
    currentStep: 3,
  },
};

export const Vertical: Story = {
  args: {
    steps: importSteps,
    currentStep: 1,
    orientation: 'vertical',
  },
};

export const SmallSize: Story = {
  args: {
    steps: importSteps,
    currentStep: 1,
    size: 'sm',
  },
};

export const LargeSize: Story = {
  args: {
    steps: importSteps,
    currentStep: 1,
    size: 'lg',
  },
};

export const ManySteps: Story = {
  args: {
    steps: [
      { id: 'step1', label: 'Seleccionar' },
      { id: 'step2', label: 'Configurar' },
      { id: 'step3', label: 'Procesar' },
      { id: 'step4', label: 'Verificar' },
      { id: 'step5', label: 'Completar' },
    ],
    currentStep: 2,
  },
};

export const Interactive: Story = {
  render: () => {
    const [step, setStep] = useState(1);

    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
        <Stepper
          steps={importSteps}
          currentStep={step}
          onStepClick={(index) => setStep(index)}
        />
        <div style={{ display: 'flex', gap: '12px', justifyContent: 'center' }}>
          <button
            onClick={() => setStep(Math.max(0, step - 1))}
            disabled={step === 0}
            style={{
              padding: '8px 16px',
              backgroundColor: 'var(--lumik-surface-container-high, #2a2a2a)',
              color: 'var(--lumik-on-surface, #e5e2e1)',
              border: '1px solid var(--lumik-outline-variant, #424654)',
              borderRadius: '4px',
              cursor: step === 0 ? 'not-allowed' : 'pointer',
              opacity: step === 0 ? 0.5 : 1,
            }}
          >
            Anterior
          </button>
          <button
            onClick={() => setStep(Math.min(importSteps.length, step + 1))}
            disabled={step >= importSteps.length}
            style={{
              padding: '8px 16px',
              backgroundColor: 'var(--lumik-primary-container, #558dff)',
              color: 'var(--lumik-on-primary, #002d6e)',
              border: 'none',
              borderRadius: '4px',
              cursor: step >= importSteps.length ? 'not-allowed' : 'pointer',
              opacity: step >= importSteps.length ? 0.5 : 1,
            }}
          >
            Siguiente
          </button>
        </div>
      </div>
    );
  },
};

export const AllSizes: Story = {
  render: () => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '32px' }}>
      <div>
        <p style={{ color: '#8c90a0', marginBottom: '12px', fontSize: '12px' }}>Small</p>
        <Stepper steps={importSteps} currentStep={1} size="sm" />
      </div>
      <div>
        <p style={{ color: '#8c90a0', marginBottom: '12px', fontSize: '12px' }}>Medium</p>
        <Stepper steps={importSteps} currentStep={1} size="md" />
      </div>
      <div>
        <p style={{ color: '#8c90a0', marginBottom: '12px', fontSize: '12px' }}>Large</p>
        <Stepper steps={importSteps} currentStep={1} size="lg" />
      </div>
    </div>
  ),
};
