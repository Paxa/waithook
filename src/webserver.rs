use std::sync::mpsc::Sender;
use std::net::Shutdown;
use std::io::{self, Write, Read};
use std::net::TcpStream;
use std::fs::File;
use std::str;
use url::Url;
use rustc_serialize::json::Json;
use flate2::Compression;
use flate2::write::ZlibEncoder;

use request_wrap::RequestWrap;

fn get_file_body(filepath: &str) -> Result<String, io::Error> {
    let filename = format!("public/{}", filepath.replace("..", ""));

    /*
    if filepath.starts_with("/") {
        return Err(io::Error::new(io::ErrorKind::Other, "За тобой уже выехали!"))
    }
    */

    println!("Reading {}", filename);
    let mut file = try!(File::open(filename));
    let mut body = String::new();
    try!(file.read_to_string(&mut body));


    Ok(body)
}

/*
pub fn create_http_response(body: String, extra_headers: &str) -> String {
    create_http_response(body, extra_headers, false)
}
*/

pub fn create_http_response(body: String, extra_headers: &str, compress: bool) -> Vec<u8> {
    let mut gzip_header = "";

    let body_bytes = if compress {
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        gzip_header = "Content-Encoding: deflate\r\n";
        match e.write(body.as_bytes()) {
            Ok(_) => {},
            Err(e) => { println!("Compression error: {}", e); }
        }
        e.finish().unwrap()
    } else {
        body.into_bytes()
    };

    //println!("BODY: {:?}", body_bytes);

    let mut result = if extra_headers == "" {
        format!("{}\r\n{}{}\r\n{}: {}\r\n\r\n",
            "HTTP/1.1 200 OK",
            gzip_header,
            "Connection: close",
            "Content-Length", body_bytes.len()
        ).into_bytes()
    } else {
        format!("{}\r\n{}{}\r\n{}: {}\r\n{}\r\n\r\n",
            "HTTP/1.1 200 OK",
            gzip_header,
            "Connection: close",
            "Content-Length", body_bytes.len(),
            extra_headers
        ).into_bytes()
    };

    result.extend(body_bytes);
    result
}

pub fn create_default_reponse() -> Vec<u8> {
    create_http_response("OK\n".to_string(), "Content-Type: text/plain", false)
}

pub fn handle(request: RequestWrap, mut writer: TcpStream, sender: Sender<RequestWrap>) {
    println!("HTTP {} {}", request.method, request.url);
    let enable_compression = request.support_gzip();

    let raw_response = if request.url == "/" || request.url.starts_with("/?") {
        let body = match get_file_body("index.html") {
            Ok(b) => b,
            Err(e) => {
                format!("Read file error: {}", e)
            }
        };

        create_http_response(body, "Content-Type: text/html", enable_compression)
    } else if request.url.starts_with("/@/") {
        let (_, filename) = request.url.split_at(3);
        let body = match get_file_body(filename) {
            Ok(b) => b,
            Err(e) => {
                format!("Read file error: {}", e)
            }
        };

        let mut content_type = "text/plain";

        if filename.ends_with(".css") {
            content_type = "text/css"
        } else if filename.ends_with(".js") {
            content_type = "application/javascript; charset=utf-8"
        } else if filename.ends_with(".html") {
            content_type = "text/html"
        } else if filename.ends_with(".ico") {
            content_type = "image/x-icon"
        }

        create_http_response(body, &format!("Content-Type: {}", content_type), enable_compression)
    } else {
        let req_body = request.body.clone();
        let req_url  = request.url.clone();
        match sender.send(request) {
            Ok(_) => {},
            Err(e) => { println!("HTTP Channel send error: {}", e); }
        }

        if req_url.contains("type=slack") {
            match Json::from_str(&req_body) {
                Ok (json) => {
                    match json.find("challenge") {
                        Some(body_value) => {
                            if body_value.is_string() {
                                create_http_response(body_value.as_string().unwrap().to_string(), "Content-Type: text/plain", false)
                            } else {
                                create_http_response("OK\n".to_string(), "Content-Type: text/plain", false)
                            }
                        },
                        None => {
                            println!("Slack message don't have 'challenge'");
                            create_http_response("OK\n".to_string(), "Content-Type: text/plain", false)
                        }
                    }
                },
                Err(e) => {
                    println!("Error parsing slack message: {}", e);
                    create_http_response("OK\n".to_string(), "Content-Type: text/plain", false)
                }
            }
        } else if req_url.contains("resp=") {
            match Url::parse(&format!("http://example.com{}", &req_url)) {
                Ok(url) => {
                    let resp_arg = url.query_pairs().find(|ref x| x.0 == "resp" );
                    let resp_type = url.query_pairs().find(|ref x| x.0 == "resp_type" );

                    let arg_copy;
                    let content_type = match resp_type {
                        Some(value) => {
                            arg_copy = value.1.into_owned();
                            match arg_copy.as_str() {
                                "json" => "application/json",
                                "xml"  => "application/xml",
                                "html" => "text/html",
                                _      => arg_copy.as_str()
                            }
                        },
                        None => {
                            "text/plain"
                        }
                    };

                    match resp_arg {
                        Some(value) => {
                            create_http_response(value.1.into_owned(), &format!("Content-Type: {}", content_type), false)
                        },
                        None => {
                            create_default_reponse()
                        }
                    }
                },
                Err(e) => {
                    println!("Error parsing resp param: {}", e);
                    create_default_reponse()
                }
            }
        } else {
            create_http_response("OK\n".to_string(), "Content-Type: text/plain", false)
        }
    };

    match writer.write(raw_response.as_slice()) {
        Ok(_) => {},
        Err(e) => { println!("HTTP Socket write error: {}", e); }
    }

    writer.flush().unwrap();
    writer.shutdown(Shutdown::Both).unwrap();
}
