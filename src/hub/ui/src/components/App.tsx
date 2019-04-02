import * as React from 'react';
import {HashRouter as Router, Redirect, Route, Switch} from 'react-router-dom';

import {ApolloProvider} from 'react-apollo';

import {InMemoryCache} from 'apollo-cache-inmemory';
import ApolloClient from 'apollo-client';
import {createHttpLink} from 'apollo-link-http';

import {AppBar, CssBaseline, IconButton, Menu, MenuItem, Toolbar, Typography} from '@material-ui/core';
import {Menu as MenuIcon} from '@material-ui/icons';

import {Agent} from './Agent';
import {Agents} from './Agents';
import {GraphiQL} from './GraphiQL';

interface AppProps {
    apiHost: string;
}

interface AppState {
    menuAnchor: HTMLElement | null;
}

export interface AppConfig {
    apiHost: string;
}

const AppConfigContext = React.createContext({});
export const AppConfigConsumer = AppConfigContext.Consumer;

export class App extends React.Component<AppProps, AppState> {
    private client: ApolloClient<any>;

    constructor(props: any) {
        super(props);

        const link = createHttpLink({
            uri: '//' + (this.props.apiHost || window.location.host) + '/graphql',
        });

        this.client = new ApolloClient({
            cache: new InMemoryCache(),
            link,
        });

        this.state = {
            menuAnchor: null,
        };
    }

    render() {
        const config = {
            apiHost: this.props.apiHost,
        };
        return (
            <AppConfigContext.Provider value={config}>
                <ApolloProvider client={this.client}>
                    <Router>
                        <React.Fragment>
                            <CssBaseline />
                            <AppBar
                                position="static"
                            >
                                <Toolbar variant="dense">
                                    <IconButton
                                        aria-haspopup="true"
                                        aria-label="Menu"
                                        aria-owns={this.state.menuAnchor ? 'app-bar-menu' : undefined}
                                        color="inherit"
                                        onClick={(e) => {
                                            this.setState({
                                                menuAnchor: e.currentTarget,
                                            });
                                        }}
                                    >
                                        <MenuIcon />
                                    </IconButton>
                                    <Route
                                        render={({history}) => (
                                            <Menu
                                                id="app-bar-menu"
                                                anchorEl={this.state.menuAnchor}
                                                open={!!this.state.menuAnchor}
                                                onClose={() => {
                                                    this.setState({
                                                        menuAnchor: null,
                                                    });
                                                }}
                                            >
                                                <MenuItem
                                                    onClick={() => {
                                                        history.push('/agents');
                                                        this.setState({
                                                            menuAnchor: null,
                                                        });
                                                    }}
                                                >
                                                    Agents
                                                </MenuItem>
                                                <MenuItem
                                                    onClick={() => {
                                                        history.push('/graphiql');
                                                        this.setState({
                                                            menuAnchor: null,
                                                        });
                                                    }}
                                                >
                                                    GraphiQL
                                                </MenuItem>
                                            </Menu>
                                        )}
                                    />
                                </Toolbar>
                            </AppBar>
                            <Switch>
                                <Route
                                    path="/agents/:id"
                                    component={(props: IdProps) => (
                                        <Agent id={props.match.params.id} />
                                    )}
                                />
                                <Route
                                    path="/agents"
                                    component={() => (
                                        <Agents />
                                    )}
                                />
                                <Route
                                    path="/graphiql"
                                    component={() => (
                                        <GraphiQL />
                                    )}
                                />
                                <Route
                                    component={() => (
                                        <Redirect to="/agents" />
                                    )}
                                />
                            </Switch>
                        </React.Fragment>
                    </Router>
                </ApolloProvider>
            </AppConfigContext.Provider>
        );
    }
}

interface IdProps {
    match: {
        params: {
            id: string;
        };
    };
}
