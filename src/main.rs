#[macro_use] extern crate actix;
extern crate actix_web;
extern crate decklink;
extern crate futures;
extern crate gethostname;
extern crate ipnetwork;
#[macro_use] extern crate juniper;
extern crate libc;
extern crate pnet;
extern crate rmp_serde;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate simple_error;
#[macro_use] extern crate slog;
extern crate slog_scope;
extern crate slog_stdlog;
extern crate slog_term;
extern crate slog_async;
extern crate tokio;
extern crate tokio_timer;
extern crate url;
extern crate uuid;

mod agent;
mod hub;
mod common;

use slog::Drain;

fn main() -> Result<(), Box<std::error::Error>> {
    let sys = actix::System::new("blackmagic-c2");

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().filter_level(slog::Level::Info).fuse();

    let logger = slog::Logger::root(drain, o!());
    
    let _scope_guard = slog_scope::set_global_logger(logger.clone());
    let _log_guard = slog_stdlog::init().unwrap();

    agent::start(agent::Config{
        id: gethostname::gethostname().into_string().unwrap(),
        hub_url: "http://127.0.0.1:8080".to_string(),
        logger: logger.new(o!("service" => "agent")),
    })?;

    hub::start(hub::Config{
        port: 8080,
        logger: logger.new(o!("service" => "hub")),
    });

    let _ = sys.run();
    Ok(())
}
