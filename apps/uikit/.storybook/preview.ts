import type { Preview } from '@storybook/react';
import '@lumik/ui/tokens.css';
import '../src/styles.css';

const preview: Preview = {
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    backgrounds: {
      default: 'lumik-dark',
      values: [
        {
          name: 'lumik-dark',
          value: '#131313',
        },
        {
          name: 'lumik-surface',
          value: '#201f1f',
        },
        {
          name: 'white',
          value: '#ffffff',
        },
      ],
    },
  },
};

export default preview;
