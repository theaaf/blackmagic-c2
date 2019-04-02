import * as React from 'react';
import {Route} from 'react-router-dom';

import {Query} from 'react-apollo';

import {AgentsQuery as AgentsQueryData} from '../__generated__/graphql';
class AgentsQuery extends Query<AgentsQueryData> {}

import gql from 'graphql-tag';

import {createStyles, Paper, Table, TableBody, TableCell, TableHead, TableRow, Theme, Toolbar, Typography, withStyles, WithStyles} from '@material-ui/core';

const styles = (theme: Theme) => createStyles({
    paper: {
        padding: 0,
    },
    root: {
        padding: '16px',
    },
    row: {
        cursor: 'pointer',
    },
});

export const Agents = withStyles(styles)(
    class extends React.Component<WithStyles<typeof styles>, {}> {
        render() {
            return (
                <div className={this.props.classes.root}>
                    <Paper className={this.props.classes.paper}>
                        <AgentsQuery
                            pollInterval={1000}
                            query={gql`
                                query Agents {
                                    agents {
                                        id
                                        remote
                                        state {
                                            decklinkDevices {
                                                modelName
                                            }
                                        }
                                    }
                                }
                            `}
                        >
                            {({loading, error, data}) => {
                                if (loading) {
                                    return <Typography>Loading...</Typography>;
                                } else if (error) {
                                    alert(error);
                                    return null;
                                }
                                return data && (
                                    <React.Fragment>
                                        <Toolbar>
                                            <Typography variant="h6">Agents</Typography>
                                        </Toolbar>
                                        <Table>
                                            <TableHead>
                                                <TableRow>
                                                    <TableCell>Id</TableCell>
                                                    <TableCell>Remote</TableCell>
                                                    <TableCell>Decklink Devices</TableCell>
                                                </TableRow>
                                            </TableHead>
                                            <Route
                                                render={({history}) => (
                                                    <TableBody>
                                                        {data.agents.map((agent) => (
                                                            <TableRow
                                                                className={this.props.classes.row}
                                                                hover={true}
                                                                key={agent.id}
                                                                onClick={() => {
                                                                    history.push('/agents/' + agent.id);
                                                                }}
                                                            >
                                                                <TableCell>{agent.id}</TableCell>
                                                                <TableCell>{agent.remote}</TableCell>
                                                                <TableCell>{agent.state.decklinkDevices.length}</TableCell>
                                                            </TableRow>
                                                        ))}
                                                    </TableBody>
                                                )}
                                            />
                                        </Table>
                                    </React.Fragment>
                                );
                            }}
                        </AgentsQuery>
                    </Paper>
                </div>
            );
        }
    },
);
