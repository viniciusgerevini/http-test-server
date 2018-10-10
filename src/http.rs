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

    pub fn equal(&self, value: &str) -> bool {
        self.value() == value
    }
}

#[derive(Debug)]
pub enum Status {
    Continue = 100,
    SwitchingProtocols = 101,
    Processing = 102,
    OK = 200,
    Created = 201,
    Accepted = 202,
    NonAuthoritativeInformation = 203,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,
    MultiStatus= 207,
    MultipleChoices= 300,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified= 304,
    UseProxy = 305,
    TemporaryRedirect = 307,
    PermanentRedirect = 308,
    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired= 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Conflict = 409,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    PayloadTooLarge = 413,
    UriTooLong = 414,
    UnsupportedMediaType  = 415,
    RangeNotSatisfiable = 416,
    ExpectationFailed = 417,
    ImATeapot = 418,
    UnprocessableEntity= 422,
    Locked = 423,
    FailedDependency = 424,
    UpgradeRequired = 426,
    PreconditionRequired   = 428,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    HttpVersionNotSupported = 505,
    InsufficientStorage = 507,
    NetworkAuthenticationRequired = 511
}

impl Status {
    pub fn description(&self) -> &'static str {
        match self {
            Status::Continue => "100 Continue",
            Status::SwitchingProtocols => "101 Switching Protocols",
            Status::Processing => "102 Processing",
            Status::OK => "200 Ok",
            Status::Created => "201 Created",
            Status::Accepted => "202 Accepted",
            Status::NonAuthoritativeInformation => "203 Non Authoritative Information",
            Status::NoContent => "204 No Content",
            Status::ResetContent  => "205 Reset Content",
            Status::PartialContent => "206 Partial Content",
            Status::MultiStatus=> "207 Multi Status",
            Status::MultipleChoices=> "300 Multiple Choices",
            Status::MovedPermanently => "301 Moved Permanently",
            Status::Found => "302 Found",
            Status::SeeOther => "303 See Other",
            Status::NotModified=> "304 Not Modified",
            Status::UseProxy => "305 Use Proxy",
            Status::TemporaryRedirect => "307 Temporary Redirect",
            Status::PermanentRedirect => "308 Permanent Redirect",
            Status::BadRequest => "400 Bad Request",
            Status::Unauthorized => "401 Unauthorized",
            Status::PaymentRequired=> "402 Payment Required",
            Status::Forbidden => "403 Forbidden",
            Status::NotFound => "404 Not Found",
            Status::MethodNotAllowed => "405 Method Not Allowed",
            Status::NotAcceptable => "406 Not Acceptable",
            Status::ProxyAuthenticationRequired => "407 Proxy Authentication Required",
            Status::RequestTimeout => "408 Request Timeout",
            Status::Conflict => "409 Conflict",
            Status::Gone => "410 Gone",
            Status::LengthRequired => "411 Length Required",
            Status::PreconditionFailed => "412 Precondition Failed",
            Status::PayloadTooLarge => "413 Payload Too Large",
            Status::UriTooLong => "414 URI Too Long",
            Status::UnsupportedMediaType  => "415 Unsupported Media Type",
            Status::RangeNotSatisfiable => "416 Range Not Satisfiable",
            Status::ExpectationFailed => "417 Expectation Failed",
            Status::ImATeapot => "418 I'm A Teapot",
            Status::UnprocessableEntity=> "422 Unprocessable Entity",
            Status::Locked => "423 Locked",
            Status::FailedDependency => "424 Failed Dependency",
            Status::UpgradeRequired => "426 Upgrade Required",
            Status::PreconditionRequired   => "428 Precondition Required",
            Status::TooManyRequests => "429 Too Many Requests",
            Status::RequestHeaderFieldsTooLarge => "431 Request Header Fields Too Large",
            Status::InternalServerError => "500 Internal Server Error",
            Status::NotImplemented => "501 Not Implemented",
            Status::BadGateway => "502 Bad Gateway",
            Status::ServiceUnavailable => "503 Service Unavailable",
            Status::GatewayTimeout => "504 Gateway Timeout",
            Status::HttpVersionNotSupported => "505 Http Version Not Supported",
            Status::InsufficientStorage => "507 Insufficient Storage",
            Status::NetworkAuthenticationRequired => "511 Network Authentication Required",
        }
    }
}

