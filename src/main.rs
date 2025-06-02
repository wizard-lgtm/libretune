use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Result};
use serde::Deserialize;
use tokio;
use tokio::time::Duration;
use std::env;

use std::sync::LazyLock;
use surrealdb;
use surrealdb::Surreal;
use crate::surrealdb::engine::remote::ws::Client;
use crate::surrealdb::opt::auth::Root;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::Error;

use dotenv::dotenv;


mod error {
    use actix_web::{HttpResponse, ResponseError};
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum Error {
        #[error("database error")]
        Db(String),
    }

    impl ResponseError for Error {
        fn error_response(&self) -> HttpResponse {
            match self {
                Error::Db(e) => HttpResponse::InternalServerError().body(e.to_string()),
            }
        }
    }

    impl From<surrealdb::Error> for Error {
        fn from(error: surrealdb::Error) -> Self {
            eprintln!("{error}");
            Self::Db(error.to_string())
        }
    }
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/users/{user_id}/")] // <- define path parameters
async fn index(path: web::Path<(String)>) -> impl Responder {
    let (user_id) = path.into_inner();
    let result = format!("Welcome {}!", user_id);
    HttpResponse::Ok().body(result)
}

#[derive(Deserialize)]
struct SearchParams {
    query: String,
    limit: Option<u32>,
    offset: Option<u32>,
}
#[get("/search")]
async fn search(params: web::Query<SearchParams>) -> impl Responder {
    let query = &params.query;
    let limit = params.limit.unwrap_or(10);
    let offset = params.offset.unwrap_or(0);
    
    // Simulate a search operation
    let result = format!("Searching for '{}' with limit {} and offset {}", query, limit, offset);
    
    HttpResponse::Ok().body(result)
}

async fn db_connect(db: &LazyLock<Surreal<Client>>) -> Result<(),  surrealdb::Error>{

    db.connect::<Ws>("localhost:8000").await?;


    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;

    println!("🚀 Connected to SurrealDB!");

    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok(); // Load environment variables from `.env`

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT")
    .unwrap_or_else(|_| "8000".to_string())
    .parse::<u16>()
    .expect("PORT must be a number");


    static DB: LazyLock<Surreal<Client>> = LazyLock::new(Surreal::init);

    if let Err(e) = db_connect(&DB).await {
        eprintln!("❌ Failed to connect to SurrealDB: {}", e);
        std::process::exit(1);
    } 

    println!("🚀 Libretune is running at http://127.0.0.1:{}", port);

    HttpServer::new(|| {
        App::new()
        .service(hello)
        .service(index)
        .service(search)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}