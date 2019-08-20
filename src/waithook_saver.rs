use std::thread;
use std::env;
use std::sync::mpsc::Receiver;
use std::collections::HashMap;

use request_wrap::RequestWrap;

use postgres::{Connection, TlsMode};
//use rustc_serialize::{Encodable, Encoder, json};
use hyper::header::Headers;

fn headers_json(headers: Headers) -> serde_json::Value {
    let mut headers_hash : HashMap<String, String> = HashMap::new();

    for header in headers.iter() {
        headers_hash.insert(header.name().to_string(), header.value_string().to_string());
    }

    serde_json::to_value(headers_hash).unwrap()
}

pub fn saver_enabled() -> bool {
    env::var("DATABASE_URL").is_ok()
}

pub fn run_request_saver(reciever: Receiver<RequestWrap>) {
    if env::var("DATABASE_URL").is_ok() {
        thread::spawn(move || {
            let conn = match Connection::connect(env::var("DATABASE_URL").unwrap(), TlsMode::None) {
                Ok(c) => c,
                Err(e) => {
                    println!("Saver: Connection Error: {:?}", e);
                    return;
                }
            };
            let insert_stmt = conn.prepare("INSERT INTO requests \
                (method, url, body, headers, sender_address, created_at) \
                VALUES ($1, $2, $3, $4, $5, NOW())").unwrap();

            loop {
                let request = reciever.recv();

                match request {
                    Ok(request) => {
                        let sql_res = insert_stmt.execute(&[
                            &request.method,
                            &request.url,
                            &request.body,
                            &headers_json(request.headers),
                            &format!("{}", request.client_ip)
                        ]);
                        match sql_res {
                            Ok(res) => {
                                println!("Saver: Inserted {:?}", res);
                            },
                            Err(err) => {
                                println!("Saver: Insert Error {:?}", err);
                            }
                        }
                    },
                    Err(e) => {
                        println!("Saver: Recieve Error: {}", e);
                    }
                }
            }
        });
    } else {
        println!("Saver: No DATABASE_URL defined");
    }
}

pub fn get_history(path: String) -> String {
    if env::var("DATABASE_URL").is_ok() {
        let conn = match Connection::connect(env::var("DATABASE_URL").unwrap(), TlsMode::None) {
            Ok(c) => c,
            Err(e) => {
                println!("Saver: Connection Error: {:?}", e);
                return "[]".to_string();
            }
        };

        let res = conn.query("SELECT COALESCE(json_agg(requests), '[]'::json)::text as json_rows FROM requests where url = $1 OR url like $2 ORDER BY id", &[
            &format!("/{}", path),
            &format!("/{}?%", path)
        ]).unwrap();

        let json_rows: String = res.get(0).get("json_rows");
        return json_rows;
    } else {
        return "[]".to_string();
    }
}