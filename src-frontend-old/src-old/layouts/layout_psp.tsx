import { createMemo, useContext } from 'solid-js';
import { BUTTON_IMAGES, Direction8 } from '../button/direction8';
import { ControllerContext } from '../global_context';
import { setStateButtons } from '../state_manager';
import { toggleBit } from '../utils/bit_manipulation';
import { ButtonId } from '../vkwebsocket';

export function LayoutDefault() {
  const controllerContext = useContext(ControllerContext);

  const pingDisplay = createMemo(() => {
    const status = controllerContext.connectionStatus[0]();
    const ping = controllerContext.ping[0]();
    return status === 'Connected' ? `[${Math.round(ping)}ms]` : '';
  });

  return (
    <main class='layout-psp'>
      <Direction8
        width={256}
        height={256}
        centerGap={1 / 3}
        images={BUTTON_IMAGES.dpad}
        class='dpad'
        onUpdate={(buttons) => {
          setStateButtons((result) => {
            result = toggleBit(result, ButtonId.dpadLeft, buttons.left);
            result = toggleBit(result, ButtonId.dpadRight, buttons.right);
            result = toggleBit(result, ButtonId.dpadDown, buttons.down);
            result = toggleBit(result, ButtonId.dpadUp, buttons.up);
            return result;
          });
        }}
      />
      <Direction8
        images={BUTTON_IMAGES.actions}
        class='action-buttons'
        width={256}
        height={256}
        squareSize={96}
        centerGap={0.2}
        onUpdate={(buttons) => {
          setStateButtons((result) => {
            result = toggleBit(result, ButtonId.buttonWest, buttons.left);
            result = toggleBit(result, ButtonId.buttonEast, buttons.right);
            result = toggleBit(result, ButtonId.buttonNorth, buttons.up);
            result = toggleBit(result, ButtonId.buttonSouth, buttons.down);
            return result;
          });
        }}
      />
      <div class='info'>
        <span>
          {controllerContext.connectionStatus[0]()} {pingDisplay()}
        </span>
      </div>
    </main>
  );
}
