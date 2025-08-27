use std::io::Write;
#[allow(unused_imports)]
use std::net::TcpListener;

fn construct_response() -> Result<String, anyhow::Error> {
    let status = String::from("HTTP/1.1 200 OK\r\n");
    let headers = String::from("\r\n");
    let body = String::from("\r\n");
    Ok(status + &headers + &body)
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                //let reader = BufReader::new(&stream);
                if let Ok(res_str) = construct_response() {
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
