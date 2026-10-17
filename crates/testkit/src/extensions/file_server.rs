use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

#[derive(Clone)]
pub struct FileServer {
    address: String,
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl FileServer {
    pub fn start() -> io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = format!("http://{}", listener.local_addr()?);
        let files: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::default();

        let serving = Arc::clone(&files);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = answer(stream, &serving);
            }
        });

        Ok(Self { address, files })
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn put(&self, name: &str, bytes: Vec<u8>) {
        self.files
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(format!("/{name}"), bytes);
    }
}

fn answer(mut stream: TcpStream, files: &Mutex<HashMap<String, Vec<u8>>>) -> io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;

    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? <= 2 {
            break;
        }
    }

    let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
    let body = files
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&path)
        .cloned();

    let response = match body {
        Some(body) => [
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes(),
            body,
        ]
        .concat(),
        None => {
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
        }
    };

    stream.write_all(&response)
}
