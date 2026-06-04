import {
  type Accessor,
  type JSXElement,
  type Setter,
  createContext,
  createSignal,
  onCleanup,
  onMount,
} from 'solid-js';
import { Listener } from './vkwebsocket';

type ConnectionStatus = 'Connected' | 'Disconnected' | 'Error';

interface ControllerContextMap {
  ping: [Accessor<number>, Setter<number>];
  menu: [Accessor<boolean>, Setter<boolean>];
  connectionStatus: [Accessor<ConnectionStatus>, Setter<ConnectionStatus>];
  player: [Accessor<number>, Setter<number>];
}

const ControllerContext = createContext<ControllerContextMap>();

interface ControllerContextProviderProps {
  children: JSXElement;
}

function ControllerContextProvider(props: ControllerContextProviderProps) {
  const ping = createSignal(0);
  const menu = createSignal(false);
  const connectionStatus = createSignal<ConnectionStatus>('Disconnected');
  const player = createSignal(0);

  onMount(() => {
    Listener.onConnect = () => connectionStatus[1]('Connected');
    Listener.onDisconnect = (error) =>
      connectionStatus[1](error ? 'Error' : 'Disconnected');
    Listener.onPlayerChange = (playerNumber) => player[1](playerNumber);
    Listener.onLatencyReceive = (latency) => ping[1](latency);
  });

  onCleanup(() => {
    Listener.onConnect = () => {};
    Listener.onDisconnect = () => {};
    Listener.onPlayerChange = () => {};
    Listener.onLatencyReceive = () => {};
  });

  return (
    <ControllerContext.Provider
      value={{
        ping: ping,
        menu: menu,
        connectionStatus: connectionStatus,
        player: player,
      }}
    >
      {props.children}
    </ControllerContext.Provider>
  );
}

export {
  type ConnectionStatus,
  ControllerContext,
  type ControllerContextMap,
  ControllerContextProvider,
  type ControllerContextProviderProps,
};
