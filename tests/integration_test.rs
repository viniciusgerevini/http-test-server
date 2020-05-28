extern crate http_test_server;

use std::io::prelude::*;
use std::io::BufReader;
use std::net::TcpStream;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use http_test_server::TestServer;
use http_test_server::http::{Method, Status};

#[test]
fn test_defaults() {
    let server = TestServer::new().unwrap();
    let resource = server.create_resource("/defaults");
    let requests = server.requests();

    let response = request(server.port(), "/defaults", "GET");
    let request_data = requests.recv().unwrap();

    assert_eq!(response, "HTTP/1.1 200 Ok\r\n\r\n");

    assert_eq!(request_data.url, "/defaults");
    assert_eq!(request_data.method, "GET");
    assert_eq!(request_data.headers, HashMap::new());

    assert_eq!(resource.request_count(), 1);
}

#[test]
fn test_post_request() {
    let server = TestServer::new().unwrap();
    let resource = server.create_resource("/create");

    resource
        .method(Method::POST)
        .status(Status::Created)
        .header("Content-Type", "text")
        .body("Everything is fine!");

    let response = request(server.port(), "/create", "POST");

    assert_eq!(response, "HTTP/1.1 201 Created\r\nContent-Type: text\r\n\r\nEverything is fine!");
}

#[test]
fn test_stream() {
    let server = TestServer::new().unwrap();
    let resource = server.create_resource("/stream");

    resource
        .stream()
        .header("Content-Type", "text/event-stream")
        .body(": initial data\n");

    let resource_clone = resource.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        resource_clone.send_line("Hello.");
        resource_clone.send_line("Is there anybody in there?");
        resource_clone.send_line("Just nod if you can hear me.");
        resource_clone.close_open_connections();
    });

    let stream = open_stream(server.port(), "/stream", "GET");
    thread::sleep(Duration::from_millis(200));
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_to_string(&mut response).unwrap();

    assert_eq!(response, "HTTP/1.1 200 Ok\r\nContent-Type: text/event-stream\r\n\r\n: initial data\nHello.\nIs there anybody in there?\nJust nod if you can hear me.\n");
}

#[test]
fn test_request_with_path_and_query_params() {
    let server = TestServer::new().unwrap();
    let resource = server.create_resource("/user/{userId}?filter=*&version=1");

    resource
        .status(Status::OK)
        .header("Content-Type", "application/json")
        .body(r#"{"id": 123, "userId": "{path.userId}", "filter": "{query.filter}", "v": {query.version}}"#);

    let response = request(server.port(), "/user/superUser?filter=all&version=1", "GET");

    assert_eq!(
        response,
        "HTTP/1.1 200 Ok\r\nContent-Type: application/json\r\n\r\n{\"id\": 123, \"userId\": \"superUser\", \"filter\": \"all\", \"v\": 1}"
    );
}

#[test]
fn test_request_to_regex_uri() {
    let server = TestServer::new().unwrap();
    let resource = server.create_resource("/hello/[0-9]/[A-z]/.*");
    let requests = server.requests();

    let response = request(server.port(), "/hello/2/b/goodbye", "GET");
    let request_data = requests.recv().unwrap();

    assert_eq!(response, "HTTP/1.1 200 Ok\r\n\r\n");

    assert_eq!(request_data.url, "/hello/2/b/goodbye");
    assert_eq!(request_data.method, "GET");
    assert_eq!(request_data.headers, HashMap::new());

    assert_eq!(resource.request_count(), 1);
}

#[test]
fn request_to_loopback_ip() {
    let server = TestServer::new().unwrap();
    let resource = server.create_resource("/hello");

    let host = format!("127.0.0.1:{}", server.port());
    let mut stream = TcpStream::connect(host).unwrap();

    stream.write("GET /hello HTTP/1.1\r\n\r\n".as_bytes()).unwrap();
    stream.flush().unwrap();

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_to_string(&mut response).unwrap();

    assert_eq!(response, "HTTP/1.1 200 Ok\r\n\r\n");
    assert_eq!(resource.request_count(), 1);
}


fn request(port: u16, uri: &str, method: &str) -> String {
    let stream = open_stream(port, uri, method);

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_to_string(&mut response).unwrap();

    response
}

fn open_stream(port: u16, uri: &str, method: &str) -> TcpStream {
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

