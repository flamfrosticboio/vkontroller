/* @refresh reload */

import './index.css';
import './utils.css';
import './button/dpad.css';
import './button/action_buttons.css';
import './layouts/layout_psp.css';

import type { Logger } from 'loglevel';
import { Show, onCleanup, onMount, useContext } from 'solid-js';
import { render } from 'solid-js/web';
import {
  ControllerContext,
  ControllerContextProvider,
} from './global_context.tsx';
import { LayoutDefault } from './layouts/layout_psp.tsx';
import { Menu } from './layouts/menu.tsx';
import { RumbleExtension } from './rumble.ts';
import { ControllerStateManager } from './state_manager.tsx';
import { createLogger } from './utils/logging.ts';
import { Options, connect, disconnect } from './vkwebsocket.ts';

const root = document.getElementById('root');
const log = createLogger('main', '#ff0');

function defaultConnect() {
  const wsProtocol = window.location.protocol === 'https:' ? 'wss://' : 'ws://';
  const wsUrl = `${wsProtocol}${window.location.host}/ws`;
  connect(wsUrl);
}

function App() {
  const controllerContext = useContext(ControllerContext);

  onMount(() => {
    // Set logger
    Options.log = createLogger('websocket', '#0ff') as Logger;
    defaultConnect();
  });

  onCleanup(() => {
    disconnect();
  });

  return (
    <>
      <LayoutDefault />
      <button
        type='button'
        id='toggle-menu'
        onpointerdown={() => {
          controllerContext.menu[1]((prev) => {
            log.debug(`${!prev ? 'Opening' : 'Closing'} menu`);
            // simple toggle
            return !prev;
          });
        }}
      >
        Toggle Menu
      </button>
      <Show when={controllerContext.menu[0]()}>
        <Menu />
      </Show>
      <RumbleExtension></RumbleExtension>
      <ControllerStateManager></ControllerStateManager>
    </>
  );
}

function Root() {
  return (
    <ControllerContextProvider>
      <App />
    </ControllerContextProvider>
  );
}

if (root) {
  render(() => <Root />, root);
}
