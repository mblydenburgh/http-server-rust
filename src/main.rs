#[allow(unused_imports)]
use std::net::TcpListener;
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
};

#[derive(Debug)]
enum StatusCode {
    Ok(),
    Created(),
    NotFound(),
    MethodNotAllowed(),
    ServerError(),
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
            StatusCode::Created() => "201 Created".into(),
            StatusCode::NotFound() => "404 Not Found".into(),
            Self::MethodNotAllowed() => "405 Method Not Allowed".into(),
            Self::ServerError() => "500 Server Error".into(),
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

fn parse_request(mut stream: &TcpStream) -> Result<RawHttpRequest, anyhow::Error> {
    let mut buffer = [0; 4096];
    let req_bytes = stream.read(&mut buffer).expect("could not read stream");
    if req_bytes == 0 {
        return Err(anyhow::anyhow!("could not read request bytes"));
    }
    let req_str = String::from_utf8_lossy(&buffer[..req_bytes]);
    let mut lines = req_str.lines();
    let request_line = lines.next().ok_or(anyhow::anyhow!("no request line"))?;
    let request_line_parts: Vec<String> = request_line
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    if request_line_parts.len() < 3 {
        return Err(anyhow::anyhow!("Invalid request line"));
    }

    let mut headers: Vec<Header> = vec![];
    let mut content_length = 0;

    for line in lines.by_ref() {
        if line.is_empty() {
            break;
        }
        let header_parts: Vec<&str> = line.split_whitespace().collect();
        println!("header parts: {:?}", header_parts);
        headers.push(Header {
            key: header_parts[0].replace(":", "").into(),
            value: header_parts[1].into(),
        });
        if header_parts[0].eq_ignore_ascii_case("content-length") {
            content_length = header_parts[1].parse().unwrap_or(0);
        }
    }

    let body_start_index = req_str.find("\r\n\r\n").unwrap_or(0) + 4;
    println!("body start: {}", body_start_index);
    let body = if body_start_index > request_line.len() {
        req_str[body_start_index..].to_string()
    } else {
        String::new()
    };
    println!("parsed body: {}", body);

    let body = if content_length > 0 && body.len() > content_length {
        body[..content_length].to_string()
    } else {
        body
    };

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
        body,
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
        "files" => match request.method.as_str() {
            "GET" => get_file(request),
            "POST" => post_file(request),
            _ => construct_response(StatusCode::MethodNotAllowed(), None, None),
        },
        "" => construct_response(StatusCode::Ok(), None, None),
        _ => construct_response(StatusCode::NotFound(), None, None),
    };

    if let Ok(res_str) = response {
        let response = res_str.as_bytes();
        let result = stream.write_all(response);
        if let Err(e) = result {
            let err = construct_response(StatusCode::ServerError(), None, Some(e.to_string()));
            let _ = stream.write_all(err.unwrap().as_bytes());
        }
    } else {
        let err = construct_response(StatusCode::ServerError(), None, None);
        let _ = stream.write_all(err.unwrap().as_bytes());
    }
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

fn echo(request: RawHttpRequest) -> Result<String, anyhow::Error> {
    let encoding_header = request.headers.iter().find(|h| h.key == "Accept-Encoding");
    let headers = match encoding_header {
        Some(h) => {
            let mut default = vec![
                ("Content-Type".into(), "text/plain".into()),
                (
                    "Content-Length".into(),
                    request.path_parts[1].len().to_string(),
                ),
            ];
            if h.value == "gzip" {
                default.push(("Content-Encoding".into(), "gzip".into()));
            }
            default
        }
        None => vec![
            ("Content-Type".into(), "text/plain".into()),
            (
                "Content-Length".into(),
                request.path_parts[1].len().to_string(),
            ),
        ],
    };
    construct_response(
        StatusCode::Ok(),
        Some(headers),
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

fn get_file(request: RawHttpRequest) -> Result<String, anyhow::Error> {
    let file_name = format!("{}", request.path_parts[1].clone());
    // get directory from ./your_program.sh --directory /tmp/
    let argv = std::env::args().collect::<Vec<String>>();
    let dir = argv[2].clone();
    if let Ok(mut file) = fs::File::open(format!("{dir}{file_name}")) {
        let mut body = String::new();
        file.read_to_string(&mut body).unwrap();
        let content_length = body.len().to_string();
        construct_response(
            StatusCode::Ok(),
            Some(vec![
                ("Content-Type".into(), "application/octet-stream".into()),
                ("Content-Length".into(), content_length),
            ]),
            Some(body),
        )
    } else {
        construct_response(StatusCode::NotFound(), None, None)
    }
}

fn post_file(request: RawHttpRequest) -> Result<String, anyhow::Error> {
    println!("posting file {}", request.path_parts[1].clone());
    println!("body contents: {}", request.body);
    let file_name = format!("{}", request.path_parts[1].clone());
    // get directory from ./your_program.sh --directory /tmp/some/dir/
    let argv = std::env::args().collect::<Vec<String>>();
    let dir = argv[2].clone(); // /tmp/some/dir/

    let name = format!("{dir}{file_name}");

    let body_length = request
        .headers
        .iter()
        .find(|h| h.key == "Content-Length")
        .expect("No Content-Length header")
        .value
        .clone()
        .parse::<usize>()
        .unwrap();

    match fs::write(name, &request.body.as_bytes()) {
        Ok(_) => {
            println!("wrote file");
            construct_response(StatusCode::Created(), None, None)
        }
        Err(_) => {
            println!("could not write file");
            construct_response(StatusCode::ServerError(), None, None)
        }
    }
}
