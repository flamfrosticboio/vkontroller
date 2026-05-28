import './button.css';

import { clsx } from 'clsx';
import type { Setter } from 'solid-js';
import { toggleBit } from '../utils/bit_manipulation';
import { createLogger } from '../utils/logging';
import type { ButtonId, UnsignedInt32 } from '../vkwebsocket';

const log = createLogger('Button');

export interface ControllerButtonProps {
  id: ButtonId;
  imagePath: string;
  width: number;
  height: number;
  class?: string;
  setter: Setter<UnsignedInt32>;
}

export function ControllerButton(props: ControllerButtonProps) {
  let refImage: HTMLImageElement | null = null;

  function set(toggle: boolean) {
    props.setter((result) => toggleBit(result, props.id, toggle));
    refImage?.classList.toggle('active', toggle);
    log.debug(`${props.id}: ${toggle}`);
  }

  return (
    <button
      type='button'
      class={clsx('controller_button', 'no-interact', props.class)}
      style={{
        '--button-width': `${props.width}px`,
        '--button-height': `${props.height}px`,
      }}
      onpointerdown={(e) => {
        e.preventDefault();
        set(true);
      }}
      onpointerup={(e) => {
        e.preventDefault();
        set(false);
      }}
      onpointerleave={(e) => {
        e.preventDefault();
        set(false);
      }}
      onpointercancel={(e) => {
        e.preventDefault();
        set(false);
      }}
    >
      {/* We can definitely make the image smaller while the hitbox is bigger */}
      <img
        class={clsx('no-interact')}
        draggable={false}
        src={props.imagePath}
        width={props.width}
        height={props.height}
        alt={`controller-button-${props.id}`}
        ref={(el) => {
          refImage = el;
        }}
      />
      <span>{props.id}</span>
    </button>
  );
}
