import type { Meta, StoryObj } from '@storybook/react';
import { useState } from 'react';
import { LogoSplash } from '@lumik/ui';

const meta: Meta<typeof LogoSplash> = {
  title: 'Components/LogoSplash',
  component: LogoSplash,
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component:
          'Splash de arranque (spec `app-splash`): el libro se abre en ~1.5 s y se quita solo. Con `prefers-reduced-motion: reduce` muestra el logo quieto (emúlalo en DevTools → Rendering).',
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

/** Se vuelve a ver con "Repetir"; debajo, un texto que hace las veces de la app. */
export const Default: Story = {
  render: (args) => {
    const [run, setRun] = useState(0);
    const [shown, setShown] = useState(true);
    return (
      <div style={{ minHeight: '100vh', background: '#131313', color: '#e5e2e1', padding: 24 }}>
        <p>La app, debajo del splash.</p>
        <button
          type="button"
          onClick={() => {
            setShown(true);
            setRun((r) => r + 1);
          }}
          disabled={shown}
        >
          Repetir
        </button>
        {shown && <LogoSplash key={run} {...args} onDone={() => setShown(false)} />}
      </div>
    );
  },
};
