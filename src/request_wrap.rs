use hyper::header;

use std::fmt;
use std::clone::Clone;
//use rustc_serialize::json;
//use rustc_serialize::Encodable;

//#[derive(RustcEncodable)]
pub struct RequestWrap {
    pub method: String,
    pub url: String,
    pub headers: header::Headers,
    pub body: String,
}

/*
impl Encodable for header::Headers {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_usize(*self)
    }
}
*/

impl fmt::Debug for RequestWrap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RequestWrap {{ method: {} url: {}, body: {} }}", self.method, self.url, self.body)
    }
}

impl Clone for RequestWrap {
    fn clone(&self) -> RequestWrap {
        RequestWrap {
            method: self.method.clone(),
            url: self.url.clone(),
            headers: self.headers.clone(),
            body: self.body.clone()
        }
    }
}

//unsafe impl Sync for RequestWrap {}
//unsafe impl Send for RequestWrap {}