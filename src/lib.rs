pub mod resource;

use std::thread;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::prelude::*;
use std::io::Error;
use std::io::BufReader;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
use resource::Resource;

type ServerResources = Arc<Mutex<HashMap<String, Vec<Arc<Resource>>>>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH
}

impl Method {
    fn value(&self) -> &'static str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
            Method::PATCH => "PATCH"
        }
    }
}

pub struct TestServer {
    port: u16,
    resources: ServerResources
}

impl TestServer {
    pub fn new() -> Result<TestServer, Error> {
        let listener = TcpListener::bind("localhost:0").unwrap();
        let port = listener.local_addr()?.port();
        let resources: ServerResources = Arc::new(Mutex::new(HashMap::new()));

        let res = Arc::clone(&resources);

        thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();

                let mut buffer = [0; 512];
                stream.peek(&mut buffer).unwrap();

                if buffer.starts_with(b"CLOSE") {
                    break;
                }

                handle_client(&stream, res.clone());
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
        resources.insert(String::from(uri), vec!(resource.clone()));

        resource
    }
}

fn handle_client(stream: &TcpStream, resources: ServerResources) {
    let stream = stream.try_clone().unwrap();

    thread::spawn(move || {
        let mut write_stream = stream.try_clone().unwrap();
        let mut reader = BufReader::new(stream);

        let mut request_header = String::from("");
        reader.read_line(&mut request_header).unwrap();

        let request_header: Vec<&str> = request_header
            .split_whitespace().collect();

        let (method, url) = (request_header[0], request_header[1]);

        for line in reader.lines() {
            let line = line.unwrap();
            let resources = resources.lock().unwrap();

            if let Some(resource) = resources.get(url) {
                let resource = resource.iter().find(|r| {
                    r.get_method().value() == method
                });

                match resource {
                    Some(resource) => {
                        let response = format!(
                            "HTTP/1.1 {}\r\n\r\n{}",
                            resource.get_status_description(),
                            resource.get_body()
                        );

                        write_stream.write(response.as_bytes()).unwrap();
                        write_stream.flush().unwrap();
                    },
                    None => {
                        write_stream.write(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n").unwrap();
                        write_stream.flush().unwrap();
                    }
                }
            } else {
                write_stream.write(b"HTTP/1.1 404 Not Found\r\n").unwrap();
                write_stream.flush().unwrap();
            }

            if line == "" {
                break;
            }
        }
    });
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
       request(port, uri, "GET")
    }

    fn make_post_request(port: u16, uri: &str) -> TcpStream {
       request(port, uri, "POST")
    }

    fn request(port: u16, uri: &str, method: &str) -> TcpStream {
        let host = format!("localhost:{}", port);
        let mut stream = TcpStream::connect(host).unwrap();
        let request = format!(
            "{} {} HTTP/1.1\r\n\r\n",
            method,
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
        let resource = server.create_resource("/something-else");

        resource.status(200);

        let stream = make_request(server.port(), "/something-else");

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        assert_eq!(line, "HTTP/1.1 200 Ok\r\n");
        server.close().unwrap();
    }

    #[test]
    fn should_return_resource_body() {
        let server = TestServer::new().unwrap();
        let resource = server.create_resource("/something-else");

        resource.status(200).body("<some body>");

        let stream = make_request(server.port(), "/something-else");

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_to_string(&mut line).unwrap();

        assert_eq!(line, "HTTP/1.1 200 Ok\r\n\r\n<some body>");
        server.close().unwrap();
    }

    #[test]
    fn should_listen_to_defined_method() {
        let server = TestServer::new().unwrap();
        let resource = server.create_resource("/something-else");

        resource.method(Method::POST).status(200).body("<some body>");

        let stream = make_post_request(server.port(), "/something-else");

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_to_string(&mut line).unwrap();

        assert_eq!(line, "HTTP/1.1 200 Ok\r\n\r\n<some body>");
        server.close().unwrap();
    }

    #[test]
    fn should_return_405_when_method_not_defined() {
        let server = TestServer::new().unwrap();
        let resource = server.create_resource("/something-else");

        resource.method(Method::POST).status(200).body("<some body>");

        let stream = make_request(server.port(), "/something-else");

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_to_string(&mut line).unwrap();

        assert_eq!(line, "HTTP/1.1 405 Method Not Allowed\r\n\r\n");
        server.close().unwrap();
    }
}
