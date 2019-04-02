import * as React from 'react';

import * as Explorer from 'graphiql';

import {ApolloConsumer} from 'react-apollo';

import ApolloClient from 'apollo-client';

import '../../node_modules/graphiql/graphiql.css';

import {createStyles, Paper, Theme, withStyles, WithStyles} from '@material-ui/core';

import {parse} from 'graphql';

const selectedOperation = (queryDoc: any, operationName: string) => {
    let ret;
    for (const definition of queryDoc.definitions) {
        if (definition.kind === 'OperationDefinition') {
            if (!ret || (definition.name && definition.name.value === operationName)) {
                ret = definition.operation;
            }
        }
    }
    return ret;
};

const styles = (theme: Theme) => createStyles({
    paper: {
        height: 'calc(100vh - 100px)',
        padding: 0,
    },
    root: {
        padding: '16px',
    },
});

export const GraphiQL = withStyles(styles)(
    class extends React.Component<WithStyles<typeof styles>, {}> {
        constructor(props: any) {
            super(props);

            this.fetcher = this.fetcher.bind(this);
        }

        fetcher(client: ApolloClient<any>) {
            return ({operationName, query, variables}: any) => {
                const parsed = parse(query);

                if (selectedOperation(parsed, operationName) === 'subscription') {
                    return client.subscribe({
                        fetchPolicy: 'no-cache',
                        query: parsed,
                        variables,
                    });
                }

                if (selectedOperation(parsed, operationName) === 'mutation') {
                    return client.mutate({
                        errorPolicy: 'all',
                        fetchPolicy: 'no-cache',
                        mutation: parsed,
                        variables,
                    }).then((result) => ({
                        data: result.data || null,
                        errors: result.errors || undefined,
                    }));
                }

                return client.query({
                    errorPolicy: 'all',
                    fetchPolicy: 'no-cache',
                    query: parsed,
                    variables,
                }).then((result) => ({
                    data: result.data || null,
                    errors: result.errors || undefined,
                }));
            };
        }

        render() {
            return (
                <div className={this.props.classes.root}>
                    <Paper className={this.props.classes.paper}>
                        <ApolloConsumer>
                            {(client) => (
                                <Explorer
                                    fetcher={this.fetcher(client)}
                                />
                            )}
                        </ApolloConsumer>
                    </Paper>
                </div>
            );
        }
    },
);
