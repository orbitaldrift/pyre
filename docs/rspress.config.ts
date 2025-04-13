import * as path from 'node:path';
import { defineConfig } from 'rspress/config';

export default defineConfig({
  root: path.join(__dirname, 'docs'),
  base: '/pyre',
  title: 'Pyre',
  icon: '/odrift-logo.png',
  logo: {
    light: '/odrift-text.png',
    dark: '/odrift-text.png',
  },
  themeConfig: {
    socialLinks: [
      {
        icon: 'github',
        mode: 'link',
        content: 'https://github.com/orbitaldrift/pyre',
      },
    ],
  },
});
