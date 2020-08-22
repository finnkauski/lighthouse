/// # Helpers
///
/// The helpers module contains functions that assist the rest of the codebase
/// it is unlikely that any of these internals will have to be used manually. pub mod helpers {// imports use std::net::IpAddr; use url::Url; /// Generates the target URL for the bridge pub fn generate_target(address: IpAddr, token: &str) -> Result<Url, ()> {let mut target = Url::parse("http://localhost").unwrap(); // Unwrap as it can't fail in parsing let path = format!("api/{}/", token); target.set_path(&path[..]); if target.set_ip_host(address).is_ok() {return Ok(target);} Err(())} pub mod network {use crate::lights::SendableState; use url::Url; /// Defines the allowed methods to be sent to bridge pub enum AllowedMethod {GET, PUT, POST,} /// Implementated to allow controlled conversion into reqwest /// methods and not allow undefined methods to be sent to bridge impl std::convert::From<AllowedMethod> for reqwest::Method {fn from(value: AllowedMethod) -> Self {match value {AllowedMethod::GET => reqwest::Method::GET, AllowedMethod::POST => reqwest::Method::POST, AllowedMethod::PUT => reqwest::Method::PUT,}}} /// This type alias is a URL and a type of Request to be sent pub type RequestTarget = (Url, AllowedMethod); /// Convenience type alias for a possible Result from the reqwest client type ResponseResult = Result<reqwest::Response, reqwest::Error>; type IndexedResponseResult = (usize, ResponseResult); /// Function wrapping the request sending functionality /// to a location. pub async fn send_request(request_target: RequestTarget, state: Option<&SendableState>, client: &reqwest::Client,) -> ResponseResult {let (target, method) = request_target; match method {AllowedMethod::POST => client.post(target).json(&state).send().await, AllowedMethod::GET => client.get(target).send().await, AllowedMethod::PUT => client.put(target).json(&state).send().await,}} pub async fn send_request_indexed(index: usize, request_target: RequestTarget, state: Option<&SendableState>, client: &reqwest::Client,) -> IndexedResponseResult {(index, send_request(request_target, state, client).await)} /// Function that sends off several states to the lights /// This is much more key than individual requests functionality provided by the /// send_request function as this is allowing us to do this asynchronously across /// an arbitrary selection of lights. pub async fn send_requests(request_targets: impl IntoIterator<Item = RequestTarget>, states: impl IntoIterator<Item = Option<&SendableState>>, client: &reqwest::Client,) -> Vec<ResponseResult> {use tokio::stream::StreamExt; let mut f: futures::stream::FuturesUnordered<_> = request_targets .into_iter() .zip(states.into_iter()) .enumerate() .map(|(i, (target, state))| send_request_indexed(i, target, state, client)) .collect(); let mut res = Vec::with_capacity(f.len()); while let Some(tup) = f.next().await {res.push(tup);} res.sort_by_key(|tuple| tuple.0); res.into_iter().map(|tup| tup.1).collect()}}}
// imports
use std::net::IpAddr;
use url::Url;

/// Generates the target URL for the bridge
pub fn generate_target(address: IpAddr, token: &str) -> Result<Url, ()> {
    let mut target = Url::parse("http://localhost").unwrap(); // Unwrap as it can't fail in parsing
    let path = format!("api/{}/", token);
    target.set_path(&path[..]);
    if target.set_ip_host(address).is_ok() {
        return Ok(target);
    }
    Err(())
}

pub mod network {
    use crate::lights::SendableState;
    use url::Url;

    /// Defines the allowed methods to be sent to bridge
    pub enum AllowedMethod {
        GET,
        PUT,
        POST,
    }

    /// Implementated to allow controlled conversion into reqwest
    /// methods and not allow undefined methods to be sent to bridge
    impl std::convert::From<AllowedMethod> for reqwest::Method {
        fn from(value: AllowedMethod) -> Self {
            match value {
                AllowedMethod::GET => reqwest::Method::GET,
                AllowedMethod::POST => reqwest::Method::POST,
                AllowedMethod::PUT => reqwest::Method::PUT,
            }
        }
    }

    /// This type alias is a URL and a type of Request to be sent
    pub type RequestTarget = (Url, AllowedMethod);

    /// Convenience type alias for a possible Result from the reqwest client
    type ResponseResult = Result<reqwest::Response, reqwest::Error>;
    type IndexedResponseResult = (usize, ResponseResult);

    /// Function wrapping the request sending functionality
    /// to a location.
    pub async fn send_request(
        request_target: RequestTarget,
        state: Option<&SendableState>,
        client: &reqwest::Client,
    ) -> ResponseResult {
        let (target, method) = request_target;
        match method {
            AllowedMethod::POST => client.post(target).json(&state).send().await,
            AllowedMethod::GET => client.get(target).send().await,
            AllowedMethod::PUT => client.put(target).json(&state).send().await,
        }
    }

    pub async fn send_request_indexed(
        index: usize,
        request_target: RequestTarget,
        state: Option<&SendableState>,
        client: &reqwest::Client,
    ) -> IndexedResponseResult {
        (index, send_request(request_target, state, client).await)
    }

    /// Function that sends off several states to the lights
    /// This is much more key than individual requests functionality provided by the
    /// send_request function as this is allowing us to do this asynchronously across
    /// an arbitrary selection of lights.
    pub async fn send_requests(
        request_targets: impl IntoIterator<Item = RequestTarget>,
        states: impl IntoIterator<Item = Option<&SendableState>>,
        client: &reqwest::Client,
    ) -> Vec<ResponseResult> {
        use tokio::stream::StreamExt;
        let mut f: futures::stream::FuturesUnordered<_> = request_targets
            .into_iter()
            .zip(states.into_iter())
            .enumerate()
            .map(|(i, (target, state))| send_request_indexed(i, target, state, client))
            .collect();
        let mut res = Vec::with_capacity(f.len());
        while let Some(tup) = f.next().await {
            res.push(tup);
        }
        res.sort_by_key(|tuple| tuple.0);
        res.into_iter().map(|tup| tup.1).collect()
    }
}
