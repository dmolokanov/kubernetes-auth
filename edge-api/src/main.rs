use http::header;
use hyper::header::HeaderValue;
use hyper::service::service_fn;
use hyper::{Body, Request, Response, Server};
use reqwest::{Client};
use std::{env, fs};
use tokio::prelude::future;
fn main() {
    let addr = "0.0.0.0:35000".parse().unwrap();

    let base = format!(
        "https://{}:443",
        env::var("KUBERNETES_SERVICE_HOST").expect("env KUBERNETES_SERVICE_HOST required")
    );

    const TOKEN_FILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";
    let auth = fs::read_to_string(TOKEN_FILE).expect("Service account token file expected");

    hyper::rt::run(future::lazy(move || {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();

        let config = Config { base, token: auth };

        let new_service = move || {
            let client = client.clone();
            let config = config.clone();
            service_fn(move |req| response_examples(req, &client, &config))
        };

        let server = Server::bind(&addr)
            .serve(new_service)
            .map_err(|e| eprintln!("server error: {}", e));

        println!("Listening on http://{}", addr);

        server
    }));
}

use tokio::prelude::future::{Either, Future};

#[derive(Clone)]
struct Config {
    base: String,
    token: String,
}

fn response_examples(
    req: Request<Body>,
    client: &Client,
    config: &Config,
) -> Box<Future<Item = Response<Body>, Error = hyper::Error> + Send> {
    println!("{}", req.uri());

    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .map(HeaderValue::to_str)
        .transpose()
        .map(|token| {
            token
                .filter(|token| token.len() > 6 && &token[..7].to_uppercase() == "BEARER ")
                .map(|token| &token[7..])
        })
        .map_err(|err| panic!("{}", err));

    let fut = match token {
        Ok(token) => match token {
            Some(token) => Either::A(Either::A({
                println!("{}", token);
                let body = format!("{{\"apiVersion\": \"authentication.k8s.io/v1\", \"kind\": \"TokenReview\", \"spec\": {{\"token\": \"{}\"  }} }}", token);

                let url = format!("{}/apis/authentication.k8s.io/v1/tokenreviews", config.base);
                let text = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", config.token))
                    .body(reqwest::Body::from(body))
                    .send()
                    .unwrap()
                    .text();

                let res = match text {
                    Ok(text) => Response::new(Body::from(text)),
                    Err(err)=> {
                        let text = format!("Error from KubeAPI: {}", err);
                        Response::new(Body::from(text))
                    }
                };

                println!("a");
                future::ok(res)

            })),
            None => Either::A(Either::B(future::ok(Response::new(Body::from("no token"))))),
        },
        Err(e) => {
            println!("{}", e);
            Either::B(future::err(e))
        }
    };

    Box::new(fut)
}
