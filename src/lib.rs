pub mod resource;

use std::thread;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::prelude::*;
use std::io::Error;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;

use resource::Resource;

pub struct TestServer {
    port: u16,
    resources: Arc<Mutex<HashMap<String, Arc<Resource>>>>
}

impl TestServer {
    pub fn new() -> Result<TestServer, Error> {
        let listener = TcpListener::bind("localhost:0").unwrap();
        let port = listener.local_addr()?.port();
        let resources = Arc::new(Mutex::new(HashMap::new()));

        let res = Arc::clone(&resources);

        thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();

                let mut buffer = [0; 512];
                stream.peek(&mut buffer).unwrap();

                if buffer.starts_with(b"CLOSE") {
                    break;
                }

                handle_client(&mut stream, res.clone());
            }
        });

        Ok(TestServer{ port, resources })
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

    pub fn create_resource(&self, uri: &str) -> Arc<Resource> {
        let mut resources = self.resources.lock().unwrap();
        let resource = Arc::new(Resource::new());
        resources.insert(String::from(uri), resource.clone());
        resource
    }
}

fn handle_client(stream: &mut TcpStream, resources: Arc<Mutex<HashMap<String, Arc<Resource>>>>) {
    let resources = resources.lock().unwrap();

    let mut buffer = [0; 512];
    stream.read(&mut buffer).unwrap();

    if let Some(resource) = resources.get("/something") {
        let response = format!(
            "HTTP/1.1 {}\r\n",
            resource.get_status_description()
        );

        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    } else {
        stream.write(b"HTTP/1.1 404 Not Found\r\n").unwrap();
        stream.flush().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::io::ErrorKind;
    use std::net::TcpStream;
    use std::time::Duration;
    use super::*;

    fn make_request(port: u16, uri: &str) -> TcpStream {
        let host = format!("localhost:{}", port);
        let mut stream = TcpStream::connect(host).unwrap();
        let request = format!(
            "GET {} HTTP/1.1\r\n\r\n",
            uri
        );

        stream.write(request.as_bytes()).unwrap();
        stream.flush().unwrap();

        stream
    }

    #[test]
    fn returns_404_when_requested_enexistent_resource() {
        let server = TestServer::new().unwrap();
        let stream = make_request(server.port(), "/something");

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        assert_eq!(line, "HTTP/1.1 404 Not Found\r\n");
        server.close().unwrap();
    }

    #[test]
    fn server_should_use_random_port() {
        let server = TestServer::new().unwrap();
        let server_2 = TestServer::new().unwrap();

        assert_ne!(server.port(), server_2.port());

        server.close().unwrap();
        server_2.close().unwrap();
    }

    #[test]
    fn should_close_connection() {
        let server = TestServer::new().unwrap();
        server.close().unwrap();

        thread::sleep(Duration::from_millis(200));

        let host = format!("localhost:{}", server.port());
        let stream = TcpStream::connect(host);

        assert!(stream.is_err());
        if let Err(e) = stream {
            assert_eq!(e.kind(), ErrorKind::ConnectionRefused);
        }
    }

    #[test]
    fn should_create_resource() {
        let server = TestServer::new().unwrap();
        server.create_resource("/something");

        let stream = make_request(server.port(), "/something");

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        assert_eq!(line, "HTTP/1.1 204 No Content\r\n");
        server.close().unwrap();
    }

    #[test]
    fn should_return_configured_status_for_resource_resource() {
        let server = TestServer::new().unwrap();
        let resource = server.create_resource("/something");

        resource.status(200);

        let stream = make_request(server.port(), "/something");

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        assert_eq!(line, "HTTP/1.1 200 Ok\r\n");
        server.close().unwrap();
    }
}
