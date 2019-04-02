import * as React from 'react';

import '../../../node_modules/xterm/dist/xterm.css';

import {Terminal} from 'xterm';
import * as attach from 'xterm/lib/addons/attach/attach';
import * as fit from 'xterm/lib/addons/fit/fit';

Terminal.applyAddon(attach);
Terminal.applyAddon(fit);

import {createStyles, Paper, Theme, Typography, withStyles, WithStyles} from '@material-ui/core';

const styles = (theme: Theme) => createStyles({
    paper: {
        padding: 0,
    },
    root: {
        padding: '16px',
    },
    terminal: {
        backgroundColor: 'black',
        height: 'calc(100vh - 100px)',
        padding: '16px',
    },
});

interface ShellProps extends WithStyles<typeof styles> {
    apiHost: string;
    id: string;
}

export const Shell = withStyles(styles)(
    class extends React.Component<ShellProps, {}> {
        private terminal: Terminal | null;
        private terminalContainer: HTMLElement | null;

        constructor(props: any) {
            super(props);

            this.terminal = null;
            this.terminalContainer = null;
        }

        componentDidMount() {
            if (!this.terminalContainer) {
                return;
            }

            this.terminal = new Terminal({
                convertEol: true,
            });

            let l = window.location;
            let wsURL = ((l.protocol === 'https:') ? 'wss://' : 'ws://') + (this.props.apiHost || l.host) + '/shell?agent=' + this.props.id;
            (this.terminal as any).attach(new WebSocket(wsURL));

            this.terminal.open(this.terminalContainer);

            this.resize();
            window.addEventListener('resize', this.resize.bind(this));
        }

        componentWillUnmount() {
            window.removeEventListener('resize', this.resize.bind(this));
        }

        resize() {
            if (this.terminal) {
                (this.terminal as any).fit();
            }
        }

        render() {
            return (
                <div className={this.props.classes.root}>
                    <Paper className={this.props.classes.paper}>
                        <div
                            className={this.props.classes.terminal}
                            ref={ref => {
                                this.terminalContainer = ref;
                            }}
                        />
                    </Paper>
                </div>
            );
        }
    },
);
