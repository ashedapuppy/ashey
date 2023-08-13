use actix_files::{Files, NamedFile};
use actix_web::{
    get, http,
    middleware::{ErrorHandlers, Logger, NormalizePath},
    web, App, HttpResponse, HttpServer, Responder,
};

mod errors;
mod ssl;

#[get("/")]
async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await
}

#[get("/contact")]
async fn contact() -> impl Responder {
    NamedFile::open_async("./static/contact.html").await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let config = ssl::load_rustls_config();

    log::info!("starting HTTPS server");

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .wrap(
                ErrorHandlers::new()
                    .handler(
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        errors::internal_server_error,
                    )
                    .handler(http::StatusCode::BAD_REQUEST, errors::bad_request)
                    .handler(http::StatusCode::NOT_FOUND, errors::not_found),
            )
            .wrap(NormalizePath::default())
            .service(index)
            .service(contact)
            .service(Files::new("/posts", "./static/posts").index_file("index.html"))
            .service(Files::new("/badges", "./static/badges"))
            .route("/health", web::get().to(HttpResponse::Ok))
    })
    .bind_rustls("0.0.0.0:443", config)?
    .run()
    .await
}
