use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use synt_testkit::TestError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Received {
    pub method: String,
    pub path: String,
    pub authorization: Option<String>,
    pub body: String,
}

#[derive(Clone)]
pub struct EchoServer {
    port: u16,
    received: Arc<Mutex<Vec<Received>>>,
}

impl EchoServer {
    pub fn start() -> Result<Self, TestError> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let received: Arc<Mutex<Vec<Received>>> = Arc::default();

        let recording = Arc::clone(&received);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = answer(stream, &recording);
            }
        });

        Ok(Self { port, received })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    pub fn received(&self) -> Vec<Received> {
        self.received
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

fn answer(stream: TcpStream, received: &Mutex<Vec<Received>>) -> Result<(), TestError> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let request = read_request(&mut reader)?;

    let echoed = request.authorization.clone().unwrap_or_default();
    let body = format!(
        "{} {} auth={echoed} body={}",
        request.method, request.path, request.body
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nX-Echo-Authorization: {echoed}\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    received
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(request);

    let mut stream = stream;
    let _ = stream.write_all(response.as_bytes());

    Ok(())
}

fn read_request(reader: &mut BufReader<TcpStream>) -> Result<Received, TestError> {
    let mut first = String::new();
    reader.read_line(&mut first)?;
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let path = parts.next().unwrap_or_default().to_owned();

    let mut authorization = None;
    let mut length = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }

        let (name, value) = header.split_once(':').unwrap_or((header, ""));
        match name.to_ascii_lowercase().as_str() {
            "authorization" => authorization = Some(value.trim().to_owned()),
            "content-length" => length = value.trim().parse().unwrap_or(0),
            _ => {}
        }
    }

    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;

    Ok(Received {
        method,
        path,
        authorization,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}
