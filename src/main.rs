use std::io::{BufRead, BufReader, Read, Write};
#[allow(unused_imports)]
use std::net::TcpListener;

#[derive(Debug)]
enum StatusCode {
    Ok(String),
    NotFound(String),
}

fn construct_response(code: StatusCode) -> Result<String, anyhow::Error> {
    let status = String::from(format!("HTTP/1.1 {:?} OK\r\n", code));
    let headers = String::from("\r\n");
    let body = String::from("\r\n");
    Ok(status + &headers + &body)
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let reader = BufReader::new(&stream);
                let request: Vec<_> = reader
                    .lines()
                    .map(|line| line.unwrap())
                    .take_while(|line| !line.is_empty())
                    .collect();
                for line in request {
                    println!("Line: {}", line);
                }
                if let Ok(res_str) = construct_response(StatusCode::Ok("200 OK".into())) {
                    let response = res_str.as_bytes();
                    stream.write_all(response);
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
