#[allow(unused_imports)]
use std::net::TcpListener;
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

#[derive(Debug)]
enum StatusCode {
    Ok(),
    NotFound(),
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl StatusCode {
    fn as_string(&self) -> String {
        match self {
            StatusCode::Ok() => "200 OK".into(),
            StatusCode::NotFound() => "404 Not Found".into(),
        }
    }
}

#[derive(Debug)]
struct RawHttpRequest {
    method: String,
    path: String,
    version: String,
}

fn construct_response(code: StatusCode) -> Result<String, anyhow::Error> {
    let status = String::from(format!("HTTP/1.1 {}\r\n", code.as_string()));
    let headers = String::from("\r\n");
    let body = String::from("");
    Ok(status + &headers + &body)
}

fn parse_request(stream: TcpStream) -> Result<RawHttpRequest, anyhow::Error> {
    let reader = BufReader::new(&stream);
    let request: Vec<_> = reader
        .lines()
        .map(|line| line.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
    let request_line = request.get(0);
    match request_line {
        Some(line) => {
            let request_parts: Vec<&str> = line.split(" ").collect();
            let parsed_req = RawHttpRequest {
                method: String::from(request_parts[0]),
                path: String::from(request_parts[1]),
                version: String::from(request_parts[2]),
            };
            Ok(parsed_req)
        }
        None => panic!("could not parse"),
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let request = parse_request(stream.try_clone().unwrap()).unwrap();
                let _ = request.method;
                let _ = request.version;
                let status = if request.path != "/" {
                    StatusCode::NotFound()
                } else {
                    StatusCode::Ok()
                };
                if let Ok(res_str) = construct_response(status) {
                    let response = res_str.as_bytes();
                    let _ = stream.write_all(response);
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
