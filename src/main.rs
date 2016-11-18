#[warn(unused_variables)]

extern crate hyper;
extern crate env_logger;
extern crate websocket;
extern crate rustc_serialize;

use std::env;
use std::str::FromStr;

/// Look up our server port number in PORT, for compatibility with Heroku.
fn get_server_port() -> u16 {
    let port_str = env::var("PORT").unwrap_or(String::new());
    FromStr::from_str(&port_str).unwrap_or(3012)
}

//use std::sync::{Arc, Mutex};

mod webserver;
mod websocket_server;
mod request_wrap;


fn main() {
    env_logger::init().unwrap();

    websocket_server::run_server(get_server_port());
}
