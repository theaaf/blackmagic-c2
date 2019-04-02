use super::agent_connection;
use super::common;

use std::time::{Instant, Duration};
use std::sync::{Arc};

use actix::{
    Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler,
};
use actix_web::ws;

const PING_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Message)]
pub struct ShellOutput {
    pub bytes: Vec<u8>,
}

pub struct ShellConnection {
    id: String,
    last_activity_time: Instant,
    logger: slog::Logger,
    agent_addr: Addr<agent_connection::AgentConnection>,
}

impl Actor for ShellConnection {
    type Context = ws::WebsocketContext<Self, super::AppState>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(self.logger, "connection established");

        self.ping(ctx);

        let mut shells = ctx.state().shared.shells.write().unwrap();
        shells.insert(self.id.clone(), super::Shell{
            id: self.id.clone(),
            addr: ctx.address(),
        });

        self.agent_addr.do_send(common::Message::ShellInit{
            id: self.id.clone(),
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!(self.logger, "connection closed");

        self.agent_addr.do_send(common::Message::ShellClose{
            id: self.id.clone(),
        });
    }
}

impl Handler<ShellOutput> for ShellConnection {
    type Result = ();

    fn handle(&mut self, msg: ShellOutput, ctx: &mut Self::Context) {
        ctx.text(Arc::new(msg.bytes));
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for ShellConnection {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Text(msg) => {
                self.agent_addr.do_send(common::Message::ShellInput{
                    id: self.id.clone(),
                    bytes: msg.into(),
                });
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

impl ShellConnection {
    pub fn new(logger: slog::Logger, agent_addr: Addr<agent_connection::AgentConnection>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            last_activity_time: Instant::now(),
            logger: logger,
            agent_addr: agent_addr,
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
