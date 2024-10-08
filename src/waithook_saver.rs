use std::thread;
use std::env;
use std::sync::mpsc::Receiver;
use std::collections::HashMap;

use request_wrap::RequestWrap;

use postgres::{Client, NoTls};
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
            let mut conn = match Client::connect(env::var("DATABASE_URL").unwrap().as_str(), NoTls) {
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
                        let sql_res = conn.execute(&insert_stmt, &[
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
        let mut conn = match Client::connect(env::var("DATABASE_URL").unwrap().as_str(), NoTls) {
            Ok(c) => c,
            Err(e) => {
                println!("Saver: Connection Error: {:?}", e);
                return "[]".to_string();
            }
        };

        let res = conn.query("SELECT COALESCE(json_agg(requests), '[]'::json)::text as json_rows FROM requests where url = $1 OR url like $2 GROUP BY id ORDER BY id LIMIT 100", &[
            &format!("/{}", path),
            &format!("/{}?%", path)
        ]);

        match res {
            Ok (res_data) => {
                // result rows contain json with array of actual row
                // row 1 - '[{"id" 1, ...}]'
                // row 1 - '[{"id" 2, ...}]'
                // we asseble array of all those objects
                // result - '[{"id" 1, ...}, {"id" 2, ...}]'
                let mut json_rows = "[".to_string();
                for (i, row) in res_data.iter().enumerate() {
                    let value: String = row.get("json_rows");
                    json_rows.push_str(&value[1..value.len() - 1]);
                    if i < res_data.len() - 1 {
                        json_rows.push_str(",");
                    }
                }
                json_rows.push_str("]");
                return json_rows;
            },
            Err(e) => {
                println!("Error querying history: {:?}", e);
                return "[]".to_string();
            }
        }

    } else {
        return "[]".to_string();
    }
}