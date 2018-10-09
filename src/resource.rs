use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug)]
pub struct Resource {
    status_code: Arc<Mutex<u16>>,
    body: Arc<Mutex<&'static str>>
}

impl Resource {
    pub fn new() -> Resource {
        Resource {
            status_code: Arc::new(Mutex::new(204)),
            body: Arc::new(Mutex::new(""))
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

    pub fn get_status_description(&self) -> String {
        http_status_description(*self.status_code.lock().unwrap())
    }

    pub fn get_body(&self) -> &'static str {
        match self.body.lock() {
            Ok(body) => *body,
            _ => ""
        }
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


