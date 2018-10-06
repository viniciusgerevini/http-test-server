use std::thread;
use std::net::TcpListener;
use std::io::prelude::*;

pub struct TestServer {
}

impl TestServer {
    pub fn new() -> TestServer {
        thread::spawn(|| {
            let listener = TcpListener::bind("localhost:1234").unwrap();

            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                stream.write(b"HTTP/1.1 404 Not Found\r\n").unwrap();
                stream.flush().unwrap();
            }
        });
        return TestServer{}
    }

    pub fn port(&self) -> u32 {
       1234
    }
}

#[cfg(test)]
mod tests {
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::net::TcpStream;
    use super::*;


    #[test]
    fn returns_404_when_requested_enexistent_resource() {
        let server = TestServer::new();

        let host = format!("localhost:{}", server.port());
        let mut stream = TcpStream::connect(host).unwrap();
        let request = format!(
            "GET /something HTTP/1.1\r\nAccept: text/event-stream\r\nHost: http://localhost:{}\r\n\r\n",
            server.port()
        );

        stream.write(request.as_bytes()).unwrap();
        stream.flush().unwrap();

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        assert_eq!(line, "HTTP/1.1 404 Not Found\r\n");
    }
}
