use std::thread;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::prelude::*;
use std::io::Error;

pub struct TestServer {
    port: u16
}

impl TestServer {
    pub fn new() -> Result<TestServer, Error> {
        let listener = TcpListener::bind("localhost:0").unwrap();
        let port = listener.local_addr()?.port();

        thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();

                let mut buffer = [0; 512];
                stream.read(&mut buffer).unwrap();

                if buffer.starts_with(b"CLOSE") {
                    break;
                }

                stream.write(b"HTTP/1.1 404 Not Found\r\n").unwrap();
                stream.flush().unwrap();
            }
        });

        Ok(TestServer{ port })
    }

    pub fn port(&self) -> u16 {
       self.port
    }

    pub fn close(&self) -> Result<(), Error> {
        let mut stream = TcpStream::connect(format!("localhost:{}", self.port))?;
        stream.write(b"CLOSE")?;
        stream.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::io::ErrorKind;
    use std::net::TcpStream;
    use super::*;


    #[test]
    fn returns_404_when_requested_enexistent_resource() {
        let server = TestServer::new().unwrap();

        let host = format!("localhost:{}", server.port());
        let mut stream = TcpStream::connect(host).unwrap();
        let request = format!(
            "GET /something HTTP/1.1\r\nHost: http://localhost:{}\r\n\r\n",
            server.port()
        );

        stream.write(request.as_bytes()).unwrap();
        stream.flush().unwrap();

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        assert_eq!(line, "HTTP/1.1 404 Not Found\r\n");
    }

    #[test]
    fn server_should_use_random_port() {
        let server = TestServer::new().unwrap();
        let server_2 = TestServer::new().unwrap();

        assert_ne!(server.port(), server_2.port());
    }

    #[test]
    fn should_close_connection() {
        let server = TestServer::new().unwrap();
        server.close().unwrap();

        let host = format!("localhost:{}", server.port());
        let stream = TcpStream::connect(host);

        assert!(stream.is_err());
        if let Err(e) = stream {
            assert_eq!(e.kind(), ErrorKind::ConnectionRefused);
        }
    }
}
