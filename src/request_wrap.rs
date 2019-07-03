use hyper::header::{self, Headers, Encoding, qitem};

use std::fmt;
use std::clone::Clone;
use std::net::SocketAddr;

use std::collections::HashMap;
use rustc_serialize::{Encodable, Encoder};
use std::time::Instant;

pub struct RequestWrap {
    pub method: String,
    pub url: String,
    pub headers: Headers,
    pub body: String,
    pub client_ip: SocketAddr,
    pub time: Instant
}

impl RequestWrap {
    pub fn support_gzip(&self) -> bool {
        match self.headers.get() {
            Some(&header::AcceptEncoding(ref accept_encoding)) => {
                accept_encoding.contains(&qitem(Encoding::Deflate))
            }
            None => {
                false
            }
        }
    }
}

impl Encodable for RequestWrap {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {

        let hash_len = if self.body == "" { 3 } else { 4 };

        e.emit_map(hash_len, |e| {
            try!(e.emit_map_elt_key(0, |e| "method".encode(e)));
            try!(e.emit_map_elt_val(0, |e| self.method.encode(e)));

            try!(e.emit_map_elt_key(1, |e| "url".encode(e)));
            try!(e.emit_map_elt_val(1, |e| self.url.encode(e)));

            let mut headers_hash : HashMap<String, String> = HashMap::new();

            for header in self.headers.iter() {
                headers_hash.insert(header.name().to_string(), header.value_string().to_string());
            }

            try!(e.emit_map_elt_key(2, |e| "headers".encode(e)));
            try!(e.emit_map_elt_val(2, |e| headers_hash.encode(e)));

            if self.body != "" {
                try!(e.emit_map_elt_key(3, |e| "body".encode(e)));
                try!(e.emit_map_elt_val(3, |e| self.body.encode(e)));
            }
            Ok(())
        })
    }
}

impl fmt::Debug for RequestWrap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RequestWrap {{ method: {} url: {}, body: {} }}", self.method, self.url, self.body)
    }
}

impl Clone for RequestWrap {
    fn clone(&self) -> RequestWrap {
        RequestWrap {
            method:     self.method.clone(),
            url:        self.url.clone(),
            headers:    self.headers.clone(),
            body:       self.body.clone(),
            client_ip:  self.client_ip,
            time:       self.time
        }
    }
}

//unsafe impl Sync for RequestWrap {}
//unsafe impl Send for RequestWrap {}