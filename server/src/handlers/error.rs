use rouille::Response;
use serde::{Serialize, Serializer};

#[derive(Debug, Serialize)]
pub struct Error {
    #[serde(serialize_with = "serialize_status")]
    status: u16,
    title: &'static str,
    detail: String,
}

fn serialize_status<S>(status: &u16, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&status.to_string())
}

impl Error {
    pub fn missing_authorization() -> Self {
        Self {
            status: 401,
            title: "missing authorization",
            detail: "requests must contain an <X-API-KEY> header with an API key".to_string(),
        }
    }

    pub fn failed_authorization() -> Self {
        Self {
            status: 401,
            title: "failed authorization",
            detail: "provided API key is not associated with any client".to_string(),
        }
    }

    pub fn forbidden() -> Self {
        Self {
            status: 403,
            title: "access forbidden",
            detail: "insufficient permissions to access the requested resource".to_string(),
        }
    }
}

impl From<Error> for Response {
    fn from(error: Error) -> Self {
        Response::json(&error).with_status_code(error.status)
    }
}
