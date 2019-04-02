use common;
use super::shell_connection;

use std::sync::{Arc, Mutex, Weak};
use std::time::{Instant, Duration};

use actix::{Actor, ActorContext, AsyncContext, Handler, ResponseFuture, StreamHandler};
use actix_web::ws;
use futures::{future, Future, Async, Poll};
use futures::task::{current, Task};
use rmp_serde::{Serializer};
use serde::{Serialize};
use simple_error::{SimpleError};

const PING_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(15);

pub struct AgentConnection {
    last_activity_time: Instant,
    logger: slog::Logger,
    message_futures: Vec<Weak<Mutex<MessageFutureStatus>>>,
    remote: String,
}

impl Actor for AgentConnection {
    type Context = ws::WebsocketContext<Self, super::AppState>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(self.logger, "connection established");
        self.ping(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!(self.logger, "connection closed");
    }
}

struct MessageFutureStatus {
    predicate: Box<Fn(&common::Message) -> bool>,
    result: Option<common::Message>,
    task: Option<Task>,
}

struct MessageFuture {
    status: Arc<Mutex<MessageFutureStatus>>,
}

impl Future for MessageFuture {
    type Item = common::Message;
    type Error = SimpleError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut r = self.status.lock().unwrap();
        if let Some(msg) = r.result.clone() {
            return Ok(Async::Ready(msg));
        }
        r.task = Some(current());
        Ok(Async::NotReady)
    }
}

#[derive(Message)]
#[rtype(result="Result<common::HyperDeckCommandResponse, SimpleError>")]
pub struct HyperDeckCommand {
    pub ip_address: String,
    pub command: String,
}

impl Handler<HyperDeckCommand> for AgentConnection {
    type Result = ResponseFuture<common::HyperDeckCommandResponse, SimpleError>;

    fn handle(&mut self, msg: HyperDeckCommand, ctx: &mut Self::Context) -> Self::Result {
        let id = uuid::Uuid::new_v4().to_string();

		let mut buf = Vec::new();
        let _ = common::Message::HyperDeckCommand{
            id: id.clone(),
            command: msg.command,
            ip_address: msg.ip_address,
        }.serialize(&mut Serializer::new(&mut buf)).unwrap();
		ctx.binary(buf);

        let predicate_cmd_id = id.clone();
        Box::new(
            self.wait_for_message(move |message| {
                match message {
                    common::Message::HyperDeckCommandError{id, ..} => *id == predicate_cmd_id,
                    common::Message::HyperDeckCommandResponse{id, ..} => *id == predicate_cmd_id,
                    _ => false,
                }
            })
                .and_then(|message| {
                    match message {
                        common::Message::HyperDeckCommandError{description, ..} => future::err(SimpleError::new(description)),
                        common::Message::HyperDeckCommandResponse{response, ..} => future::ok(response),
                        _ => panic!("unexpected message"),
                    }
                })
        )
    }
}

impl Handler<common::Message> for AgentConnection {
    type Result = ();

    fn handle(&mut self, msg: common::Message, ctx: &mut Self::Context) {
		let mut buf = Vec::new();
        let _ = msg.serialize(&mut Serializer::new(&mut buf)).unwrap();
		ctx.binary(buf);
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for AgentConnection {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Binary(mut msg) => {
				let bytes = msg.take();
				let msg: Result<common::Message, _> = rmp_serde::from_read_ref(bytes.as_ref());
				match msg {
					Err(e) => {
						error!(self.logger, "error deserializing message: {}", e);
					},
					Ok(v) => {
						let message: common::Message = v;

                        self.message_futures.retain(|status| {
                            match status.upgrade() {
                                Some(status) => {
                                    let mut status = status.lock().unwrap();
                                    if ((*status).predicate)(&message) {
                                        (*status).result = Some(message.clone());
                                        if let Some(ref task) = status.task {
                                            task.notify();
                                        }
                                        false
                                    } else {
                                        true
                                    }
                                },
                                None => false,
                            }
                        });

						match message {
							common::Message::AgentState{id, state} => {
                                let mut agents = ctx.state().shared.agents.write().unwrap();
                                let agent = super::Agent{
                                    id: id.clone(),
                                    remote: self.remote.clone(),
                                    state: state,
                                    addr: ctx.address(),
                                };
                                agents.insert(id, agent);
							},
                            common::Message::ShellOutput{id, bytes} => {
                                let shells = ctx.state().shared.shells.read().unwrap();
                                if let Some(shell) = shells.get(&id) {
                                    shell.addr.do_send(shell_connection::ShellOutput{
                                        bytes: bytes,
                                    });
                                }
                            },
                            _ => {},
						}
					},
				}
            }
            ws::Message::Ping(msg) => {
                self.last_activity_time = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.last_activity_time = Instant::now();
            }
            ws::Message::Close(_) => {
                ctx.stop();
            }
            _ => {}
        }
    }
}

impl AgentConnection {
    pub fn new(logger: slog::Logger, remote: String) -> Self {
        Self {
            message_futures: Vec::new(),
            last_activity_time: Instant::now(),
            logger: logger,
            remote: remote,
        }
    }

    /// Returns a future that resolves when the first message is received that satisfies the given predicate.
    fn wait_for_message<F>(&mut self, f: F) -> MessageFuture
        where F: 'static + Fn(&common::Message) -> bool
    {
        let status = Arc::new(Mutex::new(MessageFutureStatus{
            predicate: Box::new(f),
            result: None,
            task: None,
        }));
        self.message_futures.push(Arc::downgrade(&status));
        MessageFuture{
            status: status,
        }
    }

    fn ping(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(PING_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.last_activity_time) > CLIENT_TIMEOUT {
                info!(act.logger, "disconnecting idle client");
                ctx.stop();
                return;
            }
            ctx.ping("");
        });
    }
}
