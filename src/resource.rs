// use std::io::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug)]
pub struct Resource {
    status_code: Arc<Mutex<u16>>
}

impl Resource {
    pub fn new() -> Resource {
        Resource { status_code: Arc::new(Mutex::new(204))}
    }

    pub fn status(&self, status_code: u16) {
        if let Ok(mut status) = self.status_code.lock() {
            *status = status_code;
        }
    }

    pub fn get_status_description(&self) -> String {
        http_status_description(*self.status_code.lock().unwrap())
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


