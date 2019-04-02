mod agent_connection;
mod shell_connection;
mod graphql;

use common;

use futures::Future;
use actix::{Actor, Addr, Handler};
use actix_web::{AsyncResponder, Error, fs, FutureResponse, http, HttpRequest, HttpResponse, Json, middleware, State, ws};
use juniper::http::GraphQLRequest;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, GraphQLObject)]
pub struct Agent {
    pub id: String,
    pub remote: String,
    pub state: common::AgentState,

    #[graphql(skip)]
    pub addr: Addr<agent_connection::AgentConnection>,
}

pub struct Shell {
    pub id: String,
    pub addr: Addr<shell_connection::ShellConnection>,
}

pub struct SharedState {
    pub agents: RwLock<HashMap<String, Agent>>,
    pub shells: RwLock<HashMap<String, Shell>>,
}

pub struct AppState {
    executor: actix::Addr<GraphQLExecutor>,
    logger: slog::Logger,
	shared: Arc<SharedState>,
}

pub struct Config {
    pub port: u16,
    pub logger: slog::Logger,
}

pub fn start(config: Config) {
    info!(config.logger, "listening at http://127.0.0.1:{}", config.port);

	let shared_state = Arc::new(SharedState{
        agents: RwLock::new(HashMap::new()),
        shells: RwLock::new(HashMap::new()),
	});
    let schema = Arc::new(graphql::create_schema());
    let gql_shared_state = shared_state.clone();
    let addr = actix::SyncArbiter::start(3, move || GraphQLExecutor::new(schema.clone(), gql_shared_state.clone()));
    let logger = config.logger.new(o!());

    actix_web::server::new(move || actix_web::App::with_state(AppState{
        executor: addr.clone(),
        logger: logger.new(o!()),
		shared: shared_state.clone(),
    })
        .configure(|app| {
            middleware::cors::Cors::for_app(app)
                .allowed_headers(vec![http::header::AUTHORIZATION, http::header::CONTENT_TYPE])
                .resource("/graphql", |r| r.post().with(graphql))
                .resource("/shell", |r| r.get().f(shell))
                .register()
        })
        .resource("/agent", |r| r.get().f(agent))
        .handler("/", fs::StaticFiles::new("src/hub/ui/dist").unwrap().index_file("index.html"))
    )
        .bind(std::net::SocketAddr::from(([0, 0, 0, 0], config.port)))
        .unwrap()
        .start();
}

#[derive(Serialize, Deserialize)]
struct GraphQLData(GraphQLRequest);

impl actix::Message for GraphQLData {
    type Result = Result<String, Error>;
}

struct GraphQLExecutor {
    schema: Arc<graphql::Schema>,
    shared_state: Arc<SharedState>,
}

impl GraphQLExecutor {
    fn new(schema: Arc<graphql::Schema>, shared_state: Arc<SharedState>) -> GraphQLExecutor {
        GraphQLExecutor {
            schema: schema,
            shared_state: shared_state,
        }
    }
}

impl Actor for GraphQLExecutor {
    type Context = actix::SyncContext<Self>;
}

impl Handler<GraphQLData> for GraphQLExecutor {
    type Result = Result<String, Error>;
    fn handle(&mut self, msg: GraphQLData, _ctx: &mut Self::Context) -> Self::Result {
        let res = msg.0.execute(&self.schema, &graphql::Context{
            shared_state: self.shared_state.clone(),
        });
        let res_text = serde_json::to_string(&res)?;
        Ok(res_text)
    }
}

fn graphql((st, data): (State<AppState>, Json<GraphQLData>)) -> FutureResponse<HttpResponse> {
    st.executor
        .send(data.0)
        .from_err()
        .and_then(|res| match res {
            Ok(user) => Ok(HttpResponse::Ok().content_type("application/json").body(user)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

fn agent(r: &HttpRequest<AppState>) -> Result<HttpResponse, Error> {
    ws::start(r, agent_connection::AgentConnection::new(r.state().logger.new(o!(
        "agent_connection_id" => uuid::Uuid::new_v4().to_string(),
    )), r.connection_info().remote().unwrap_or("").to_string()))
}

fn shell(r: &HttpRequest<AppState>) -> Result<HttpResponse, Error> {
    let agents = r.state().shared.agents.read().unwrap();
    match agents.get(r.query().get("agent").unwrap_or(&"".to_string())) {
        Some(agent) => {
            ws::start(r, shell_connection::ShellConnection::new(r.state().logger.new(o!(
                "shell_connection_id" => uuid::Uuid::new_v4().to_string(),
            )), agent.addr.clone()))
        },
        None => Ok(HttpResponse::BadRequest().into()),
    }
}
