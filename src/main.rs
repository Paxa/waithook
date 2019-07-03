#[warn(unused_variables)]

extern crate hyper;
extern crate hyper_native_tls;
extern crate env_logger;
extern crate websocket;
extern crate rustc_serialize;
extern crate flate2;
extern crate time;
extern crate url;
extern crate sentry;
extern crate postgres;
extern crate serde_json;

use std::env;
use std::str::FromStr;

/// Look up our server port number in PORT, for compatibility with Heroku.
fn get_server_port() -> u16 {
    let port_str = env::var("PORT").unwrap_or(String::new());
    FromStr::from_str(&port_str).unwrap_or(3012)
}

mod webserver;
mod waithook_server;
mod waithook_saver;
mod waithook_utils;
mod waithook_stats;
mod request_wrap;
mod waithook_forward;
mod hyper_utils;
mod panic_handler;

fn main() {
    env_logger::init();

    if env::var("SENTRY_DSN").is_ok() {
        println!("Registering panic handler...");
        panic_handler::register_sentry();
    }

    waithook_server::run_server(get_server_port());
}
