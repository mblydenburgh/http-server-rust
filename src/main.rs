#[allow(unused_imports)]
use std::net::TcpListener;
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
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
struct Header {
    key: String,
    value: String,
}

impl FromIterator<(String, String)> for Header {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        iter.into_iter().map(|i| (i.0, i.1)).collect()
    }
}

#[derive(Debug)]
struct RawHttpRequest {
    method: String,
    path: String,
    path_parts: Vec<String>,
    version: String,
    headers: Vec<Header>,
    body: String,
}

fn construct_headers(headers: Vec<(String, String)>) -> String {
    if headers.is_empty() {
        return String::new();
    }
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
    let status = String::from(format!("HTTP/1.1 {}", code.as_string()));
    let headers_str = match headers {
        Some(headers) => &construct_headers(headers),
        None => "\r\n",
    };
    let body = match body {
        Some(b) => String::from(b),
        None => String::from(""),
    };
    Ok(status + "\r\n" + &headers_str + "\r\n" + &body)
}

fn parse_request(stream: &TcpStream) -> Result<RawHttpRequest, anyhow::Error> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let request_line_parts: Vec<String> = request_line
        .clone()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    if request_line_parts.len() < 3 {
        return Err(anyhow::anyhow!("Invalid request line"));
    }

    let mut headers: Vec<Header> = vec![];

    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line.starts_with("\r\n") {
            break;
        }
        let header_parts: Vec<&str> = line.split_whitespace().collect();
        headers.push(Header {
            key: header_parts[0].replace(":", "").into(),
            value: header_parts[1].into(),
        });
    }

    let mut path_parts: Vec<String> = request_line_parts[1]
        .clone()
        .split("/")
        .map(|s| s.to_string())
        .collect();
    path_parts.remove(0); // first part is "/"
    Ok(RawHttpRequest {
        method: String::from(&request_line_parts[0]),
        path: String::from(&request_line_parts[1]),
        path_parts,
        version: String::from(&request_line_parts[2]),
        headers,
        body: String::new(),
    })
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
                    handle_request(stream);
                });
            }
            Err(e) => {
                println!("Error reading TCPStream: {}", e);
            }
        }
    }
}

fn handle_request(mut stream: TcpStream) {
    let request = parse_request(&stream).unwrap();
    println!("{:?}", request);

    let _ = request.method;
    let _ = request.version;

    let root = request.path_parts[0].clone();
    println!("handling request for: /{}", root);
    let response = match root.as_str() {
        "echo" => echo(request),
        "user-agent" => user_agent(request),
        "files" => file(request),
        "" => construct_response(StatusCode::Ok(), None, None),
        _ => construct_response(StatusCode::NotFound(), None, None),
    };

    if let Ok(res_str) = response {
        let response = res_str.as_bytes();
        let result = stream.write_all(response);
        let _ = stream.flush();
        if let Err(e) = result {
            panic!("ahhh! {:?}?", e);
        }
    } else {
        println!("could not build response");
    }
}

fn echo(request: RawHttpRequest) -> Result<String, anyhow::Error> {
    construct_response(
        StatusCode::Ok(),
        Some(vec![
            ("Content-Type".into(), "text/plain".into()),
            (
                "Content-Length".into(),
                request.path_parts[1].len().to_string(),
            ),
        ]),
        Some(request.path_parts[1].clone().into()),
    )
}

fn user_agent(request: RawHttpRequest) -> Result<String, anyhow::Error> {
    let user_agent: Option<&Header> = request
        .headers
        .iter()
        .clone()
        .find(|h| h.key == "User-Agent");
    let content_length = user_agent.unwrap().value.clone().len().to_string();
    let body = user_agent.unwrap().value.clone().to_string();
    construct_response(
        StatusCode::Ok(),
        Some(vec![
            ("Content-Type".into(), "text/plain".into()),
            ("Content-Length".into(), content_length),
        ]),
        Some(body),
    )
}

fn file(request: RawHttpRequest) -> Result<String, anyhow::Error> {
    let file_name = format!("{}", request.path_parts[1].clone());
    let argv = std::env::args().collect::<Vec<String>>();
    let dir = argv[2].clone();
    if let Ok(mut file) = fs::File::open(format!("{dir}{file_name}")) {
        println!("found");
        let mut body = String::new();
        file.read_to_string(&mut body).unwrap();
        let content_length = body.len().to_string();
        println!("body: {}", body);
        construct_response(
            StatusCode::Ok(),
            Some(vec![
                ("Content-Type".into(), "application/octet-stream".into()),
                ("Content-Length".into(), content_length),
            ]),
            Some(body),
        )
    } else {
        println!("not found");
        construct_response(StatusCode::NotFound(), None, None)
    }
}
