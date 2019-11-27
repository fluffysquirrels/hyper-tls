//! Example HTTPS service using hyper-tls

use core::task::{Context, Poll};
use futures_util::{
    stream::{Stream, StreamExt},
    try_future::TryFutureExt,
    try_stream::TryStreamExt,
};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode
};
use native_tls::{
    self,
    Identity,
};
use std::{
    fs,
    io,
    pin::Pin,
    sync::Arc,
};
use tokio::net::tcp::{TcpListener, TcpStream};
use tokio_tls::{
    self,
    TlsStream,
};

fn main() {
    // Serve an echo service over HTTPS, with proper error handling.
    if let Err(e) = run_server() {
        eprintln!("FAILED: {}", e);
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

    let der_bytes = fs::read("examples/tls_key/private.pkcs12").expect("Read TLS key");
    let id = Identity::from_pkcs12(&*der_bytes, "")?;
    let tls_acceptor = native_tls::TlsAcceptor::builder(id).build()?;
    let tls_acceptor2 = Arc::new(tokio_tls::TlsAcceptor::from(tls_acceptor));

    let tcp = TcpListener::bind(&addr).await?;
    let incoming_tls_stream: Pin<Box<dyn Stream<Item = Result<TlsStream<TcpStream>, io::Error>>>> =
        tcp.incoming()
            .map_err(|e| error(format!("TCP listener error: {}", e)))
            .and_then(move |s| {
                tls_acceptor2.accept(s)
                    .map_err(|e| error(format!("TLS accept error: {}", e)))
            })
            .boxed();

    let acceptor = Acceptor {
        inner: incoming_tls_stream,
    };
    let service = make_service_fn(|_| async { Ok::<_, io::Error>(service_fn(service)) });
    let server = Server::builder(acceptor)
        .serve(service);
    println!("Serving on https://{}...", addr);
    server.await?;
    Ok(())
}

struct Acceptor {
    inner: Pin<Box<dyn Stream<Item = Result<TlsStream<TcpStream>, io::Error>>>>,
}

impl hyper::server::accept::Accept for Acceptor {
    type Conn = TlsStream<TcpStream>;
    type Error = io::Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
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
