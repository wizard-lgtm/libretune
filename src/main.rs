mod db;
use db::connect_db;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;
use std::env;
use dotenv::dotenv;
mod logging;
mod request_logger;
use request_logger::RequestLogger;
use tracing_actix_web::TracingLogger;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/users/{user_id}/")] // <- define path parameters
async fn index(path: web::Path<String>) -> impl Responder {
    let user_id = path.into_inner();
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

// Add a test endpoint that returns different status codes for testing
#[get("/test/{status}")]
async fn test_status(path: web::Path<u16>) -> impl Responder {
    let status_code = path.into_inner();
    match status_code {
        200 => HttpResponse::Ok().body("Success!"),
        404 => HttpResponse::NotFound().body("Not Found!"),
        500 => HttpResponse::InternalServerError().body("Server Error!"),
        _ => HttpResponse::BadRequest().body("Bad Request!"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok(); // Load environment variables from `.env`
    logging::init(); // Initialize logging

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");
    
    if let Err(e) = connect_db().await {
        eprintln!("❌ Failed to connect to SurrealDB: {}", e);
        std::process::exit(1);
    } 
    
    println!("🚀 Libretune is running at http://127.0.0.1:{}", port);
    
    HttpServer::new(|| {
        App::new()
            .wrap(RequestLogger::with_defaults()) // Add custom request logger
            .wrap(TracingLogger::default()) 
            .service(hello)
            .service(index)
            .service(search)
            .service(test_status) // Add test endpoint
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}