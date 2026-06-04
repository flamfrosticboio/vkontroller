/**
 * @module WebsocketModule
 * @description This module handles the websocket implementation of the
 * `vkcontroller` specifications. This module requires reconnecting-websocket
 * for reconnection capabilities.
 *
 * It is possible to attach listeners and set
 * up options by overwriting the contents of these exported variables:
 * - Listener
 * - Options
 *
 * * @example
 * ```ts
 * import {
 *   Listener,
 *   connect,
 *   sendTriggerUpdate,
 *   type UnsignedInt8
 * } from "./vkwebsocket.ts";
 * Listener.onRumbleEvent = (doRumble: boolean) => {
 *   console.log(`Rumble is ${rumble}`);
 * }
 * connect("localhost:8000");
 * sendTriggerUpdate(255 as UnsignedInt8, 0 as UnsignedInt8);
 * ```
 *
 * Note: Leaving the debug logger on production is costly but its used to
 * debug the messages. Removing them manually on this script or using tools
 * like @rollup/plugin-strip inside your vite plugin with patterns:
 * "Option.log.debug" and "Option.log.info" can effectively bypass the logging
 * overhead
 */

import ReconnectingWebSocket from 'reconnecting-websocket';

// =============================== Types ======================================
export type ControllerStatus = 'closed' | 'not_assigned' | 'open' | 'error';
export type Int16 = number & { readonly __brand: 'Int16' };
export type UnsignedInt32 = number & { readonly __brand: 'UInt32' };
export type UnsignedInt8 = number & { readonly __brand: 'UInt8' };

// Server side events
// Compiled into a type: u8, value: any size
export const ServerEventType = {
  // these can be on each enum since they are small
  rumbleOn: 0x01,
  rumbleOff: 0x02,

  // exists if size=5
  ping: 0x00,
  player: 0x01,
} as const;

export type ServerEventType =
  (typeof ServerEventType)[keyof typeof ServerEventType];

export const ClientEventType = {
  buttons: 0x01,
  triggers: 0x02,
  stickLeft: 0x03,
  stickRight: 0x04,
} as const;

export type ClientEventType =
  (typeof ClientEventType)[keyof typeof ClientEventType];

// The buttons here are emulating a xinput (xbox controller)

// Buttons are sent as one byte input (32 bits)
// This is the mapped to a 32 bit integer
export const ButtonId = {
  dpadUp: 1 << 0,
  dpadDown: 1 << 1,
  dpadLeft: 1 << 2,
  dpadRight: 1 << 3,

  buttonWest: 1 << 4,
  buttonNorth: 1 << 5,
  buttonSouth: 1 << 6,
  buttonEast: 1 << 7,

  buttonLeftBumper: 1 << 8,
  buttonRightBumper: 1 << 9,
  buttonLeftStick: 1 << 10,
  buttonRightStick: 1 << 11,

  buttonStart: 1 << 12,
  buttonSelect: 1 << 13,
  buttonGuide: 1 << 14,
} as const;

export type ButtonId = (typeof ButtonId)[keyof typeof ButtonId];

// This will be referred for values that go 0.0 to 1.0
// It will be translated as 8 bit value in the transmission
export const TriggerId = {
  leftTrigger: 0x01,
  rightTrigger: 0x02,
} as const;

export type TriggerId = (typeof TriggerId)[keyof typeof TriggerId];

export const StickId = {
  leftStick: 0x01,
  rightStick: 0x02,
} as const;

export type StickId = (typeof StickId)[keyof typeof StickId];

// ============================================================================

// ================== Websocket Url Fetcher Mechanism =========================
let url = '';
const connectUrlHandler = () => url;
// ============================================================================

const connection: ReconnectingWebSocket = new ReconnectingWebSocket(
  connectUrlHandler,
);

// ============================================================================
// Required since this uses an array buffer instead of typical json
connection.binaryType = 'arraybuffer';
// ============================================================================

// ==================== Exported, modifiable variables ========================
export interface Listener {
  onPlayerChange: (playerId: number) => void;
  onDisconnect: (error?: Error) => void;
  onConnect: () => void;
  onRumbleEvent: (doRumble: boolean) => void;
  onLatencyReceive: (latency: number) => void;
}

export const Listener: Listener = {
  onPlayerChange: () => {},
  onDisconnect: () => {},
  onConnect: () => {},
  onRumbleEvent: () => {},
  onLatencyReceive: () => {},
};

export interface Options {
  /**
   * A logger object that has info, debug, warn, error functions
   */
  log: {
    // biome-ignore lint/suspicious/noExplicitAny: logging can take any value
    debug: (...args: any[]) => void;
    // biome-ignore lint/suspicious/noExplicitAny: logging can take any value
    info: (...args: any[]) => void;
    // biome-ignore lint/suspicious/noExplicitAny: logging can take any value
    warn: (...args: any[]) => void;
    // biome-ignore lint/suspicious/noExplicitAny: logging can take any value
    error: (...args: any[]) => void;
  };
}

export const Options: Options = {
  log: {
    // biome-ignore lint/suspicious/noExplicitAny: logging can take any value
    debug: (...args: any[]) => {
      console.debug(...args);
    },
    // biome-ignore lint/suspicious/noExplicitAny: logging can take any value
    info: (...args: any[]) => {
      console.info(...args);
    },
    // biome-ignore lint/suspicious/noExplicitAny: logging can take any value
    warn: (...args: any[]) => {
      console.warn(...args);
    },
    // biome-ignore lint/suspicious/noExplicitAny: logging can take any value
    error: (...args: any[]) => {
      console.error(...args);
    },
  },
};
// ============================================================================

// ============================= Ping Mechanism ===============================
// Note: There is no receiver mechanism in this section. It is handled on the
// receiver section of this script.

let pingIntervalId: number | null = null;
const pingInterval = 1000; // in milliseconds
const pingBuffer = new ArrayBuffer(4);
const pingView = new Uint32Array(pingBuffer);

/**
 * Stops the ping interval.
 * Note: This already handles if the interval exists or not
 */
function stopPingInterval() {
  if (pingInterval !== null) {
    window.clearInterval(pingIntervalId);
    pingIntervalId = null;
  }
}

/**
 * Starts the ping interval mechanism.
 */
function startPingInterval() {
  stopPingInterval();
  pingIntervalId = window.setInterval(() => {
    const now = performance.now();
    pingView[0] = now;
    connection.send(pingBuffer);
  }, pingInterval);
}
// ============================================================================

// ====================== Attaching Event Listeners ===========================
connection.addEventListener('message', (event) => {
  Options.log.debug('Received message from server:', event.data);

  if (!(event.data instanceof ArrayBuffer)) {
    Options.log.error(
      `Received a text message rather than a buffer. Message: ${event.data}`,
    );
    return;
  }

  if (event.data.byteLength <= 0) {
    Options.log.error('There are no items received');
    return;
  }

  const messageBuffer = event.data;
  const typeView = new Uint8Array(messageBuffer);

  switch (event.data.byteLength) {
    // This is the enum specific case where there are no other data rather
    // than just an enum
    case 1: {
      switch (typeView[0]) {
        case ServerEventType.rumbleOn:
          Listener.onRumbleEvent(true);
          break;
        case ServerEventType.rumbleOff:
          Listener.onRumbleEvent(false);
          break;
      }
      break;
    }

    // currently in the implementation, there is only one case (player change)
    // if the length is 5 the type exists on the last part. I'm gonna optimize
    // this and comment off the original implementation
    //
    // case 5: {
    //   const type = new Uint8Array(messageBuffer, 4, 1);
    //   // Warning: May return wrong endianness if the client uses different
    //   // endian system than the server
    //   const valueView = new Uint32Array(messageBuffer, 0, 1);
    //   const value = valueView[0];
    //   console.log('5', type, value);
    //   switch (type[0]) {
    //     case ServerEventType.player: {
    //       Listener.onPlayerChange(value);
    //     }
    //   }
    //   break;
    // }
    //
    // Optimized implementation (player change)
    case 5: {
      // Warning: May return wrong endianness if the client uses different
      // endian system than the server
      const valueView = new Uint32Array(messageBuffer, 0, 1);
      Listener.onPlayerChange(valueView[0]);
      break;
    }

    // this is specific to ping only since the server reflects the 4 bytes
    // sent from client
    case 4: {
      const now = performance.now();
      // There is no guarantee that this will return as a big-endian and mess
      // up the thing, however, the server just reflects anything,
      // so this is TOTALLY OK
      const valueView = new Uint32Array(messageBuffer, 0, 1);
      const value = valueView[0];
      const elapsed = now - value;
      Listener.onLatencyReceive(elapsed);
      break;
    }

    default: {
      Options.log.error('Received excess/little data');
    }
  }
});

connection.addEventListener('open', () => {
  startPingInterval();
  Listener.onConnect();
});

connection.addEventListener('close', () => {
  stopPingInterval();
  Listener.onDisconnect();
});

connection.addEventListener('error', (event) => {
  stopPingInterval();
  Listener.onDisconnect(event.error);
});

// ========================= Exported functions ===============================

/**
 * Connect to an url
 */
export function connect(path: string) {
  Options.log.debug(`Setting connection to ${path}`);
  url = path;
  reconnect();
}

/**
 * Disconnect the existing websocket
 */
export function disconnect() {
  connection.close();
  Options.log.info('Closing existing connection...');
}

export function reconnect() {
  connection.reconnect();
  Options.log.info(`Connecting to '${url}'...`);
}

export function getUrl() {
  return url;
}

// reusing the same buffer to prevent multiple creations of buffer
// on the hot path
const buffer = new ArrayBuffer(5);

// Adding multiple views to buffer
const typeView = new Uint8Array(buffer, 4, 1);
const stickView = new Int16Array(buffer, 0, 2);
const triggerView = new Uint8Array(buffer, 0, 2);
const buttonView = new Uint32Array(buffer, 0, 1);

/**
 * Send stick events in Int16 (-32768 - 32767)
 * @param eventType - Integer ID
 * @param x - Value in the x-coordinate
 * @param y - Value in the y-coordinate
 */
export function sendStickUpdate(
  eventType:
    | typeof ClientEventType.stickLeft
    | typeof ClientEventType.stickRight,
  x: Int16,
  y: Int16,
) {
  stickView[0] = x;
  stickView[1] = y;
  typeView[0] = eventType;

  // Can be removed by manually removing or using using vite with
  // @rollup/plugin-strip to remove automatically on production
  Options.log.debug(
    `Sending stick update for ${eventType === ClientEventType.stickLeft ? 'left' : 'right'} with values: x=${x} y=${y}`,
  );
  connection.send(buffer);
}

/**
 * Send new state of the buttons (1 for pressing, 0 for released)
 * @param button_map - Bitmap of the buttons pressed with an layout
 *
 * Button Layout:
 * [1] - dpad up
 * [2] - dpad down
 * [3] - dpad left
 * [4] - dpad right
 * [5] - west (x)
 * [6] - north (y)
 * [7] - south (a)
 * [8] - east (b)
 * [9] - left bumper
 * [10] - right bumper
 * [11] - left stick button
 * [12] - right stick button
 * [13] - start
 * [14] - select
 * [15] - guide
 */
export function sendButtonUpdate(buttonMap: UnsignedInt32) {
  buttonView[0] = buttonMap;
  typeView[0] = ClientEventType.buttons;

  // Can be removed by manually removing or using using vite with
  // @rollup/plugin-strip to remove automatically on production
  Options.log.debug(`Sending button update with values: ${buttonMap}`);
  connection.send(buffer);
}

/**
 * Send a new state of the triggers in UnsignedInt8 (0-255)
 * @param left - Value for left trigger
 * @param right - Value for right trigger
 */
export function sendTriggerUpdate(left: UnsignedInt8, right: UnsignedInt8) {
  triggerView[0] = left;
  triggerView[1] = right;
  typeView[0] = ClientEventType.triggers;

  // Can be removed by manually removing or using using vite with
  // @rollup/plugin-strip to remove automatically on production
  Options.log.debug(
    `Sending trigger update with values: left=${left} right=${right}`,
  );
  connection.send(buffer);
}
