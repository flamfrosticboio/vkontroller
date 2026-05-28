import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'
import strip from '@rollup/plugin-strip';

export default defineConfig({
  plugins: [solid(),
  ...(process.env.NODE_ENV === 'production' ? [
    strip({
      functions: ['logger.debug', 'logger.info', '*.debug', '*.info', 'Option.log.debug', 'Option.log.info'],
      labels: ['DEV'],
      sourceMap: true
      })
  ] : []),
  ]
})
