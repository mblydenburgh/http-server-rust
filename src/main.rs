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

fn construct_headers(headers: Vec<(String, String)>) -> String {
    let res_headers: Vec<String> = headers
        .iter()
        .map(|h| format!("{}: {}", h.0, h.1))
        .collect();
    res_headers.join("\r\n") + "\r\n"
}

fn construct_response(
    code: StatusCode,
    headers: Option<Vec<(String, String)>>,
    body: Option<String>,
) -> Result<String, anyhow::Error> {
    let status = String::from(format!("HTTP/1.1 {}\r\n", code.as_string()));
    let headers_str = match headers {
        Some(headers) => &construct_headers(headers),
        None => "\r\n",
    };
    let body = match body {
        Some(b) => String::from(b),
        None => String::from(""),
    };
    Ok(status + &headers_str + &body)
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
                println!("{:?}", request);
                let _ = request.method;
                let _ = request.version;
                let mut path_parts: Vec<&str> = request.path.split("/").collect();
                path_parts.remove(0);
                let root = path_parts[0];
                println!("root: {}", root);
                let response = match root {
                    "echo" => construct_response(
                        StatusCode::Ok(),
                        Some(vec![
                            ("Content-Type".into(), "text/plain".into()),
                            ("Content-Length".into(), path_parts[1].len().to_string()),
                        ]),
                        Some(path_parts[1].into()),
                    ),
                    _ => construct_response(StatusCode::NotFound(), None, None),
                };
                println!("build response: {:?}", response);
                if let Ok(res_str) = response {
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
