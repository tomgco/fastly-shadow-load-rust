extern crate tokio;
extern crate tokio_codec;
extern crate hyper;

use tokio::codec::Decoder;
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio_codec::LinesCodec;
use serde_json::Value;

use hyper::{Body, Client, Request};
use hyper::rt::{self};

use std::env;
use std::net::SocketAddr;

fn main() -> Result<(), Box<std::error::Error>> {
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse::<SocketAddr>()?;

    let socket = TcpListener::bind(&addr)?;
    println!("Listening on: {}", addr);

    let done = socket
        .incoming()
        .map_err(|e| println!("failed to accept socket; error = {:?}", e))
        .for_each(move |socket| {
            // Once we're inside this closure this represents an accepted client
            // from our server. The `socket` is the client connection (similar to
            // how the standard library operates).
            //
            // We're parsing each socket with the `LinesCodec` included in `tokio_io`,
            // and then we `split` each codec into the reader/writer halves.
            //
            // See https://docs.rs/tokio-codec/0.1/src/tokio_codec/bytes_codec.rs.html
            let framed = LinesCodec::new().framed(socket);
            let (_writer, reader) = framed.split();

            let processor = reader
                .for_each(|line| {
                    let v: Value = serde_json::from_str(&line)?;
                    println!("v: {}", v["hello"]);
                    for i in 1..5 {
                        tokio::spawn(rt::lazy(move || {
                            let client = Client::new();
                            let req = Request::builder()
                                .method("GET")
                                .uri("http://127.0.0.1:9900")
                                .body(Body::from("Hallo!"))
                                .expect("request builder");
                            let future = client.request(req);
                            println!("Making http req {} times", i);
                            Ok(())
                        }));
                    }
                    Ok(())
                })
                // After our copy operation is complete we just print out some helpful
                // information.
                .and_then(|()| {
                    println!("Socket received FIN packet and closed connection");
                    Ok(())
                })
                .or_else(|err| {
                    println!("Socket closed with error: {:?}", err);
                    // We have to return the error to catch it in the next ``.then` call
                    Err(err)
                })
                .then(|result| {
                    println!("Socket closed with result: {:?}", result);
                    Ok(())
                });

            // async by transfering ownership to tokio
            tokio::spawn(processor)
        });

    tokio::run(done);
    Ok(())
}
