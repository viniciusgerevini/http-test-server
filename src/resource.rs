use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::collections::HashMap;
use std::time::Duration;

use ::Method;
use ::Status;

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

    pub fn custom_status(&self, status_code: u16, description: &str) -> &Resource {
        if let Ok(mut status) = self.custom_status_code.lock() {
            *status = Some(format!("{} {}", status_code, description));
        }
        self
    }

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

    pub fn body(&self, content: &'static str) -> &Resource {
        if let Ok(mut body) = self.body.lock() {
            *body = content;
        }

        self
    }

    pub fn method(&self, method: Method) -> &Resource {
        if let Ok(mut m) = self.method.lock() {
            *m = method;
        }

        self
    }

    pub(crate) fn get_method(&self) -> Method {
        (*self.method.lock().unwrap()).clone()
    }

    pub fn delay(&self, delay: Duration) -> &Resource {
        if let Ok(mut d) = self.delay.lock() {
            *d = Some(delay);
        }

        self
    }

    pub(crate) fn get_delay(&self) -> Option<Duration> {
        (*self.delay.lock().unwrap()).clone()
    }

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

    pub fn request_count(&self) -> u32 {
        *(self.request_count.lock().unwrap())
    }

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

    pub fn send_line(&self, data: &str) -> &Resource {
        self.send(&format!("{}\n", data))
    }

    pub fn close_open_connections(&self) {
        if let Ok(mut listeners) = self.stream_listeners.lock() {
            listeners.clear();
        }
    }

    pub fn open_connections_count(&self) -> usize {
        let listeners = self.stream_listeners.lock().unwrap();
        listeners.len()
    }

    pub fn stream_receiver(&self) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel();

        if let Ok(mut listeners) = self.stream_listeners.lock() {
            listeners.push(tx);
        }
        rx
    }
}

impl Clone for Resource {
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
