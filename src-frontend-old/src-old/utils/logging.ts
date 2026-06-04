import log from 'loglevel';

log.setLevel(import.meta.env.DEV ? 'debug' : 'warn');

type Color = string;

/**
 * A custom scopped customized logger
 */
export interface Logger {
  /**
   * Logs a debug message to console
   */
  // biome-ignore lint/suspicious/noExplicitAny: Due to library
  debug: (...args: any[]) => void;

  /**
   * Logs a info message to console
   */
  // biome-ignore lint/suspicious/noExplicitAny: Due to library
  info: (...args: any[]) => void;

  /**
   * Logs a warning message to console
   */
  // biome-ignore lint/suspicious/noExplicitAny: Due to library
  warn: (...args: any[]) => void;

  /**
   * Logs a error message to console
   */
  // biome-ignore lint/suspicious/noExplicitAny: Due to library
  error: (...args: any[]) => void;
}

/**
 * Creates a custom logger with timestamp, scope and custom color
 */
export function createLogger(scope: string, color: Color = '#ffffff'): Logger {
  const methods = ['debug', 'info', 'warn', 'error'];
  const _color = `color: ${color}`;
  const logger = {};

  for (const level of methods) {
    // biome-ignore lint/suspicious/noExplicitAny: Due to library
    logger[level] = (...args: any[]) => {
      const time = new Date().toISOString().split('T')[1].slice(0, -1); // HH:MM:SS.mmm
      log[level](`[${time}][%c${scope}%c]:`, _color, 'color: inherit', ...args);
    };
  }

  return logger as Logger;
}

export default log;
