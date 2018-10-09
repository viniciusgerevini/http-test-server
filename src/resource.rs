use std::sync::Arc;
use std::sync::Mutex;

use ::Method;

#[derive(Debug)]
pub struct Resource {
    status_code: Arc<Mutex<u16>>,
    body: Arc<Mutex<&'static str>>,
    method: Arc<Mutex<Method>>
}

impl Resource {
    pub fn new() -> Resource {
        Resource {
            status_code: Arc::new(Mutex::new(204)),
            body: Arc::new(Mutex::new("")),
            method: Arc::new(Mutex::new(Method::GET))
        }
    }

    pub fn status(&self, status_code: u16) -> &Resource {
        if let Ok(mut status) = self.status_code.lock() {
            *status = status_code;
        }

        self
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

    pub fn get_status_description(&self) -> String {
        http_status_description(*self.status_code.lock().unwrap())
    }

    pub fn get_body(&self) -> &'static str {
        match self.body.lock() {
            Ok(body) => *body,
            _ => ""
        }
    }

    pub fn get_method(&self) -> Method {
        (*self.method.lock().unwrap()).clone()
    }
}

fn http_status_description(status_code: u16) -> String {
    let status = match status_code {
        200 => "200 Ok",
        204 => "204 No Content",
        _ => "Unknown"
    };

    String::from(status)
}


