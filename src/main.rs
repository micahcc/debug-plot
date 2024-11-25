use std::time::Duration;

use futures::sink::SinkExt;
use futures::stream::StreamExt;
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response};
use hyper_tungstenite::{tungstenite, HyperWebsocket};
use hyper_util::rt::TokioIo;
use tungstenite::Message;
use vega_lite_5::Showable;
use vega_lite_5::Vegalite;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

const STATIC_FILES: &[(&str, &[u8])] = &include!(concat!(env!("OUT_DIR"), "/static_files.rs"));

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
struct VegaPoint {
    x: f64,
    y: Option<f64>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
struct VegaDisplay {
    points: Vec<VegaPoint>,
}

struct Object {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

fn make_vega_data(objects: &[Object]) -> VegaDisplay {
    let sum_x: f64 = objects.iter().map(|o| o.x0).sum();
    let mean_x = sum_x / objects.len() as f64;

    let mut points = vec![];
    for (i, obj) in objects.iter().enumerate() {
        if i > 0 {
            points.push(VegaPoint { x: mean_x, y: None });
        }
        points.push(VegaPoint {
            x: obj.x0,
            y: Some(obj.y0),
        });
        points.push(VegaPoint {
            x: obj.x1,
            y: Some(obj.y1),
        });
    }

    return VegaDisplay { points };
}

/// Handle a HTTP or WebSocket request.
async fn handle_request(mut request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Error> {
    log::info!("{:?}", request);

    for s in STATIC_FILES.iter() {
        log::info!("checking {} vs {}", request.uri(), s.0);
        if request.uri() == "/" && s.0 == "/index.html" {
            return Ok(Response::new(Full::<Bytes>::from(s.1)));
        }
        if request.uri() == s.0 {
            return Ok(Response::new(Full::<Bytes>::from(s.1)));
        }
    }

    if request.uri() == "/ws" {
        if hyper_tungstenite::is_upgrade_request(&request) {
            let (response, websocket) = hyper_tungstenite::upgrade(&mut request, None)?;

            // Spawn a task to handle the websocket connection.
            tokio::spawn(async move {
                if let Err(e) = serve_websocket(websocket).await {
                    eprintln!("Error in websocket connection: {e}");
                }
            });

            // Return the response so the spawned future can continue.
            Ok(response)
        } else {
            Ok(Response::new(Full::<Bytes>::from("Nothing here")))
        }
    } else {
        Ok(Response::new(Full::<Bytes>::from("Nothing here")))
    }
}

fn handle_timeout() -> Option<VegaDisplay> {
    let update = vec![
        Object {
            x0: 200.0,
            y0: 223.0,
            x1: 210.0,
            y1: 233.0,
        },
        Object {
            x0: 300.0,
            y0: 323.0,
            x1: 310.0,
            y1: 333.0,
        },
        Object {
            x0: 100.0,
            y0: 123.0,
            x1: 110.0,
            y1: 133.0,
        },
    ];

    return Some(make_vega_data(&update));
}

fn handle_input(msg: Message) -> Option<VegaDisplay> {
    match msg {
        Message::Text(msg) => {
            println!("Received text message: {msg}");
        }
        Message::Binary(msg) => {
            println!("Received binary message: {msg:02X?}");
        }
        Message::Ping(msg) => {
            // No need to send a reply: tungstenite takes care of this for you.
            println!("Received ping message: {msg:02X?}");
        }
        Message::Pong(msg) => {
            println!("Received pong message: {msg:02X?}");
        }
        Message::Close(msg) => {
            // No need to send a reply: tungstenite takes care of this for you.
            if let Some(msg) = &msg {
                println!(
                    "Received close message with code {} and message: {}",
                    msg.code, msg.reason
                );
            } else {
                println!("Received close message");
            }
        }
        Message::Frame(_msg) => {
            unreachable!();
        }
    }
    return None;
}

/// Handle a websocket connection.
async fn serve_websocket(websocket: HyperWebsocket) -> Result<(), Error> {
    let mut websocket = websocket.await?;
    loop {
        let out_message = tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                handle_timeout()
            }
            msg = websocket.next() => {
                match msg {
                    Some(Ok(msg)) => { handle_input(msg) }
                    _ => {
                        break;
                    }
                }
            },
        };

        if let Some(m) = out_message {
            let msg_str = serde_json::to_string(&m).expect("must serialize");
            websocket.send(Message::text(msg_str)).await?;
        }
    }

    log::info!("Exiting");
    return Ok(());
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let addr: std::net::SocketAddr = "[::1]:3000".parse()?;
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Listening on http://{addr}");

    let mut http = hyper::server::conn::http1::Builder::new();
    http.keep_alive(true);

    //let vl = Vegalite {
    //    description: Some("Plot".to_string()),
    //    ..Default::default()
    //};

    //println!("wrote to {:?}", vl.to_string()?);

    loop {
        let (stream, _) = listener.accept().await?;
        let connection = http
            .serve_connection(
                TokioIo::new(stream),
                hyper::service::service_fn(handle_request),
            )
            .with_upgrades();
        tokio::spawn(async move {
            if let Err(err) = connection.await {
                println!("Error serving HTTP connection: {err:?}");
            }
        });
    }
}
