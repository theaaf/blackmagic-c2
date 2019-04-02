use common;
use super::agent_connection;
use super::SharedState;

use std::sync::Arc;

use futures::{future, Future};
use actix::{MailboxError};
use juniper::{FieldError, FieldResult};

pub struct Context {
    pub shared_state: Arc<SharedState>,
}

impl juniper::Context for Context {}

pub struct Query;

graphql_object!(Query: Context |&self| {
    field agent(&executor, id: String) -> Option<super::Agent> {
        let agents = executor.context().shared_state.agents.read().unwrap();
        match agents.get(&id) {
            Some(agent) => Some(agent.clone()),
            None => None,
        }
    }

    field agents(&executor) -> Vec<super::Agent> {
        let agents = executor.context().shared_state.agents.read().unwrap();
        agents.values().map(|v| v.clone()).collect()
    }
});

pub struct Mutation;

graphql_object!(Mutation: Context |&self| {
    field hyperdeck_command(&executor, agent_id: String, ip_address: String, command: String) -> FieldResult<common::HyperDeckCommandResponse> {
        {
            let agents = executor.context().shared_state.agents.read().unwrap();
            let f: Box<Future<Item=common::HyperDeckCommandResponse, Error=FieldError>> = match agents.get(&agent_id) {
                Some(agent) => Box::new({
                    agent.addr.send(agent_connection::HyperDeckCommand{
                        command: command,
                        ip_address: ip_address,
                    })
                        .map_err(|e| {
                            match e {
                                MailboxError::Closed => FieldError::from("The agent with that is has disconnected."),
                                MailboxError::Timeout => FieldError::from("Timed out sending command to agent."),
                            }
                        })
                        .and_then(|f| f.map_err(|e| FieldError::from(e)))
                }),
                None => Box::new(future::err(FieldError::from("The agent with that id is not connected."))),
            };
            f
        }.wait()
    }
});

pub type Schema = juniper::RootNode<'static, Query, Mutation>;

pub fn create_schema() -> Schema {
    Schema::new(Query{}, Mutation{})
}
