import './direction8.css';

import clsx from 'clsx';
import { createEffect, createSignal } from 'solid-js';
import { createStore } from 'solid-js/store';
import { createLogger } from '../utils/logging';

const log = createLogger('Direction8');

export interface ButtonImages {
  left: string;
  right: string;
  up: string;
  down: string;
}

export const BUTTON_IMAGES: Record<'dpad' | 'actions', ButtonImages> = {
  dpad: {
    left: '/hollow_circle.png',
    right: '/hollow_circle.png',
    up: '/hollow_circle.png',
    down: '/hollow_circle.png',
  },
  actions: {
    left: '/hollow_circle.png',
    right: '/hollow_circle.png',
    up: '/hollow_circle.png',
    down: '/hollow_circle.png',
  },
} as const;

interface Direction8ImageProps {
  image: string;
  class: string;
  width: number;
  height: number;
  ref: (element: HTMLImageElement) => void;
}

// A container that is used to display the direction8 button
function Direction8Image(props: Direction8ImageProps) {
  return (
    <img
      alt={props.image}
      src={props.image}
      draggable={false}
      class={clsx('d8-button', 'no-interact', props.class)}
      width={props.width}
      height={props.height}
      ref={props.ref}
    />
  );
}

export interface Direction8State {
  left: boolean;
  right: boolean;
  up: boolean;
  down: boolean;
}

function direction8Zero(): Direction8State {
  return {
    left: false,
    right: false,
    up: false,
    down: false,
  };
}

export interface Direction8Props {
  width: number;
  height: number;
  centerGap: number;
  onUpdate: (pressing: Direction8State) => void;
  images: ButtonImages;

  /** If passed, then it will use this value for the width and height*/
  squareSize?: number;

  class?: string;
}

export function Direction8(props: Direction8Props) {
  const [pressing, updatePressing] = createSignal(false);
  const [buttons, updateButtons] = createStore<Direction8State>(
    direction8Zero(),
  );

  let refArrowLeft: HTMLImageElement | null = null;
  let refArrowRight: HTMLImageElement | null = null;
  let refArrowUp: HTMLImageElement | null = null;
  let refArrowDown: HTMLImageElement | null = null;

  createEffect(() => {
    const currentState: Direction8State = {
      left: buttons.left,
      up: buttons.up,
      down: buttons.down,
      right: buttons.right,
    };
    props.onUpdate(currentState);

    refArrowLeft?.classList.toggle('active', currentState.left);
    refArrowRight?.classList.toggle('active', currentState.right);
    refArrowDown?.classList.toggle('active', currentState.down);
    refArrowUp?.classList.toggle('active', currentState.up);
    log.debug('New state: ', currentState);
  });

  // Calculation Constants
  const midpointLow = 0.5 - props.centerGap / 2;
  const midpointHigh = 0.5 + props.centerGap / 2;
  log.debug(
    `Calculated class='${props.class}' has low=${midpointLow} high=${midpointHigh}`,
  );

  function calcDirection8State(
    offsetX: number,
    offsetY: number,
  ): Direction8State {
    const relX = offsetX / props.width;
    const relY = offsetY / props.height;

    const result = {
      left: relX <= midpointLow,
      right: relX >= midpointHigh,
      up: relY <= midpointLow,
      down: relY >= midpointHigh,
    };

    log.debug(`Pressing at x=${relX} y=${relY} resulted to:`, result);

    return result;
  }

  function onPress(event: PointerEvent) {
    event.preventDefault();
    if (event.target !== event.currentTarget) {
      return;
    }

    updatePressing(true);
    updateButtons(calcDirection8State(event.offsetX, event.offsetY));
  }

  function onMove(event: PointerEvent) {
    event.preventDefault();
    if (event.target !== event.currentTarget || pressing() !== true) {
      return;
    }

    updatePressing(true);
    updateButtons(calcDirection8State(event.offsetX, event.offsetY));
  }

  function onRelease(event: PointerEvent) {
    event.preventDefault();
    updatePressing(false);
    updateButtons(direction8Zero());
  }

  return (
    <button
      type='button'
      style={{
        '--d8-width': `${props.width}px`,
        '--d8-height': `${props.height}px`,
      }}
      class={clsx('direction8', 'touch-fix', props.class)}
      onpointerdown={onPress}
      onpointermove={onMove}
      onpointerup={onRelease}
      onpointerleave={onRelease}
      onpointercancel={onRelease}
    >
      <Direction8Image
        image={props.images.left}
        width={props.squareSize ?? props.width / 2}
        height={props.squareSize ?? props.height / 3}
        class='left'
        ref={(el: HTMLImageElement) => {
          refArrowLeft = el;
        }}
      />
      <Direction8Image
        image={props.images.right}
        width={props.squareSize ?? props.width / 2}
        height={props.squareSize ?? props.height / 3}
        class='right'
        ref={(el: HTMLImageElement) => {
          refArrowRight = el;
        }}
      />
      <Direction8Image
        image={props.images.up}
        width={props.squareSize ?? props.width / 3}
        height={props.squareSize ?? props.height / 2}
        class='up'
        ref={(el: HTMLImageElement) => {
          refArrowUp = el;
        }}
      />
      <Direction8Image
        image={props.images.down}
        width={props.squareSize ?? props.width / 3}
        height={props.squareSize ?? props.height / 2}
        class='down'
        ref={(el: HTMLImageElement) => {
          refArrowDown = el;
        }}
      />
    </button>
  );
}
