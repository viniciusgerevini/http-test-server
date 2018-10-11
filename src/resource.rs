use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;

use ::Method;
use ::Status;

#[derive(Debug)]
pub struct Resource {
    status_code: Arc<Mutex<Status>>,
    custom_status_code: Arc<Mutex<Option<String>>>,
    headers: Arc<Mutex<HashMap<String, String>>>,
    body: Arc<Mutex<&'static str>>,
    method: Arc<Mutex<Method>>,
    request_count: Mutex<u32>
}

impl Resource {
    pub fn new() -> Resource {
        Resource {
            status_code: Arc::new(Mutex::new(Status::NoContent)),
            custom_status_code: Arc::new(Mutex::new(None)),
            headers: Arc::new(Mutex::new(HashMap::new())),
            body: Arc::new(Mutex::new("")),
            method: Arc::new(Mutex::new(Method::GET)),
            request_count: Mutex::new(0)
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

    pub fn get_method(&self) -> Method {
        (*self.method.lock().unwrap()).clone()
    }

    pub fn to_response_string(&self) -> String {
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
}

#[cfg(test)]
mod tests {
    use super::*;

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

        assert_eq!(resource_not_found.to_response_string(), "HTTP/1.1 204 No Content\r\nContent-Type: application/json\r\n\r\nhello!");
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


}
