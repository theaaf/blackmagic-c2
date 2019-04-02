mod network_scanner;
mod decklink_watcher;
mod hub_connection;
mod hyperdeck;
mod shell;

use futures::Future;
use actix::{Actor, StreamHandler};
use actix_web::ws::{Client};

pub struct Config {
    pub id: String,
    pub hub_url: String,
    pub logger: slog::Logger,
}

pub fn start(config: Config) -> Result<(), url::ParseError> {
    let hub_url = url::Url::parse(config.hub_url.as_str())?;
    let hub_endpoint = hub_url.join("agent")?;
    let err_logger = config.logger.new(o!());
    actix::Arbiter::spawn(
        Client::new(hub_endpoint)
            .connect()
            .map_err(move |e| {
                error!(err_logger, "{}", e);
                ()
            })
            .map(|(reader, writer)| {
                let _ = hub_connection::HubConnection::create(move |ctx| {
                    hub_connection::HubConnection::add_stream(reader, ctx);
                    hub_connection::HubConnection::new(config.logger.new(o!("actor" => "hub connection")), writer, config.id)
                });
                ()
            }),
    );
    Ok(())
}
