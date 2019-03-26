extern crate futures;
extern crate tokio;
extern crate tokio_codec;
extern crate hyper;

use tokio::codec::Decoder;
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio_codec::LinesCodec;
use serde_json::Value;

use hyper::{Body, Client, Request};
use hyper::client::HttpConnector;

use std::env;
use std::net::SocketAddr;

fn fetch_url(client: &Client<HttpConnector>, url: &str, host_override: String, method: &str) -> impl Future<Item=(), Error=()> {
    let mut req = Request::builder();

    println!("{}", url);

    req.method(method)
        .uri(url)
        .header("User-Agent", "Fastly-Shadow-Traffic/2.0(Conde Nast International)");

    if host_override != "" {
        req.header("Host", host_override);
    }

    let final_req = req.body(Body::empty())
        .expect("request builder");

    client
        .request(final_req)
        .map(|_| {
            println!("Done.");
        })
        .map_err(|err| {
            eprintln!("Error {}", err);
        })
}

fn main() -> Result<(), Box<std::error::Error>> {
    let client = Client::new();

    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse::<SocketAddr>()?;
    let times = env::args().nth(2).unwrap_or("2".to_string()).parse::<i32>().unwrap();

    let socket = TcpListener::bind(&addr)?;
    println!("Listening on: {}", addr);

    let client = client.clone();
    let done = socket
        .incoming()
        .map_err(|e| println!("failed to accept socket; error = {:?}", e))
        .for_each(move |socket| {
            let framed = LinesCodec::new().framed(socket);
            let (_writer, reader) = framed.split();

            let client = client.clone();
            let processor = reader
                .for_each(move |line| {
                    let v: Value = serde_json::from_str(&line)?;
                    let event = &v["event"];
                    let url = event["url"].as_str().unwrap();
                    let method = event["request"].as_str().unwrap();

                    for _i in 0..times {
                        tokio::spawn(
                            fetch_url(
                                &client,
                                url,
                                "".to_string(),
                                method
                            )
                        );
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
