# Waithook - HTTP to web-socket transmitter

An excelent tool to debug incoming HTTP notifications. Let's say your application need to receive HTTP request from 3rd parties.
It can payment provider or incoming email processing or GitHub push notifications, etc.
Developing it on localhost can be hard, using SSH tunnels always gave me hard times.

## Client Libraries

* https://github.com/Paxa/waithook-ruby

## How it Works

1. Open web-socket connection to http://waithook.com/something (choose your namespace)
2. Send HTTP request to http://waithook.com/something
3. Waithook will forward it as json to all listening web-socket connections

Example of message send to websocket:
```json
{
  "method": "POST",
  "url": "/testing_346?foo=bar",
  "headers": {
    "X-Request-Start": "1478935535042",
    "X-Forwarded-For": "182.253.140.80",
    "Total-Route-Time": "0",
    "Accept": "*/*",
    "Connection": "close",
    "Via": "1.1 vegur",
    "Content-Type": "application/json",
    "Content-Length": "69",
    "Connect-Time": "1",
    "Origin": "https://waithook.com",
    "Referer": "https://waithook.com/",
    "Host": "waithook.com",
    "X-Request-Id": "5d12e9b2-31d1-47b1-abc9-f9bca03dc4bf",
    "X-Forwarded-Proto": "https",
    "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/54.0.2840.59 Safari/537.36",
    "Accept-Language": "en-US,en;q=0.8",
    "X-Forwarded-Port": "443",
    "Accept-Encoding": "gzip, deflate, br"
  },
  "body": "{\n  \"type\": \"Testing Request\",\n  \"time\": \"2016-11-12T07:25:34.280Z\"\n}"
}
```

## URL Options

* `forward=<url>` - will forward each incoming request to specified URL
* `resp=<body>` - custom response body, default = `OK`
* `resp_type=<type>` - content type for custom response body, default: `text/plain`,
    has shortcuts: for `json`, `xml`, `html`, or use full value as: `resp_type=text/csv`

## Development

```
cargo run
```

## How to Run

build with `cargo build`

Run binary `target/debug/waithook`

Env variables: (all optional)

* `PORT`
* `DATABASE_URL`
* `SENTRY_DSN`


Nginx config
```
server {
    client_max_body_size 100M;
    server_name waithook.ext.io;
    listen 443 ssl http2;
    ssl_certificate /opt/waithook.ext.io.cer;
    ssl_certificate_key /opt/waithook.ext.io.key;

    location / {
        proxy_pass http://localhost:3012;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto https;
        proxy_read_timeout 60;
        proxy_connect_timeout 60;
        proxy_set_header X-Real-IP $remote_addr;

        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $http_connection;
    }
}
```

**Persistance**

it can save reqeust history in database, need to have postgres with table `requests`

Run server with `DATABASE_URL=postgres://user:pass@localhost/waithook`

Table structure:

```sql
CREATE TABLE requests (
  id serial primary key,
  method varchar,
  url varchar,
  body varchar,
  headers jsonb,
  sender_address varchar,
  created_at timestamp not null
); 
```