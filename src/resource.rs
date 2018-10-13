//! Server resource builders
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::collections::HashMap;
use std::time::Duration;

use ::Method;
use ::Status;

/// Responsible for configuring a resource and interacting with it.
///
/// Must be created through `TestServer`.
///
/// By default a resource's method is `GET` and response is `200 Ok` with empty body.
///
/// ```
/// use http_test_server::TestServer;
/// use http_test_server::http::{Method, Status};
///
/// let server = TestServer::new().unwrap();
/// let resource = server.create_resource("/i-am-a-resource");
///
/// resource
///     .status(Status::PartialContent)
///     .method(Method::POST)
///     .body("All good!");
///
/// ```
#[derive(Debug)]
pub struct Resource {
    status_code: Arc<Mutex<Status>>,
    custom_status_code: Arc<Mutex<Option<String>>>,
    headers: Arc<Mutex<HashMap<String, String>>>,
    body: Arc<Mutex<&'static str>>,
    method: Arc<Mutex<Method>>,
    delay: Arc<Mutex<Option<Duration>>>,
    request_count: Arc<Mutex<u32>>,
    is_stream: Arc<Mutex<bool>>,
    stream_listeners: Arc<Mutex<Vec<mpsc::Sender<String>>>>
}

impl Resource {
    pub(crate) fn new() -> Resource {
        Resource {
            status_code: Arc::new(Mutex::new(Status::OK)),
            custom_status_code: Arc::new(Mutex::new(None)),
            headers: Arc::new(Mutex::new(HashMap::new())),
            body: Arc::new(Mutex::new("")),
            method: Arc::new(Mutex::new(Method::GET)),
            delay: Arc::new(Mutex::new(None)),
            request_count: Arc::new(Mutex::new(0)),
            is_stream: Arc::new(Mutex::new(false)),
            stream_listeners: Arc::new(Mutex::new(vec!()))
        }
    }

    /// Defines response's HTTP Status .
    ///
    /// Refer to [`custom_status`] for Statuses not covered by [`Status`].
    /// ```
    /// # use http_test_server::TestServer;
    /// # use http_test_server::http::{Method, Status};
    /// # let server = TestServer::new().unwrap();
    /// # let resource = server.create_resource("/i-am-a-resource");
    /// resource.status(Status::PartialContent);
    /// ```
    /// [`custom_status`]: struct.Resource.html#method.custom_status
    /// [`Status`]: ../http/enum.Status.html
    pub fn status(&self, status_code: Status) -> &Resource {
        if let Ok(mut status) = self.status_code.lock() {
            *status = status_code;
        }

        if let Ok(mut custom_status) = self.custom_status_code.lock() {
            *custom_status = None;
        }

        self
    }

    fn get_status_description(&self) -> String {
        match *(self.custom_status_code.lock().unwrap()) {
            Some(ref custom_status) => custom_status.clone(),
            None => self.status_code.lock().unwrap().description().to_string()
        }
    }

    /// Defines a custom HTTP Status to response.
    ///
    /// Use it to return HTTP statuses that are not covered by [`Status`].
    ///
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// # let resource = server.create_resource("/i-am-a-resource");
    /// resource.custom_status(333, "Only Half Beast");
    /// ```
    /// [`Status`]: ../http/enum.Status.html
    pub fn custom_status(&self, status_code: u16, description: &str) -> &Resource {
        if let Ok(mut status) = self.custom_status_code.lock() {
            *status = Some(format!("{} {}", status_code, description));
        }
        self
    }

    /// Defines response headers.
    ///
    /// Call it multiple times to add multiple headers.
    /// If a header is defined twice only the late value is returned.
    ///
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// # let resource = server.create_resource("/i-am-a-resource");
    /// resource
    ///     .header("Content-Type", "application/json")
    ///     .header("Connection", "Keep-Alive");
    /// ```
    pub fn header(&self, header_name: &str, header_value: &str) -> &Resource {
        let mut headers = self.headers.lock().unwrap();
        headers.insert(String::from(header_name), String::from(header_value));
        self
    }

    fn get_headers(&self) -> String {
        let headers = self.headers.lock().unwrap();
        headers.iter().fold(String::new(), | headers, (name, value) | {
            headers + &format!("{}: {}\r\n", name, value)
        })
    }

    /// Defines response's body.
    ///
    /// If the response is a stream this value will be sent straight after connection.
    ///
    /// Calling multiple times will overwrite the value.
    ///
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// # let resource = server.create_resource("/i-am-a-resource");
    /// resource.body("this is important!");
    /// ```
    pub fn body(&self, content: &'static str) -> &Resource {
        if let Ok(mut body) = self.body.lock() {
            *body = content;
        }

        self
    }

    /// Defines HTTP method.
    ///
    /// A resource will only respond to one method, however multiple resources with same URL and
    /// different methods can be created.
    /// ```
    /// # use http_test_server::TestServer;
    /// use http_test_server::http::Method;
    /// # let server = TestServer::new().unwrap();
    /// let resource_put = server.create_resource("/i-am-a-resource");
    /// let resource_post = server.create_resource("/i-am-a-resource");
    ///
    /// resource_put.method(Method::PUT);
    /// resource_post.method(Method::POST);
    /// ```
    pub fn method(&self, method: Method) -> &Resource {
        if let Ok(mut m) = self.method.lock() {
            *m = method;
        }

        self
    }

    pub(crate) fn get_method(&self) -> Method {
        (*self.method.lock().unwrap()).clone()
    }

    /// Defines delay to response after client connected
    /// ```
    /// # use http_test_server::TestServer;
    /// use std::time::Duration;
    /// # let server = TestServer::new().unwrap();
    /// # let resource = server.create_resource("/i-am-a-resource");
    ///
    /// resource.delay(Duration::from_millis(500));
    /// ```
    pub fn delay(&self, delay: Duration) -> &Resource {
        if let Ok(mut d) = self.delay.lock() {
            *d = Some(delay);
        }

        self
    }

    pub(crate) fn get_delay(&self) -> Option<Duration> {
        (*self.delay.lock().unwrap()).clone()
    }

    /// Set response as stream, this means clients won't be disconnected after body is sent and
    /// updates can be sent and received.
    ///
    /// See also: [`send`], [`send_line`], [`stream_receiver`].
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// let resource = server.create_resource("/stream");
    ///
    /// resource.stream();
    ///
    /// resource
    ///     .send_line("some")
    ///     .send_line("data")
    ///     .close_open_connections();
    /// ```
    /// [`send`]: struct.Resource.html#method.send
    /// [`send_line`]: struct.Resource.html#method.send_line
    /// [`stream_receiver`]: struct.Resource.html#method.stream_receiver
    /// [`close_open_connections`]: struct.Resource.html#method.close_open_connections
    pub fn stream(&self) -> &Resource {
        *(self.is_stream.lock().unwrap()) = true;

        self
    }

    pub(crate) fn is_stream(&self) -> bool {
        *(self.is_stream.lock().unwrap())
    }

    pub(crate) fn to_response_string(&self) -> String {
        format!("HTTP/1.1 {}\r\n{}\r\n{}",
            self.get_status_description(),
            self.get_headers(),
            *(self.body.lock().unwrap())
        )
    }

    pub(crate) fn increment_request_count(&self) {
        *(self.request_count.lock().unwrap()) += 1;
    }

    /// Send data to all connected clients.
    ///
    /// See also: [`send_line`], [`stream`].
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// let resource = server.create_resource("/stream");
    ///
    /// resource.stream();
    ///
    /// resource
    ///     .send("some")
    ///     .send(" data");
    /// ```
    /// [`send_line`]: struct.Resource.html#method.send_line
    /// [`stream`]: struct.Resource.html#method.stream
    pub fn send(&self, data: &str) -> &Resource {
        if let Ok(mut listeners) = self.stream_listeners.lock() {
            let mut invalid_listeners = vec!();
            for (i, listener) in listeners.iter().enumerate() {
                if let Err(_) = listener.send(String::from(data)) {
                    invalid_listeners.push(i);
                }
            }

            for i in invalid_listeners.iter() {
                listeners.remove(*i);
            }
        }

        self
    }

    /// Send data to all connected clients.
    /// Same as [`send`], but appends `\n` to data.
    ///
    /// See also: [`stream`]
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// let resource = server.create_resource("/stream");
    ///
    /// resource.stream();
    ///
    /// resource
    ///     .send_line("one line")
    ///     .send_line("another line");
    /// ```
    /// [`send`]: struct.Resource.html#method.send
    /// [`stream`]: struct.Resource.html#method.stream
    pub fn send_line(&self, data: &str) -> &Resource {
        self.send(&format!("{}\n", data))
    }

    /// Close all connections with clients.
    ///
    /// See also: [`stream`]
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// let resource = server.create_resource("/stream");
    ///
    /// resource.stream();
    ///
    /// resource.close_open_connections();
    /// ```
    /// [`stream`]: struct.Resource.html#method.stream

    pub fn close_open_connections(&self) {
        if let Ok(mut listeners) = self.stream_listeners.lock() {
            listeners.clear();
        }
    }

    /// Number of clients connected to stream.
    ///
    /// See also: [`stream`]
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// let resource = server.create_resource("/stream");
    ///
    /// resource
    ///     .stream()
    ///     .close_open_connections();
    ///
    /// assert_eq!(resource.open_connections_count(), 0);
    /// ```
    /// [`stream`]: struct.Resource.html#method.stream
    pub fn open_connections_count(&self) -> usize {
        let listeners = self.stream_listeners.lock().unwrap();
        listeners.len()
    }

    /// Receives data sent from clients through stream.
    ///
    /// See also: [`stream`]
    /// ```no_run
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// let resource = server.create_resource("/stream");
    /// let receiver = resource.stream().stream_receiver();
    ///
    /// let new_message = receiver.recv().unwrap();
    ///
    /// for message in receiver.iter() {
    ///     println!("Client message: {}", message);
    /// }
    /// ```
    /// [`stream`]: struct.Resource.html#method.stream
    pub fn stream_receiver(&self) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel();

        if let Ok(mut listeners) = self.stream_listeners.lock() {
            listeners.push(tx);
        }
        rx
    }

    /// Number of requests received
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// # let resource = server.create_resource("/stream");
    /// assert_eq!(resource.request_count(), 0);
    /// ```
    pub fn request_count(&self) -> u32 {
        *(self.request_count.lock().unwrap())
    }
}

impl Clone for Resource {
    /// Returns a `Resource` copy that shares state with other copies.
    ///
    /// This is useful when working with same Resource across threads.
    fn clone(&self) -> Self {
        Resource {
            status_code: self.status_code.clone(),
            custom_status_code: self.custom_status_code.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            method: self.method.clone(),
            delay: self.delay.clone(),
            request_count: self.request_count.clone(),
            is_stream: self.is_stream.clone(),
            stream_listeners: self.stream_listeners.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn should_convert_to_response_string() {
        let resource_not_found = Resource::new();
        resource_not_found.status(Status::NotFound);

        assert_eq!(resource_not_found.to_response_string(), "HTTP/1.1 404 Not Found\r\n\r\n");
    }

    #[test]
    fn should_convert_to_response_with_body() {
        let resource_not_found = Resource::new();
        resource_not_found.status(Status::Accepted).body("hello!");

        assert_eq!(resource_not_found.to_response_string(), "HTTP/1.1 202 Accepted\r\n\r\nhello!");
    }

    #[test]
    fn should_allows_custom_status() {
        let resource_not_found = Resource::new();
        resource_not_found.custom_status(666, "The Number Of The Beast").body("hello!");

        assert_eq!(resource_not_found.to_response_string(), "HTTP/1.1 666 The Number Of The Beast\r\n\r\nhello!");
    }

    #[test]
    fn should_overwrite_custom_status_with_status() {
        let resource_not_found = Resource::new();
        resource_not_found.custom_status(666, "The Number Of The Beast").status(Status::Forbidden).body("hello!");

        assert_eq!(resource_not_found.to_response_string(), "HTTP/1.1 403 Forbidden\r\n\r\nhello!");
    }

    #[test]
    fn should_add_headers() {
        let resource_not_found = Resource::new();
        resource_not_found
            .header("Content-Type", "application/json")
            .body("hello!");

        assert_eq!(resource_not_found.to_response_string(), "HTTP/1.1 200 Ok\r\nContent-Type: application/json\r\n\r\nhello!");
    }

    #[test]
    fn should_append_headers() {
        let resource_not_found = Resource::new();
        resource_not_found
            .header("Content-Type", "application/json")
            .header("Connection", "Keep-Alive")
            .body("hello!");

        let response = resource_not_found.to_response_string();

        assert!(response.contains("Content-Type: application/json\r\n"));
        assert!(response.contains("Connection: Keep-Alive\r\n"));
    }

    #[test]
    fn should_increment_request_count() {
        let resource = Resource::new();
        resource.body("hello!");

        resource.increment_request_count();
        resource.increment_request_count();
        resource.increment_request_count();

        assert_eq!(resource.request_count(), 3);
    }

    #[test]
    fn clones_should_share_same_state() {
        let resource = Resource::new();
        let dolly = resource.clone();

        resource.increment_request_count();
        dolly.increment_request_count();

        assert_eq!(resource.request_count(), dolly.request_count());
        assert_eq!(resource.request_count(), 2);
    }

    #[test]
    fn should_set_as_stream() {
        let resource = Resource::new();

        resource.stream().status(Status::Accepted);

        assert!(resource.is_stream());
    }


    #[test]
    fn should_notify_data() {
        let resource = Resource::new();

        let receiver = resource.stream_receiver();
        resource.send("some data").send("some data");

        assert_eq!(receiver.recv().unwrap(), "some data");
        assert_eq!(receiver.recv().unwrap(), "some data");
    }

    #[test]
    fn should_close_connections() {
        let resource = Resource::new();
        let res = resource.clone();
        let receiver = resource.stream_receiver();

        thread::spawn(move || {
            res.send("some data");
            res.send("some data");
            res.close_open_connections();
        });

        let mut string = String::new();

        for data in receiver.iter() {
            string = string + &data;
        }

        assert_eq!(string, "some datasome data");
    }

    #[test]
    fn should_return_number_of_connecteds_users() {
        let resource = Resource::new();
        let _receiver = resource.stream_receiver();
        let _receiver_2 = resource.stream_receiver();

        assert_eq!(resource.open_connections_count(), 2);
    }


    #[test]
    fn should_decrease_count_when_receiver_dropped() {
        let resource = Resource::new();
        resource.stream_receiver();

        resource.send("some data");

        assert_eq!(resource.open_connections_count(), 0);
    }

    #[test]
    fn should_send_data_with_line_break() {
        let resource = Resource::new();

        let receiver = resource.stream_receiver();
        resource.send_line("some data").send_line("again");

        assert_eq!(receiver.recv().unwrap(), "some data\n");
        assert_eq!(receiver.recv().unwrap(), "again\n");
    }

    #[test]
    fn should_set_delay() {
        let resource = Resource::new();
        resource.delay(Duration::from_millis(200));

        assert_eq!(resource.get_delay(), Some(Duration::from_millis(200)));
    }
}
