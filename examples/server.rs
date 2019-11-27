//! Example HTTPS service using hyper-tls

use futures_util::{
    try_future::TryFutureExt,
    try_stream::TryStreamExt,
};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode
};
use native_tls::{self, Identity};
use std::io;
use tokio::net::tcp::TcpListener;
use tokio_tls;

fn main() {
    if let Err(e) = run_server() {
        println!("Error: {}", e);
        std::process::exit(1);
    }
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

#[tokio::main]
async fn run_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = 8443u16;
    let addr = format!("127.0.0.1:{}", port);

    let der_bytes = include_bytes!("tls_key/private.pkcs12");
    let id = Identity::from_pkcs12(&*der_bytes, "")?;
    let tls_acceptor = native_tls::TlsAcceptor::builder(id).build()?;
    let tls_acceptor2 = tokio_tls::TlsAcceptor::from(tls_acceptor);

    let tcp = TcpListener::bind(&addr).await?;
    let incoming_tls_stream =
        tcp.incoming()
        .map_err(|e| error(format!("TCP listener error: {}", e)))
        .and_then(|s| {
            tls_acceptor2
                .accept(s)
                .map_err(|e| error(format!("TLS accept error: {}", e)))
        });

    let acceptor = hyper::server::accept::from_stream(incoming_tls_stream);
    let service = make_service_fn(|_| async { Ok::<_, io::Error>(service_fn(service)) });
    let server = Server::builder(acceptor)
        .serve(service);
    println!("Serving on https://{}...", addr);
    server.await?;
    Ok(())
}

async fn service(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            *response.body_mut() = Body::from("Hello, world!");
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };
    Ok(response)
}
