import { createEffect, createSignal } from 'solid-js';
import { createStore } from 'solid-js/store';
import {
  ClientEventType,
  type Int16,
  type UnsignedInt32,
  type UnsignedInt8,
  sendButtonUpdate,
  sendStickUpdate,
  sendTriggerUpdate,
} from './vkwebsocket';

export interface Sticks {
  leftStickX: Int16;
  leftStickY: Int16;
  rightStickX: Int16;
  rightStickY: Int16;
}

export interface Triggers {
  left: UnsignedInt8;
  right: UnsignedInt8;
}

const [_stateButtons, _setStateButtons] = createSignal<UnsignedInt32>(
  0 as UnsignedInt32,
);

const [stateSticks, _setStateSticks] = createStore<Sticks>({
  leftStickX: 0 as Int16,
  leftStickY: 0 as Int16,
  rightStickX: 0 as Int16,
  rightStickY: 0 as Int16,
});

const [stateTriggers, _setStateTriggers] = createStore<Triggers>({
  left: 0 as UnsignedInt8,
  right: 0 as UnsignedInt8,
});

export function ControllerStateManager() {
  createEffect(() => {
    const buttons = _stateButtons();
    sendButtonUpdate(buttons);
  });

  createEffect(() => {
    const x = stateSticks.leftStickX;
    const y = stateSticks.leftStickY;
    sendStickUpdate(ClientEventType.stickLeft, x, y);
  });

  createEffect(() => {
    const x = stateSticks.rightStickX;
    const y = stateSticks.rightStickY;
    sendStickUpdate(ClientEventType.stickRight, x, y);
  });

  createEffect(() => {
    const left = stateTriggers.left;
    const right = stateTriggers.right;
    sendTriggerUpdate(left, right);
  });

  return null;
}

export const stateButtons = _stateButtons;
export const setStateButtons = _setStateButtons;
export const setStateSticks = _setStateSticks;
export const setStateTriggers = _setStateTriggers;
