import * as React from 'react';
import * as ReactDOM from 'react-dom';

import {App} from './components/App';

declare global {
  interface Window {
    InitWebApp: any;
  }
}

window.InitWebApp = (config: {
    apiHost: string,
    container: Element,
}) => {
    ReactDOM.render((
        <App
            apiHost={config.apiHost}
        />
    ), config.container);
};
