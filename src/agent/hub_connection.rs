use super::network_scanner::{NetworkState, NetworkScanner};
use super::decklink_watcher::{DeckLinkState, DeckLinkWatcher};

use common;
use super::hyperdeck;
use super::shell;

use std::error::Error;
use std::collections::HashMap;
use std::io::{Write};
use std::time::{Instant, Duration};
use std::net::{IpAddr, SocketAddr};

use futures::{Future};
use actix::{Actor, ActorContext, ActorFuture, Addr, AsyncContext, Context, Handler, StreamHandler};
use actix_web::ws::{ClientWriter, Message, ProtocolError};
use rmp_serde::{Serializer};
use serde::{Serialize};
use actix_web::ws;

const PING_INTERVAL: Duration = Duration::from_secs(5);
const SERVER_TIMEOUT: Duration = Duration::from_secs(15);

pub struct HubConnection {
    agent_id: String,
    agent_state: common::AgentState,
    network_scanner: Option<Addr<NetworkScanner>>,
    decklink_watcher: Option<Addr<DeckLinkWatcher>>,
    last_activity_time: Instant,
    logger: slog::Logger,
    shells: HashMap<String, shell::Shell>,
    writer: ClientWriter,
}

impl Actor for HubConnection {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.network_scanner = Some(NetworkScanner::start(NetworkScanner::new(self.logger.new(o!("actor" => "network scanner")), ctx.address().recipient())));
        self.decklink_watcher = Some(DeckLinkWatcher::start(DeckLinkWatcher::new(self.logger.new(o!("actor" => "decklink watcher")), ctx.address().recipient())));
        self.ping(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        info!(self.logger, "disconnected");
    }
}

impl Handler<shell::ShellOutput> for HubConnection {
    type Result = ();

    fn handle(&mut self, msg: shell::ShellOutput, _ctx: &mut Self::Context) {
		let mut buf = Vec::new();
        let _ = common::Message::ShellOutput{
            id: msg.id,
            bytes: msg.bytes,
        }.serialize(&mut Serializer::new(&mut buf)).unwrap();
		self.writer.binary(buf);
    }
}

impl StreamHandler<Message, ProtocolError> for HubConnection {
    fn handle(&mut self, msg: Message, ctx: &mut Context<Self>) {
        match msg {
            ws::Message::Binary(mut msg) => {
				let bytes = msg.take();
				let msg: Result<common::Message, _> = rmp_serde::from_read_ref(bytes.as_ref());
				match msg {
					Err(e) => {
						error!(self.logger, "{}", e);
					},
					Ok(v) => {
						let message: common::Message = v;
						match message {
							common::Message::ShellInit{id} => {
                                match shell::Shell::new(&id, ctx.address().recipient()) {
                                    Ok(shell) => {
                                        self.shells.insert(id, shell);
                                    },
                                    Err(e) => {
                                        error!(self.logger, "{}", e);
                                    },
                                }
							},
							common::Message::ShellInput{id, bytes} => {
                                if let Some(shell) = self.shells.get_mut(&id) {
                                    if let Err(e) = shell.write_all(bytes.as_ref()) {
                                        error!(self.logger, "{}", e);
                                    }
                                }
                            },
							common::Message::ShellClose{id} => {
                                self.shells.remove(&id);
                            },
                            common::Message::HyperDeckCommand{id, ip_address, command} => {
                                let addr: Result<IpAddr, _> = ip_address.parse();
                                match addr {
                                    Err(e) => {
                                        let mut buf = Vec::new();
                                        let _ = common::Message::HyperDeckCommandError{
                                            id: id,
                                            description: e.description().to_string(),
                                        }.serialize(&mut Serializer::new(&mut buf)).unwrap();
                                        self.writer.binary(buf);
                                    },
                                    Ok(addr) => {
                                        let addr = SocketAddr::new(addr, hyperdeck::DEFAULT_PORT);
                                        ctx.spawn(
                                            actix::fut::wrap_future::<_, Self>(
                                                hyperdeck::HyperDeck::connect(&addr)
                                                    .and_then(|hyperdeck| hyperdeck.write_command(command))
                                                    .and_then(|hyperdeck| hyperdeck.read_command_response())
                                            ).then(|result, act, _| {
                                                let mut buf = Vec::new();
                                                match result {
                                                    Err(e) => {
                                                        let _ = common::Message::HyperDeckCommandError{
                                                            id: id,
                                                            description: e.description().to_string(),
                                                        }.serialize(&mut Serializer::new(&mut buf)).unwrap();
                                                    },
                                                    Ok((_, response)) => {
                                                        let _ = common::Message::HyperDeckCommandResponse{
                                                            id: id,
                                                            response: common::HyperDeckCommandResponse{
                                                                code: response.code,
                                                                text: response.text,
                                                                payload: response.payload,
                                                            },
                                                        }.serialize(&mut Serializer::new(&mut buf)).unwrap();
                                                    },
                                                };
                                                act.writer.binary(buf);
                                                actix::fut::ok(())
                                            })
                                        );
                                    },
                                }
                            },
                            _ => {},
						}
					},
				}
            }
            Message::Ping(msg) => {
                self.last_activity_time = Instant::now();
                self.writer.pong(&msg);
            }
            Message::Pong(_) => {
                self.last_activity_time = Instant::now();
            }
            _ => (),
        }
    }

    fn started(&mut self, _ctx: &mut Context<Self>) {
        info!(self.logger, "connected");
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        info!(self.logger, "server disconnected");
        ctx.stop()
    }
}

impl Handler<NetworkState> for HubConnection {
    type Result = ();

    fn handle(&mut self, msg: NetworkState, _ctx: &mut Context<Self>) {
        let mut state = self.agent_state.clone();
        state.network_devices = msg.devices;
        self.agent_state = state.clone();
		let mut buf = Vec::new();
        let _ = common::Message::AgentState{
            id: self.agent_id.clone(),
            state: state,
        }.serialize(&mut Serializer::new(&mut buf)).unwrap();
		self.writer.binary(buf);
    }
}

impl Handler<DeckLinkState> for HubConnection {
    type Result = ();

    fn handle(&mut self, msg: DeckLinkState, _ctx: &mut Context<Self>) {
        let mut state = self.agent_state.clone();
        state.decklink_devices = msg.devices;
        self.agent_state = state.clone();
		let mut buf = Vec::new();
        let _ = common::Message::AgentState{
            id: self.agent_id.clone(),
            state: state,
        }.serialize(&mut Serializer::new(&mut buf)).unwrap();
		self.writer.binary(buf);
    }
}

impl HubConnection {
    pub fn new(logger: slog::Logger, writer: ClientWriter, agent_id: String) -> Self {
        Self {
            agent_id: agent_id,
            agent_state: common::AgentState{
                network_devices: Vec::new(),
                decklink_devices: Vec::new(),
            },
            network_scanner: None,
            decklink_watcher: None,
            last_activity_time: Instant::now(),
            logger: logger,
            shells: HashMap::new(),
            writer: writer,
        }
    }

    fn ping(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(PING_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.last_activity_time) > SERVER_TIMEOUT {
                info!(act.logger, "disconnecting idle server");
                ctx.stop();
                return;
            }
            act.writer.ping("");
        });
    }
}
