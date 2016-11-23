use std::collections::HashMap;
use std::net::Shutdown;
use std::io::Write;

use websocket::stream::WebSocketStream;
use time;

use rustc_serialize::json;
use rustc_serialize::{Encodable, Encoder};

use request_wrap::RequestWrap;
use webserver;
use waithook_server::SenderAndIp;

fn pretty_json(stats : WaithookStats) -> String {
    let encoder = json::as_pretty_json(&stats);
    format!("{}", encoder)
}

pub struct WaithookStats {
    pub total_listeners: usize,
    pub listeners:       HashMap<String, u32>,
    pub start_time:      time::Tm
}

impl Encodable for WaithookStats {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {

        let hash_len = 3;

        e.emit_map(hash_len, |e| {
            try!(e.emit_map_elt_key(0, |e| "total_listeners".encode(e)));
            try!(e.emit_map_elt_val(0, |e| self.total_listeners.encode(e)));

            try!(e.emit_map_elt_key(1, |e| "listeners".encode(e)));
            try!(e.emit_map_elt_val(1, |e| self.listeners.encode(e)));

            try!(e.emit_map_elt_key(2, |e| "start_time".encode(e)));
            try!(e.emit_map_elt_val(2, |e| self.start_time.strftime("%FT%T.000%z").unwrap().to_string().encode(e)));

            Ok(())
        })
    }
}


pub fn show_stats(request: RequestWrap, mut writter: WebSocketStream, listeners: &Vec<SenderAndIp>, start_time: time::Tm) {
    println!("HTTP {} {}", request.method, request.url);
    let mut listeners_hash : HashMap<String, u32> = HashMap::new();
    for i in 0..listeners.len() {
        if listeners_hash.contains_key(&listeners[i].path) {
            let number = *listeners_hash.get(&listeners[i].path).unwrap();
            listeners_hash.insert(listeners[i].path.clone(), number + 1);
        } else {
            listeners_hash.insert(listeners[i].path.clone(), 1);
        }
        println!("Listener {:?} {:?}", listeners[i].ip, listeners[i].path);
    }
    let stats = WaithookStats {
        total_listeners: listeners.len(),
        listeners: listeners_hash,
        start_time: start_time
    };

    let raw_response = webserver::create_http_response(pretty_json(stats), "Content-Type: application/json");
    match writter.write(raw_response.as_bytes()) {
        Ok(_) => {},
        Err(e) => { println!("HTTP Socket write error: {}", e); }
    }

    writter.flush().unwrap();
    writter.shutdown(Shutdown::Both).unwrap();
}
