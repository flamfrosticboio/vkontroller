import { createSignal, onCleanup, onMount } from 'solid-js';
import { createLogger } from './utils/logging.ts';
import { Listener } from './vkwebsocket.ts';

const log = createLogger('Rumble', '#f00');

// measured in milliseconds
const RUMBLE_INTERVAL = 100;

const [rumblingIntervalId, setRumblingIntervalId] = createSignal<number>(-1);

function rumble() {
  navigator.vibrate(RUMBLE_INTERVAL);
  log.debug('Rumbling');
}

function stopRumbling() {
  if (isRumbling()) {
    window.clearInterval(rumblingIntervalId());
    navigator.vibrate(0);
    setRumblingIntervalId(-1);
    log.debug('Rumbling has stopped');
  }
}

function onRumble(toggle: boolean) {
  if (!hasRumble()) {
    return;
  }

  if (toggle) {
    // try to do initial rumble
    if (isRumbling()) return;
    rumble();
    setRumblingIntervalId(setInterval(rumble, RUMBLE_INTERVAL));
  } else {
    stopRumbling();
  }
}

export function hasRumble(): boolean {
  return 'vibrate' in navigator;
}

function isRumbling(): boolean {
  return rumblingIntervalId() > 0;
}

/**
 * Creates rumble capabilities as an component
 * This component will do nothing if it has no vibration plugin
 */
export function RumbleExtension() {
  // Just stop completely if the environment has no vibrator
  if (!hasRumble()) return null;

  onMount(() => {
    Listener.onRumbleEvent = onRumble;
    log.info('Rumble extension has been attached');
  });

  onCleanup(() => {
    stopRumbling();
    log.info('Rumble extension disconnected');
  });

  return null;
}
