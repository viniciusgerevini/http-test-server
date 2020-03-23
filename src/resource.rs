//! Server resource builders
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::collections::HashMap;
use std::time::Duration;

use ::Method;
use ::Status;

use regex::Regex;

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
///
/// To create resources with variable parts in the URL, you may use path and query parameters:
///
/// ```
/// # use http_test_server::TestServer;
/// # use http_test_server::http::{Method, Status};
/// # let server = TestServer::new().unwrap();
/// // matches /user/*/details?filter=*
/// let resource = server.create_resource("/user/{userId}/details?filter=*");
/// resource.body("All good for {path.userId} with filter {query.filter}!");
/// ```
/// _Note: I don't think it's a good idea to write mocks with complex behaviours. Usually,
///  they are less maintainable and harder to track._
///
///  _Instead, I would suggest creating one resource
///  for each behaviour expected. Having said that, I'm not here to judge. Do whatever floats your boat! :)_
///

pub struct Resource {
    uri: String,
    uri_regex: Regex,
    params: Arc<Mutex<URIParameters>>,
    status_code: Arc<Mutex<Status>>,
    custom_status_code: Arc<Mutex<Option<String>>>,
    headers: Arc<Mutex<HashMap<String, String>>>,
    body: Arc<Mutex<Option<&'static str>>>,
    body_builder: Arc<Mutex<Option<Box<dyn Fn(RequestParameters) -> String + Send>>>>, // ᕦ(ò_óˇ)ᕤ
    method: Arc<Mutex<Method>>,
    delay: Arc<Mutex<Option<Duration>>>,
    request_count: Arc<Mutex<u32>>,
    is_stream: Arc<Mutex<bool>>,
    stream_listeners: Arc<Mutex<Vec<mpsc::Sender<String>>>>
}

struct URIParameters {
    path: Vec<String>,
    query: HashMap<String, String>
}

impl Resource {
    pub(crate) fn new(uri: &str) -> Resource {
        let (uri_regex, params) = create_uri_regex(uri);

        Resource {
            uri: String::from(uri),
            uri_regex,
            params: Arc::new(Mutex::new(params)),
            status_code: Arc::new(Mutex::new(Status::OK)),
            custom_status_code: Arc::new(Mutex::new(None)),
            headers: Arc::new(Mutex::new(HashMap::new())),
            body: Arc::new(Mutex::new(None)),
            body_builder: Arc::new(Mutex::new(None)),
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

    /// Defines query parameters.
    ///
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// # let resource = server.create_resource("/i-am-a-resource");
    /// resource
    ///     .query("filter", "*") // wildcard, matches any value
    ///     .query("version", "1"); // only matches request with version == 1
    /// ```
    /// This is equivalent to:
    /// ```
    /// # use http_test_server::TestServer;
    /// # use http_test_server::http::{Method, Status};
    /// # let server = TestServer::new().unwrap();
    /// let resource = server.create_resource("/?filter=*&version=1");
    /// ```
    pub fn query(&self, name: &str, value: &str) -> &Resource {
        let mut params = self.params.lock().unwrap();
        params.query.insert(String::from(name), String::from(value));
        self
    }

    /// Defines response's body.
    ///
    /// If the response is a stream this value will be sent straight after connection.
    ///
    /// Calling multiple times will overwrite the previous value.
    ///
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// # let resource = server.create_resource("/i-am-a-resource");
    /// resource.body("this is important!");
    /// ```
    ///
    /// It's possible to use path and query parameters in the response body by defining `{path.<parameter_name>}` or `{query.<parameter_name>}`:
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// let resource = server.create_resource("/user/{userId}?filter=*");
    /// resource.body("Response for user: {path.userId} filter: {query.filter}");
    /// ```
    pub fn body(&self, content: &'static str) -> &Resource {
        if self.body_builder.lock().unwrap().is_some() {
            panic!("You can't define 'body' when 'body_fn' is already defined");
        }

        if let Ok(mut body) = self.body.lock() {
            *body = Some(content);
        }

        self
    }

    /// Defines function used to build the response's body.
    ///
    /// If the response is a stream value will be sent straight after connection.
    ///
    /// Calling multiple times will overwrite the previous value.
    ///
    /// ```
    /// # use http_test_server::TestServer;
    /// # let server = TestServer::new().unwrap();
    /// let resource = server.create_resource("/character/{id}?version=*");
    /// resource.body_fn(|params| {
    ///     println!("version: {}", params.query.get("version").unwrap());
    ///
    ///     match params.path.get("id").unwrap().as_str() {
    ///         "Balrog" => r#"{ "message": "YOU SHALL NOT PASS!" }"#.to_string(),
    ///         _ => r#"{ "message": "Fly, you fools!" }"#.to_string()
    ///     }
    /// });
    ///
    /// ```
    pub fn body_fn(&self, builder: impl Fn(RequestParameters) -> String + Send + 'static) -> &Resource {
        if self.body.lock().unwrap().is_some() {
            panic!("You can't define 'body_fn' when 'body' is already defined");
        }

        if let Ok(mut body_builder) = self.body_builder.lock() {
            *body_builder = Some(Box::new(builder));
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

    fn create_body(&self, uri: &str) -> String {
        let params = self.extract_params_from_uri(uri);

        if let Some(body_builder) = &*self.body_builder.lock().unwrap() {
            return body_builder(params);
        }

        match *self.body.lock().unwrap() {
            Some(body) => {
                let mut body = body.to_string();

                for (name, value) in &params.path {
                    let key = format!("{{path.{}}}", name);
                    body = body.replace(&key, value);
                }

                for (name, value) in &params.query {
                    let key = format!("{{query.{}}}", name);
                    body = body.replace(&key, value);
                }

                body.to_string()
            },
            None => {
                String::from("")
            }
        }
    }

    fn extract_params_from_uri(&self, uri: &str) -> RequestParameters {
        RequestParameters { path: self.extra_path_params(uri), query: extract_query_params(uri) }
    }

    fn extra_path_params(&self, uri: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();

        if let Some(values) = self.uri_regex.captures(uri) {
            for param in &self.params.lock().unwrap().path {
                if let Some(value) = values.name(param) {
                    params.insert(String::from(param), String::from(value.as_str()));
                }
            }
        }

        params
    }

    pub(crate) fn build_response(&self, uri: &str) -> String {
        format!("HTTP/1.1 {}\r\n{}\r\n{}",
            self.get_status_description(),
            self.get_headers(),
            self.create_body(uri)
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
                if listener.send(String::from(data)).is_err() {
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

    pub(crate) fn matches_uri(&self, uri: &str) -> bool {
        self.uri_regex.is_match(uri) && self.matches_query_parameters(uri)
    }

    fn matches_query_parameters(&self, uri: &str) -> bool {
        let query_params = extract_query_params(uri);

        for (expected_key, expected_value) in &self.params.lock().unwrap().query {
            if let Some(value) = query_params.get(expected_key) {
                if expected_value != value && expected_value != "*" {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

impl Clone for Resource {
    /// Returns a `Resource` copy that shares state with other copies.
    ///
    /// This is useful when working with same Resource across threads.
    fn clone(&self) -> Self {
        Resource {
            uri: self.uri.clone(),
            uri_regex: self.uri_regex.clone(),
            params: self.params.clone(),
            status_code: self.status_code.clone(),
            custom_status_code: self.custom_status_code.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            body_builder: self.body_builder.clone(),
            method: self.method.clone(),
            delay: self.delay.clone(),
            request_count: self.request_count.clone(),
            is_stream: self.is_stream.clone(),
            stream_listeners: self.stream_listeners.clone()
        }
    }
}

pub struct RequestParameters {
    pub path: HashMap<String, String>,
    pub query: HashMap<String, String>
}


fn create_uri_regex(uri: &str) -> (Regex, URIParameters) {
    let re = Regex::new(r"\{(?P<p>([A-z|0-9|_])+)\}").unwrap();
    let query_regex = Regex::new(r"\?.*").unwrap();

    let params: Vec<String> = re.captures_iter(uri).filter_map(|cap| {
        match cap.name("p") {
            Some(p) => Some(String::from(p.as_str())),
            None => None
        }
    }).collect();

    let query_params = extract_query_params(uri);

    let pattern = query_regex.replace(uri, "");
    let pattern = re.replace_all(&pattern, r"(?P<$p>[^//|/?]+)");

    (Regex::new(&pattern).unwrap(), URIParameters { path: params, query: query_params})
}

fn extract_query_params(uri: &str) -> HashMap<String, String> {
    let query_regex = Regex::new(r"((?P<qk>[^&]+)=(?P<qv>[^&]+))*").unwrap();
    let path_regex = Regex::new(r".*\?").unwrap();
    let only_query_parameters = path_regex.replace(uri, "");

    query_regex.captures_iter(&only_query_parameters).filter_map(|cap| {
        if let Some(query_key) = cap.name("qk") {
            let query_value = match cap.name("qv") {
                Some(v) => String::from(v.as_str()),
                None => String::from("")
            };
            return Some((String::from(query_key.as_str()), query_value));
        }
        None
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn should_convert_to_response_string() {
        let resource = Resource::new("/");
        resource.status(Status::NotFound);

        assert_eq!(resource.build_response("/"), "HTTP/1.1 404 Not Found\r\n\r\n");
    }

    #[test]
    fn should_convert_to_response_with_body() {
        let resource = Resource::new("/");
        resource.status(Status::Accepted).body("hello!");

        assert_eq!(resource.build_response("/"), "HTTP/1.1 202 Accepted\r\n\r\nhello!");
    }

    #[test]
    fn should_allows_custom_status() {
        let resource = Resource::new("/");
        resource.custom_status(666, "The Number Of The Beast").body("hello!");

        assert_eq!(resource.build_response("/"), "HTTP/1.1 666 The Number Of The Beast\r\n\r\nhello!");
    }

    #[test]
    fn should_overwrite_custom_status_with_status() {
        let resource = Resource::new("/");
        resource.custom_status(666, "The Number Of The Beast").status(Status::Forbidden).body("hello!");

        assert_eq!(resource.build_response("/"), "HTTP/1.1 403 Forbidden\r\n\r\nhello!");
    }

    #[test]
    fn should_add_headers() {
        let resource = Resource::new("/");
        resource
            .header("Content-Type", "application/json")
            .body("hello!");

        assert_eq!(resource.build_response("/"), "HTTP/1.1 200 Ok\r\nContent-Type: application/json\r\n\r\nhello!");
    }

    #[test]
    fn should_append_headers() {
        let resource = Resource::new("/");
        resource
            .header("Content-Type", "application/json")
            .header("Connection", "Keep-Alive")
            .body("hello!");

        let response = resource.build_response("/");

        assert!(response.contains("Content-Type: application/json\r\n"));
        assert!(response.contains("Connection: Keep-Alive\r\n"));
    }

    #[test]
    fn should_increment_request_count() {
        let resource = Resource::new("/");
        resource.body("hello!");

        resource.increment_request_count();
        resource.increment_request_count();
        resource.increment_request_count();

        assert_eq!(resource.request_count(), 3);
    }

    #[test]
    fn clones_should_share_same_state() {
        let resource = Resource::new("/");
        let dolly = resource.clone();

        resource.increment_request_count();
        dolly.increment_request_count();

        assert_eq!(resource.request_count(), dolly.request_count());
        assert_eq!(resource.request_count(), 2);
    }

    #[test]
    fn should_set_as_stream() {
        let resource = Resource::new("/");

        resource.stream().status(Status::Accepted);

        assert!(resource.is_stream());
    }


    #[test]
    fn should_notify_data() {
        let resource = Resource::new("/");

        let receiver = resource.stream_receiver();
        resource.send("some data").send("some data");

        assert_eq!(receiver.recv().unwrap(), "some data");
        assert_eq!(receiver.recv().unwrap(), "some data");
    }

    #[test]
    fn should_close_connections() {
        let resource = Resource::new("/");
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
        let resource = Resource::new("/");
        let _receiver = resource.stream_receiver();
        let _receiver_2 = resource.stream_receiver();

        assert_eq!(resource.open_connections_count(), 2);
    }


    #[test]
    fn should_decrease_count_when_receiver_dropped() {
        let resource = Resource::new("/");
        resource.stream_receiver();

        resource.send("some data");

        assert_eq!(resource.open_connections_count(), 0);
    }

    #[test]
    fn should_send_data_with_line_break() {
        let resource = Resource::new("/");

        let receiver = resource.stream_receiver();
        resource.send_line("some data").send_line("again");

        assert_eq!(receiver.recv().unwrap(), "some data\n");
        assert_eq!(receiver.recv().unwrap(), "again\n");
    }

    #[test]
    fn should_set_delay() {
        let resource = Resource::new("/");
        resource.delay(Duration::from_millis(200));

        assert_eq!(resource.get_delay(), Some(Duration::from_millis(200)));
    }

    #[test]
    fn should_match_uri() {
        let resource = Resource::new("/some-endpoint");
        assert!(resource.matches_uri("/some-endpoint"));
    }

    #[test]
    fn should_not_match_uri_when_uri_does_not_match() {
        let resource = Resource::new("/some-endpoint");
        assert!(!resource.matches_uri("/some-other-endpoint"));
    }

    #[test]
    fn should_match_uri_with_path_params() {
        let resource = Resource::new("/endpoint/{param1}/some/{param2}");
        assert!(resource.matches_uri("/endpoint/123/some/abc"));
        assert!(resource.matches_uri("/endpoint/123-345/some/abc"));
    }

    #[test]
    fn should_not_match_uri_with_path_params_when_uri_does_not_match() {
        let resource = Resource::new("/endpoint/{param1}/some/{param2}");
        assert!(!resource.matches_uri("/endpoint/123/some/"));
    }

    #[test]
    fn should_match_uri_with_query_params() {
        let resource = Resource::new("/endpoint?userId=123");
        assert!(resource.matches_uri("/endpoint?userId=123"));
    }

    #[test]
    fn should_not_match_uri_with_wrong_query_parameter() {
        let resource = Resource::new("/endpoint?userId=123");
        assert!(!resource.matches_uri("/endpoint?userId=abc"));
    }

    #[test]
    fn should_match_uri_with_multiple_query_params() {
        let resource = Resource::new("/endpoint?userId=123&hello=abc");
        assert!(resource.matches_uri("/endpoint?userId=123&hello=abc"));
    }

    #[test]
    fn should_match_uri_with_wildcard_query_params() {
        let resource = Resource::new("/endpoint?userId=123&collectionId=*");
        assert!(resource.matches_uri("/endpoint?userId=123&collectionId=banana"));
    }

    #[test]
    fn should_match_uri_with_query_params_in_different_order() {
        let resource = Resource::new("/endpoint?hello=abc&userId=123");
        assert!(resource.matches_uri("/endpoint?userId=123&hello=abc"));
    }

    #[test]
    fn should_not_match_uri_when_one_query_param_is_wrong() {
        let resource = Resource::new("/endpoint?userId=123&hello=abc");
        assert!(!resource.matches_uri("/endpoint?userId=123&hello=bbc"));
    }

    #[test]
    fn should_match_uri_with_query_params_defined_through_method() {
        let resource = Resource::new("/endpoint");
        resource.query("hello", "abc").query("userId", "123");
        assert!(resource.matches_uri("/endpoint?userId=123&hello=abc"));
    }

    #[test]
    fn should_match_uri_with_wildcard_query_params_defined_through_method() {
        let resource = Resource::new("/endpoint");
        resource.query("hello", "*");
        assert!(resource.matches_uri("/endpoint?hello=1234"));
    }

    #[test]
    fn should_build_response() {
        let resource = Resource::new("/");
        resource.status(Status::NotFound);

        assert_eq!(resource.build_response("/"), "HTTP/1.1 404 Not Found\r\n\r\n");
    }

    #[test]
    fn should_build_response_with_body() {
        let resource = Resource::new("/");
        resource.status(Status::Accepted).body("hello!");

        assert_eq!(resource.build_response("/"), "HTTP/1.1 202 Accepted\r\n\r\nhello!");
    }

    #[test]
    fn should_build_response_with_path_parameters() {
        let resource = Resource::new("/endpoint/{param1}/{param2}");
        resource.status(Status::Accepted).body("Hello: {path.param2} {path.param1}");

        assert_eq!(resource.build_response("/endpoint/123/abc"), "HTTP/1.1 202 Accepted\r\n\r\nHello: abc 123");
    }

    #[test]
    fn should_build_response_with_query_parameters() {
        let resource = Resource::new("/endpoint/{param1}?param2=111");
        resource.status(Status::Accepted).body("Hello: {query.param2} {path.param1}");

        assert_eq!(resource.build_response("/endpoint/123?param2=111"), "HTTP/1.1 202 Accepted\r\n\r\nHello: 111 123");
    }

    #[test]
    fn should_build_response_with_wildcard_query_parameters() {
        let resource = Resource::new("/endpoint/{param1}?param2=111&param3=*");
        resource.status(Status::Accepted).body("Hello: {query.param3}");

        assert_eq!(resource.build_response("/endpoint/123?param2=111&param3=banana"), "HTTP/1.1 202 Accepted\r\n\r\nHello: banana");
    }

    #[test]
    fn should_build_response_using_body_fn() {
        let resource = Resource::new("/endpoint/{param1}/{param2}");
        resource.status(Status::Accepted).body_fn(|params| {
            format!("Hello: {} {}", params.path.get("param2").unwrap(), params.path.get("param1").unwrap())
        });

        assert_eq!(resource.build_response("/endpoint/123/abc"), "HTTP/1.1 202 Accepted\r\n\r\nHello: abc 123");
    }

    #[test]
    #[should_panic(expected = "You can't define 'body_fn' when 'body' is already defined")]
    fn should_fail_when_trying_to_define_body_fn_after_defining_body() {
        let resource = Resource::new("/endpoint/{param1}/{param2}");
        resource.body("some body");
        resource.body_fn(|_params| String::from(""));
    }

    #[test]
    #[should_panic(expected = "You can't define 'body' when 'body_fn' is already defined")]
    fn should_fail_when_trying_to_define_body_after_defining_body_fn() {
        let resource = Resource::new("/endpoint/{param1}/{param2}");
        resource.body_fn(|_params| String::from(""));
        resource.body("some body");
    }
}
