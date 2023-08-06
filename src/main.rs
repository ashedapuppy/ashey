use std::{fs::File, io::BufReader};

use actix_files::NamedFile;
use actix_web::{
    dev::{self, Service},
    get, http,
    middleware::{ErrorHandlerResponse, ErrorHandlers, Logger},
    App, HttpResponse, HttpServer, Responder, Result,
};
use futures_util::{
    future::{self, Either},
    FutureExt,
};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};

#[get("/")]
async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await
}

fn bad_request<B>(res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    let new_resp = NamedFile::open("static/errors/400.html")?
        .customize()
        .with_status(res.status())
        .respond_to(res.request())
        .map_into_boxed_body()
        .map_into_right_body();

    Ok(ErrorHandlerResponse::Response(res.into_response(new_resp)))
}

fn not_found<B>(res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    let new_resp = NamedFile::open("static/errors/404.html")?
        .customize()
        .with_status(res.status())
        .respond_to(res.request())
        .map_into_boxed_body()
        .map_into_right_body();

    Ok(ErrorHandlerResponse::Response(res.into_response(new_resp)))
}

fn internal_server_error<B>(res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    let new_resp = NamedFile::open("static/errors/500.html")?
        .customize()
        .with_status(res.status())
        .respond_to(res.request())
        .map_into_boxed_body()
        .map_into_right_body();

    Ok(ErrorHandlerResponse::Response(res.into_response(new_resp)))
}

fn load_rustls_config() -> rustls::ServerConfig {
    // init server config builder with safe defaults
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth();

    // load TLS key/cert files
    let cert_file = &mut BufReader::new(File::open("cert.pem").unwrap());
    let key_file = &mut BufReader::new(File::open("key.pem").unwrap());

    // convert files to key/cert objects
    let cert_chain = certs(cert_file)
        .unwrap()
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
        .unwrap()
        .into_iter()
        .map(PrivateKey)
        .collect();

    // exit if no keys could be parsed
    if keys.is_empty() {
        log::error!("Could not locate PKCS 8 private keys.");
        std::process::exit(1);
    }

    config.with_single_cert(cert_chain, keys.remove(0)).unwrap()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let config = load_rustls_config();

    log::info!("starting HTTPS server");

    HttpServer::new(|| {
        App::new()
            .wrap_fn(|sreq, srv| {
                let host = sreq.connection_info().host().to_owned();
                let uri = sreq.uri().to_owned();
                let url = format!("https://{host}{uri}");

                if sreq.connection_info().scheme() == "https" {
                    Either::Left(srv.call(sreq).map(|res| res))
                } else {
                    println!("An http request has arrived here, i will redirect it to use https");
                    return Either::Right(future::ready(Ok(sreq.into_response(
                        HttpResponse::MovedPermanently()
                            .append_header((http::header::LOCATION, url))
                            .finish(),
                    ))));
                }
            })
            .wrap(Logger::default())
            .wrap(
                ErrorHandlers::new()
                    .handler(
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        internal_server_error,
                    )
                    .handler(http::StatusCode::BAD_REQUEST, bad_request)
                    .handler(http::StatusCode::NOT_FOUND, not_found),
            )
            .service(index)
    })
    .bind(("95.217.119.115", 80))?
    .bind_rustls("95.217.119.115:443", config)?
    .run()
    .await
}
