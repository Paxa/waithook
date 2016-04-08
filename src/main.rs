#[warn(unused_variables)]

extern crate hyper;
extern crate env_logger;
//extern crate handlebars_iron as hbs;
extern crate websocket;
extern crate rustc_serialize;

//use std::sync::{Arc, Mutex};

mod webserver;
mod websocket_s;
mod request_wrap;


fn main() {
    env_logger::init().unwrap();

    websocket_s::run_server();
}
