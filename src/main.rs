use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Result};
use serde::Deserialize;
use tokio;
use tokio::time::Duration;

#[get("/")]
async fn hello() -> impl Responder {
    tokio::time::sleep(Duration::from_secs(5)).await;
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
        .service(hello)
        .service(index)
        .service(search)
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await
}