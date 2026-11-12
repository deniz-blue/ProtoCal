use std::{
    fmt,
    future::Future,
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use http_body_util::{BodyExt, Empty, Limited};
use hyper::{Request, body::Bytes, header};
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{
        Client,
        connect::{HttpConnector, dns::Name},
    },
    rt::TokioExecutor,
};
use serde_json::Value;
use url::{Host, Url};

use super::{Issue, PathSegment, validate};

const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BYTES: usize = 256 * 1024;
const MAX_REDIRECTS: usize = 3;
const USER_AGENT: &str = "ProtoCal (https://protocal.tsx.lt)";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchProblem {
    NotHttps,
    NotPublic,
    Unreachable(String),
    Status(u16),
    TooBig,
    NotJson,
    NotAnEvent(String),
    TooManyRedirects,
}

impl fmt::Display for FetchProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotHttps => write!(formatter, "is not an https link"),
            Self::NotPublic => write!(formatter, "points at a private address"),
            Self::Unreachable(reason) => write!(formatter, "could not be reached ({reason})"),
            Self::Status(status) => write!(formatter, "answered with HTTP {status}"),
            Self::TooBig => write!(formatter, "is larger than {} KB", MAX_BYTES / 1024),
            Self::NotJson => write!(formatter, "is not JSON"),
            Self::NotAnEvent(issue) => write!(formatter, "is not an Open Evnt event ({issue})"),
            Self::TooManyRedirects => write!(formatter, "redirects more than {MAX_REDIRECTS} times"),
        }
    }
}

fn is_public_v4(address: Ipv4Addr) -> bool {
    let [first, second, ..] = address.octets();
    let shared = first == 100 && (64..128).contains(&second);
    let benchmarking = first == 198 && (18..20).contains(&second);
    !(address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_broadcast()
        || address.is_documentation()
        || address.is_unspecified()
        || address.is_multicast()
        || shared
        || benchmarking
        || first == 0
        || first >= 240)
}

fn is_public_v6(address: Ipv6Addr) -> bool {
    if let Some(mapped) = address.to_ipv4_mapped() {
        return is_public_v4(mapped);
    }
    let first = address.segments()[0];
    let unique_local = first & 0xfe00 == 0xfc00;
    let link_local = first & 0xffc0 == 0xfe80;
    !(address.is_loopback() || address.is_unspecified() || address.is_multicast() || unique_local || link_local)
}

pub fn is_public(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_v4(address),
        IpAddr::V6(address) => is_public_v6(address),
    }
}

pub fn https_url_of(address: &str) -> Result<Url, FetchProblem> {
    let url = Url::parse(address.trim()).map_err(|_| FetchProblem::NotHttps)?;
    if url.scheme() != "https" {
        return Err(FetchProblem::NotHttps);
    }
    match url.host() {
        None => Err(FetchProblem::NotHttps),
        Some(Host::Ipv4(address)) if !is_public_v4(address) => Err(FetchProblem::NotPublic),
        Some(Host::Ipv6(address)) if !is_public_v6(address) => Err(FetchProblem::NotPublic),
        Some(_) => Ok(url),
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PublicAddressResolver;

impl tower_service::Service<Name> for PublicAddressResolver {
    type Response = std::vec::IntoIter<SocketAddr>;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, name: Name) -> Self::Future {
        Box::pin(async move {
            let found = tokio::net::lookup_host((name.as_str(), 0)).await?;
            let public: Vec<SocketAddr> = found.filter(|address| is_public(address.ip())).collect();
            if public.is_empty() {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "no public address"));
            }
            Ok(public.into_iter())
        })
    }
}

type HttpsClient = Client<HttpsConnector<HttpConnector<PublicAddressResolver>>, Empty<Bytes>>;

fn https_client() -> HttpsClient {
    let mut http = HttpConnector::new_with_resolver(PublicAddressResolver);
    http.enforce_http(false);
    http.set_connect_timeout(Some(TIMEOUT));
    let roots = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
    let tls = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("ring supports the default TLS versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(tls)
        .https_only()
        .enable_http1()
        .wrap_connector(http);
    Client::builder(TokioExecutor::new()).build(https)
}

fn issue_text(issue: &Issue) -> String {
    let path: Vec<String> = issue
        .path
        .iter()
        .map(|segment| match segment {
            PathSegment::Key(key) => key.clone(),
            PathSegment::Index(index) => index.to_string(),
        })
        .collect();
    if path.is_empty() { issue.message.clone() } else { format!("`{}` {}", path.join("."), issue.message) }
}

fn unreachable(error: impl fmt::Display) -> FetchProblem {
    FetchProblem::Unreachable(error.to_string())
}

async fn body_of(response: hyper::Response<hyper::body::Incoming>) -> Result<Bytes, FetchProblem> {
    let collected = tokio::time::timeout(TIMEOUT, Limited::new(response.into_body(), MAX_BYTES).collect())
        .await
        .map_err(|_| unreachable("it took too long"))?;
    collected.map(|body| body.to_bytes()).map_err(|error| {
        if error.is::<http_body_util::LengthLimitError>() { FetchProblem::TooBig } else { unreachable(error) }
    })
}

pub async fn fetch_event_document(address: &str) -> Result<Value, FetchProblem> {
    let client = https_client();
    let mut url = https_url_of(address)?;
    for _ in 0..=MAX_REDIRECTS {
        let request = Request::get(url.as_str())
            .header(header::USER_AGENT, USER_AGENT)
            .header(header::ACCEPT, "application/json")
            .body(Empty::new())
            .map_err(unreachable)?;
        let response = tokio::time::timeout(TIMEOUT, client.request(request))
            .await
            .map_err(|_| unreachable("it took too long"))?
            .map_err(unreachable)?;
        let status = response.status();
        if status.is_redirection() {
            let location = response.headers().get(header::LOCATION).and_then(|value| value.to_str().ok());
            let next = location.and_then(|location| url.join(location).ok());
            url = https_url_of(next.ok_or(FetchProblem::Status(status.as_u16()))?.as_str())?;
            continue;
        }
        if !status.is_success() {
            return Err(FetchProblem::Status(status.as_u16()));
        }
        let body = body_of(response).await?;
        let document: Value = serde_json::from_slice(&body).map_err(|_| FetchProblem::NotJson)?;
        return match validate(&document) {
            Ok(()) => Ok(document),
            Err(issues) => Err(FetchProblem::NotAnEvent(issue_text(&issues[0]))),
        };
    }
    Err(FetchProblem::TooManyRedirects)
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use tower_service::Service;

    use super::*;

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn only_public_addresses_are_public() {
        for private in ["127.0.0.1", "10.1.2.3", "192.168.1.1", "172.16.0.1", "169.254.1.1", "100.64.0.1", "0.0.0.0", "::1", "fd00::1", "fe80::1", "::ffff:127.0.0.1"] {
            assert!(!is_public(ip(private)), "{private}");
        }
        for public in ["1.1.1.1", "185.199.108.153", "2606:4700::1111", "::ffff:1.1.1.1"] {
            assert!(is_public(ip(public)), "{public}");
        }
    }

    #[test]
    fn only_https_links_to_public_hosts_are_fetched() {
        assert!(https_url_of("https://deniz.blue/events-data/2026/foss/fosdem26.evnt.json").is_ok());
        assert_eq!(https_url_of("http://deniz.blue/a.json"), Err(FetchProblem::NotHttps));
        assert_eq!(https_url_of("not a link"), Err(FetchProblem::NotHttps));
        assert_eq!(https_url_of("https://127.0.0.1/a.json"), Err(FetchProblem::NotPublic));
        assert_eq!(https_url_of("https://[::1]/a.json"), Err(FetchProblem::NotPublic));
    }

    #[tokio::test]
    async fn names_that_resolve_only_to_private_addresses_are_refused() {
        let error = PublicAddressResolver.call("localhost".parse().unwrap()).await.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        let problem = fetch_event_document("https://localhost:9/event.json").await.unwrap_err();
        assert!(matches!(problem, FetchProblem::Unreachable(_)), "{problem:?}");
    }
}
