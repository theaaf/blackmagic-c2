import * as React from 'react';
import {Route, Switch} from 'react-router-dom';

import {AppConfig, AppConfigConsumer} from '../App';

import {Overview} from './Overview';
import {Shell} from './Shell';

interface AgentProps {
    id: string;
}

export class Agent extends React.Component<AgentProps, {}> {
    render() {
        return (
            <AppConfigConsumer>
                {(config: AppConfig) => (
                    <Switch>
                        <Route
                            path="/agents/:id/shell"
                            component={() => (
                                <Shell apiHost={config.apiHost} id={this.props.id} />
                            )}
                        />
                        <Route
                            component={() => (
                                <Overview id={this.props.id} />
                            )}
                        />
                    </Switch>
                )}
            </AppConfigConsumer>
        );
    }
};
