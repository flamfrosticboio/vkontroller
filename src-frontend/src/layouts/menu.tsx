import { useContext } from 'solid-js';
import { ControllerContext } from '../global_context';
import { connect, getUrl } from '../vkwebsocket';

export function Menu() {
  const controllerContext = useContext(ControllerContext);
  return (
    <div id='menu'>
      <p>Player: {controllerContext.player[0]() ?? 'Not assigned'} </p>
      <p>
        Connection Status: {controllerContext.connectionStatus[0]()}{' '}
        {controllerContext.ping[0]()}
      </p>
      <label for='debug-url'>Debug Url Connect: </label>
      <input
        id='debug-url'
        type='url'
        value={getUrl()}
        onfocusout={(event) => {
          event.preventDefault();
          connect(event.currentTarget.value);
        }}
      />
      <button
        type='button'
        onpointerdown={() => {
          if (!document.fullscreenElement) {
            document.documentElement.requestFullscreen().catch((error) => {
              console.error('Could not go fullscreen', error);
            });
          } else {
            document.exitFullscreen();
          }
        }}
      >
        Toggle Fullscreen
      </button>
    </div>
  );
}
