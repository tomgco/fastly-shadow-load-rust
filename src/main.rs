extern crate futures;
extern crate tokio;
extern crate tokio_codec;
extern crate hyper;
extern crate clap;

use clap::{App, Arg};

use tokio::codec::Decoder;
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio_codec::LinesCodec;
use serde_json::Value;

use hyper::{Body, Client, Request};
use hyper::client::HttpConnector;

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
    let matches = App::new("fastly-shadow-load")
                        .version("1.0")
                        .author("Tom Gallacher <me@tomg.co>")
                        .about("Slam your servers with requests!")
                        .arg(Arg::with_name("listen")
                                    .short("l")
                                    .long("listen")
                                    .help("Address and port which the syslog server will bind to")
                                    .default_value("127.0.0.1:8080"))
                        .arg(Arg::with_name("filter_hit")
                                    .short("f")
                                    .long("filterHit")
                                    .help("Filters out cache hits. By default it does not filter out x-cache: HIT")
                                    .default_value("false"))
                        .arg(Arg::with_name("times")
                                    .short("x")
                                    .long("times")
                                    .help("Number of times to repeat a request")
                                    .default_value("1"))
                        .arg(Arg::with_name("target")
                                    .short("t")
                                    .long("target")
                                    .help("Target HTTP host, where traffic will be sent, we use Kubernetes Services")
                                    .default_value("http://my-service.default.svc.cluster.local"))
                        .arg(Arg::with_name("host")
                                    .long("host")
                                    .default_value("other-service.default.svc.cluster.local")
                                    .help("HTTP(s) host override, can be used to send traffic to ingress controllers"))
                        .get_matches();

    let client = Client::new();

    let addr = matches.value_of("listen").unwrap();
    let times = matches.value_of("times").unwrap().parse::<i32>().unwrap();
    let host_override = matches.value_of("host").unwrap().to_owned();
    let filter_hit = matches.value_of("host").unwrap().parse::<bool>().to_owned();

    let addr = addr.parse::<SocketAddr>()?;

    let socket = TcpListener::bind(&addr)?;
    println!("Listening on: {}", addr);

    let client = client.clone();
    let host_override = host_override.clone();
    let filter_hit = filter_hit.clone();

    let done = socket
        .incoming()
        .map_err(|e| println!("failed to accept socket; error = {:?}", e))
        .for_each(move |socket| {
            let framed = LinesCodec::new().framed(socket);
            let (_writer, reader) = framed.split();

            let client = client.clone();
            let host_override = host_override.clone();
            let filter_hit = filter_hit.clone();
            let processor = reader
                .for_each(move |line| {
                    let filter_hit = filter_hit.clone();
                    let v: Value = serde_json::from_str(&line)?;
                    let event = &v["event"];
                    let req_hit = event["hit"].as_bool().unwrap();
                    if req_hit && filter_hit.unwrap() {
                        let url = event["url"].as_str().unwrap();
                        let method = event["request"].as_str().unwrap();

                        for _i in 0..times {
                            let host_override = host_override.clone();
                            tokio::spawn(
                                fetch_url(
                                    &client,
                                    url,
                                    host_override,
                                    method
                                )
                            );
                        }
                        Ok(())
                    } else {
                        Ok(())
                    }
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
