use std::fmt::Display;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Method {
    #[serde(alias = "OPTIONS")]
    Options,
    #[serde(alias = "GET")]
    Get,
    #[serde(alias = "POST")]
    Post,
    #[serde(alias = "PUT")]
    Put,
    #[serde(alias = "DELETE")]
    Delete,
    #[serde(alias = "HEAD")]
    Head,
    #[serde(alias = "TRACE")]
    Trace,
    #[serde(alias = "CONNECT")]
    Connect,
    #[serde(alias = "PATCH")]
    Patch,
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Options => "OPTIONS",
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Trace => "TRACE",
            Self::Connect => "CONNECT",
            Self::Patch => "PATCH",
        })
    }
}
