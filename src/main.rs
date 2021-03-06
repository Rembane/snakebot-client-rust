#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;
#[macro_use] extern crate serde_derive;
extern crate clap;
extern crate config;
extern crate log4rs;
extern crate rustc_version;
extern crate serde;
extern crate serde_json;
extern crate target_info;
extern crate ws;

mod maputil;
mod messages;
mod snake;
mod structs;
mod util;

use clap::{ Arg, App };
use messages::{ Inbound };
use snake::{ Snake };
use std::path::Path;
use std::string::{ String };
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const LOG_TARGET: &'static str = "client";
const HEART_BEAT_S: u64 = 20;

const CONFIG_FILE: &'static str = "snake.conf";
const DEFAULT_HOST: &'static str = "snake.cygni.se";
const DEFAULT_PORT: &'static str = "80";
const DEFAULT_SNAKE_NAME: &'static str = "default-rust-snake-name";
const DEFAULT_VENUE: &'static str = "training";

#[derive(Clone, Debug)]
struct Config {
    host: String,
    port: i32,
    snake_name: String,
    venue: String
}

quick_error! {
    #[derive(Debug)]
    pub enum ClientError {
        Message(err: serde_json::Error) {
            from()
        }

        Websocket(err: ws::Error) {
            from()
        }

        StringChannel(err: mpsc::SendError<String>) {
            from()
        }

        WebsocketChannel(err: mpsc::SendError<Arc<ws::Sender>>) {
            from()
        }
    }
}

struct Client {
    out: Arc<ws::Sender>,
    snake: Snake,
    config: Config,
    out_sender: mpsc::Sender<Arc<ws::Sender>>,
    id_sender: mpsc::Sender<String>
}

fn route_msg(client: &mut Client, str_msg: &String) -> Result<(), ClientError> {
    let snake = &mut client.snake;
    let inbound_msg = try!(messages::parse_inbound_msg(str_msg));

    match inbound_msg {
        Inbound::GameEnded(msg) => {
            snake.on_game_ended(&msg);
            if client.config.venue == "training" {
                try!(client.out.close(ws::CloseCode::Normal));
            }
        },
        Inbound::TournamentEnded(msg) => {
            snake.on_tournament_ended(&msg);
            try!(client.out.close(ws::CloseCode::Normal));
        },
        Inbound::MapUpdate(msg) => {
            let direction = maputil::direction_as_string(&snake.get_next_move(&msg));
            let response = try!(messages::create_register_move_msg(direction, msg));
            debug!(target: LOG_TARGET, "Responding with RegisterMove {:?}", response);
            try!(client.out.send(response));
        },
        Inbound::SnakeDead(msg) => {
            snake.on_snake_dead(&msg);
        },
        Inbound::GameStarting(msg) => {
            snake.on_game_starting(&msg);
        },
        Inbound::PlayerRegistered(msg) => {
            info!(target: LOG_TARGET, "Successfully registered player");
            snake.on_player_registered(&msg);

            if msg.gameMode == "TRAINING" {
                let response = try!(messages::create_start_game_msg());
                debug!(target: LOG_TARGET, "Requesting a game start {:?}", response);
                try!(client.out.send(response));
            };

            info!(target: LOG_TARGET, "Starting heart beat");
            try!(client.out_sender.send(client.out.clone()));
            try!(client.id_sender.send(msg.receivingPlayerId));
        },
        Inbound::InvalidPlayerName(msg) => {
            snake.on_invalid_playername(&msg);
        },
        Inbound::HeartBeatResponse(_) => {
            // do nothing
        },
        Inbound::GameLinkEvent(msg) => {
            info!(target: LOG_TARGET, "Watch game at {}", msg.url);
        },
        Inbound::UnrecognizedMessage => {
            error!(target: LOG_TARGET, "Received unrecognized message {:?}", str_msg);
        }
    };

    Ok(())
}


impl ws::Handler for Client {
    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        debug!(target: LOG_TARGET, "Connection to Websocket opened");

        let client_info = messages::create_client_info_msg();
        if let Ok(message) = client_info {
            info!(target: LOG_TARGET, "Sending client info to server: {:?}", message);
            try!(self.out.send(message));
        } else {
            error!(target: LOG_TARGET, "Unable to create client info message {:?}", client_info);
            try!(self.out.close(ws::CloseCode::Error));
        }

        let parse_msg = messages::create_play_registration_msg(self.config.snake_name.clone());
        if let Ok(response) = parse_msg {
            info!(target: LOG_TARGET, "Registering player with message: {:?}", response);
            self.out.send(response)
        } else {
            error!(target: LOG_TARGET, "Unable to create play registration message {:?}", parse_msg);
            self.out.close(ws::CloseCode::Error)
        }
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        if let ws::Message::Text(text) = msg {
            let route_result = route_msg(self, &text);
            match route_result {
                Err(e) => error!(target: LOG_TARGET, "Got error \'{:?}\' when routing message: {}", e, text),
                Ok(_) => debug!(target: LOG_TARGET, "Succeeded in routing message {}", text)
            }
        } else {
            warn!(target: LOG_TARGET, "Unexpectedly received non-string message: {:?}", msg)
        }

        Ok(())
    }
}

fn read_conf_file() -> Config {
    let config_path = Path::new(CONFIG_FILE);
    info!(target: LOG_TARGET, "Reading config from file at {:?}", config_path.canonicalize());
    let matches = App::new("Rust snake client")
        .version("1.1.0")
        .author("Martin Barksten <martin.barksten@cygni.se>")
        .about("A snake client in the least friendly language.")
        .arg(Arg::with_name("host")
             .short("h")
             .long("host")
             .help("The host to connect to")
             .takes_value(true)
             .default_value(DEFAULT_HOST))
        .arg(Arg::with_name("port")
             .short("p")
             .long("port")
             .help("The port to connect to")
             .takes_value(true)
             .default_value(DEFAULT_PORT))
        .arg(Arg::with_name("venue")
             .short("v")
             .long("venue")
             .help("The venue (tournament or training)")
             .takes_value(true)
             .default_value(DEFAULT_VENUE)
             .possible_values(&["tournament", "training"]))
        .arg(Arg::with_name("snake-name")
             .short("n")
             .long("snake-name")
             .help("The name of the snake")
             .takes_value(true)
             .default_value(DEFAULT_SNAKE_NAME))
        .get_matches();

    let port = matches.value_of("port").unwrap_or(DEFAULT_PORT).parse::<i32>().unwrap();

    Config {
        host: String::from(matches.value_of("host").unwrap_or(DEFAULT_HOST)),
        port: port,
        snake_name: String::from(matches.value_of("snake-name").unwrap_or(DEFAULT_SNAKE_NAME)),
        venue: String::from(matches.value_of("venue").unwrap_or(DEFAULT_VENUE))
    }
}

fn start_websocket_thread(id_sender: mpsc::Sender<String>,
                          out_sender: mpsc::Sender<Arc<ws::Sender>>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let config = read_conf_file();

        let connection_url = format!("ws://{}:{}/{}", config.host, config.port, config.venue);
        info!(target: LOG_TARGET, "Connecting to {:?}", connection_url);

        let result = ws::connect(connection_url, |out| {
            Client {
                out: Arc::from(out),
                snake: snake::Snake,
                config: config.clone(),
                out_sender: out_sender.clone(),
                id_sender: id_sender.clone()
            }
        });
        debug!(target: LOG_TARGET, "Websocket is done, result {:?}", result);
    })
}

fn do_heart_beat(id: String, out: Arc<ws::Sender>, done_receiver: mpsc::Receiver<()>) {
    loop {
        thread::sleep(Duration::from_secs(HEART_BEAT_S));
        let rec = done_receiver.try_recv();

        // if the channel is disconnected or a done message is sent, break the loop
        if let Err(e) = rec {
            if e == mpsc::TryRecvError::Disconnected {
                debug!(target: LOG_TARGET, "Stopping heartbeat due to channel disconnecting");
                break;
            }
        } else {
            debug!(target: LOG_TARGET, "Stopping heartbeat due to finished execution");
            break;
        }

        debug!(target: LOG_TARGET, "Sending heartbeat request");

        let id = id.clone();
        let parsed_msg = messages::create_heart_beat_msg(id);
        if let Ok(heart_beat) = parsed_msg {
            let send_result = out.send(heart_beat);
            if let Err(e) = send_result {
                error!(target: LOG_TARGET, "Unable to send heartbeat, got error {:?}", e);
            }
        } else {
            error!(target: LOG_TARGET, "Unable to parse heart beat message {:?}", parsed_msg);
        }
    }
}

pub fn recv_channels(id_receiver: mpsc::Receiver<String>,
                     out_receiver: mpsc::Receiver<Arc<ws::Sender>>)
                     -> Result<(String, Arc<ws::Sender>), mpsc::RecvError> {
    let id = try!(id_receiver.recv());
    let out = try!(out_receiver.recv());
    Ok((id, out))
}

fn start_heart_beat_thread(id_receiver: mpsc::Receiver<String>,
                           out_receiver: mpsc::Receiver<Arc<ws::Sender>>,
                           done_receiver: mpsc::Receiver<()>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let res = recv_channels(id_receiver, out_receiver);

        if let Ok((id, out)) = res {
            debug!(target: LOG_TARGET, "Starting heartbeat");
            do_heart_beat(id, out, done_receiver);
        } else {
            error!(target: LOG_TARGET, "Unable to start heart beat, the channel has been closed.");
        };
    })
}

fn start_client() {
    let (id_sender,id_receiver) = mpsc::channel();
    let (out_sender,out_receiver) = mpsc::channel();
    let (done_sender,done_receiver) = mpsc::channel();

    let websocket = start_websocket_thread(id_sender, out_sender);
    let heartbeat = start_heart_beat_thread(id_receiver, out_receiver, done_receiver);

    let websocket_res = websocket.join();
    debug!(target: LOG_TARGET, "Joining Websocket thread gave result {:?}", websocket_res);

    let send_res = done_sender.send(());
    if let Err(e) = send_res {
        error!(target: LOG_TARGET, "Unable to send done message, got error {:?}", e);
    }

    let heartbeat_res = heartbeat.join();
    debug!(target: LOG_TARGET, "Joining heartbeat thread gave result {:?}", heartbeat_res);

}

fn main() {
    if let Err(_) = log4rs::init_file("log4rs.toml", Default::default()) {
        log4rs::init_file("../log4rs.toml", Default::default()).unwrap();
    }
    start_client();
}
