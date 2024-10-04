use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail};
use bytes::Bytes;
use serde::Deserialize;

use crate::HttpClient;

// TODO: figure DemoStream trait

// links to dig into:
// - https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Broadcast
// - https://github.com/saul/demofile-net/pull/93
// - https://github.com/FlowingSPDG/gotv-plus-go

// csgo sources to dig into:
// - engine/demostreamhttp.cpp
// - engine/cl_broadcast.cpp

// request examples:
// - http://dist1-ord1.steamcontent.com/tv/18895867/sync
// - http://dist1-ord1.steamcontent.com/tv/18895867/0/start
// - http://dist1-ord1.steamcontent.com/tv/18895867/295/full
// - http://dist1-ord1.steamcontent.com/tv/18895867/296/delta

const MAX_DELTAFRAME_RETRIES: u32 = 5;

#[derive(Debug)]
#[repr(u32)]
pub enum AppId {
    Deadlock = 1422450,
}

// from wiresharking deadlock:
// Hypertext Transfer Protocol
//     GET /tv/18895867/sync HTTP/1.1\r\n
//     user-agent: Valve/Steam HTTP Client 1.0 (1422450)\r\n
//     Host: dist1-ord1.steamcontent.com\r\n
//     Accept: text/html,*/*;q=0.9\r\n
//     accept-encoding: gzip,identity,*;q=0\r\n
//     accept-charset: ISO-8859-1,utf-8,*;q=0.7\r\n
//     \r\n
//     [Response in frame: 4227]
//     [Full request URI: http://dist1-ord1.steamcontent.com/tv/18895867/sync]

fn default_headers(app_id: AppId) -> Result<http::HeaderMap, http::header::InvalidHeaderValue> {
    use http::header::{self, HeaderMap, HeaderValue};

    let mut headers: HeaderMap = Default::default();

    headers.insert(
        header::USER_AGENT,
        format!("Valve/Steam HTTP Client 1.0 ({})", app_id as u32).try_into()?,
    );
    headers.insert(
        header::ACCEPT,
        HeaderValue::from_static("text/html,*/*;q=0.9"),
    );
    headers.insert(
        header::ACCEPT_ENCODING,
        HeaderValue::from_static("gzip,identity,*;q=0"),
    );
    headers.insert(
        header::ACCEPT_CHARSET,
        HeaderValue::from_static("ISO-8859-1,utf-8,*;q=0.7"),
    );

    Ok(headers)
}

#[derive(Debug, Deserialize)]
pub struct SyncResponse {
    pub tick: i32,
    pub endtick: i32,
    pub maxtick: i32,
    pub rtdelay: f32,
    pub rcvage: f32,
    pub fragment: i32,
    pub signup_fragment: i32,
    pub tps: i32,
    pub keyframe_interval: i32,
    pub map: String,
    pub protocol: i32,
}

#[derive(Debug)]
pub enum FragmentType {
    Delta,
    Full,
}

#[derive(thiserror::Error, Debug)]
pub enum HttpBroadcastClientError<HttpClientError: std::error::Error + Send + Sync + 'static> {
    #[error("could not build request: {0}")]
    BuildRequestError(http::Error),
    #[error("http client error: {0}")]
    HttpClientError(#[from] HttpClientError),
    #[error("http status code error: {0}")]
    StatusCode(http::StatusCode),
    #[error("could not deserialize json: {0}")]
    DeserializeJsonError(serde_json::Error),
}

fn is_result_not_found<T, HttpClientError: std::error::Error + Send + Sync + 'static>(
    result: &Result<T, HttpBroadcastClientError<HttpClientError>>,
) -> bool {
    match result {
        Err(HttpBroadcastClientError::StatusCode(http::StatusCode::NOT_FOUND)) => true,
        _ => false,
    }
}

pub struct HttpBroadcastClient<'client, C: HttpClient + 'client> {
    http_client: C,
    base_uri: String,
    default_headers: http::HeaderMap,
    _marker: PhantomData<&'client ()>,
}

impl<'client, C: HttpClient + 'client> HttpBroadcastClient<'client, C> {
    pub fn new(http_client: C, base_uri: impl Into<String>, app_id: AppId) -> Self {
        Self {
            http_client,
            base_uri: base_uri.into(),
            default_headers: default_headers(app_id)
                // NOTE: if this fails bail out loudly - this is not a user error, the whole thing
                // is broken.
                .expect("default headers are fucked"),
            _marker: PhantomData,
        }
    }

    async fn get(
        &self,
        url: &str,
    ) -> Result<http::Response<Result<Bytes, C::Error>>, HttpBroadcastClientError<C::Error>> {
        let mut request_builder = http::Request::builder();
        *request_builder
            .headers_mut()
            // NOTE: this should not fail, the request was just constructed, it contains no errors.
            .expect("could not get request headers") = self.default_headers.clone();
        let request = request_builder
            .method(http::Method::GET)
            .uri(url)
            .body(Bytes::default())
            .map_err(HttpBroadcastClientError::BuildRequestError)?;

        let response = self.http_client.execute(request).await?;
        if response.status().is_client_error() || response.status().is_server_error() {
            Err(HttpBroadcastClientError::StatusCode(response.status()))
        } else {
            Ok(response)
        }
    }

    // `SendGet( request, new CSyncRequest( m_SyncParams, nResync ) )`
    // call within the
    // `void CDemoStreamHttp::SendSync( int nResync )`
    pub async fn get_sync(&self) -> Result<SyncResponse, HttpBroadcastClientError<C::Error>> {
        let url = format!("{}/sync", &self.base_uri);
        serde_json::from_slice(&self.get(&url).await?.into_body()?)
            .map_err(HttpBroadcastClientError::DeserializeJsonError)
    }

    // `SendGet( CFmtStr( "/%d/start", m_SyncResponse.nSignupFragment ), new CStartRequest( ) )`
    // call within the
    // `bool CDemoStreamHttp::OnSync( int nResync )`.
    pub async fn get_start(
        &self,
        signup_fragment: i32,
    ) -> Result<Bytes, HttpBroadcastClientError<C::Error>> {
        assert!(signup_fragment >= 0);
        let url = format!("{}/{}/start", &self.base_uri, signup_fragment);
        Ok(self.get(&url).await?.into_body()?)
    }

    // void CDemoStreamHttp::RequestFragment( int nFragment, FragmentTypeEnum_t nType )
    pub async fn get_fragment(
        &self,
        fragment: i32,
        typ: FragmentType,
    ) -> Result<Bytes, HttpBroadcastClientError<C::Error>> {
        let path = match typ {
            FragmentType::Delta => "delta",
            FragmentType::Full => "full",
        };
        let url = format!("{}/{}/{}", self.base_uri, fragment, path);
        Ok(self.get(&url).await?.into_body()?)
    }
}

// ----

// StreamStateEnum_t (not 1:1, but similar enough);
#[derive(Debug)]
enum StreamState {
    Start,
    Fullframe,
    DeltaframesCatchup { catchup_fragments: i32 },
    DeltaframesIntervaled,
    DeltaframeRetry,
}

// NOTE: on Bytes... Bytes type provides zero-copy cloning, meaning that cloned Bytes objects will
// reference the same underlying memory.

pub struct HttpBroadcast<'client, C: HttpClient + 'client> {
    client: HttpBroadcastClient<'client, C>,
    stream_fragment: i32,
    sync_response: SyncResponse,
    stream_signup: Bytes,
    stream_state: StreamState,
    last_deltaframe_fetch: Option<Instant>,
}

impl<'client, C: HttpClient + 'client> HttpBroadcast<'client, C> {
    pub async fn start_streaming(
        http_client: C,
        base_uri: impl Into<String>,
        app_id: AppId,
    ) -> Result<Self, anyhow::Error> {
        let client = HttpBroadcastClient::new(http_client, base_uri, app_id);

        let sync_response = client
            .get_sync()
            .await
            .map_err(|err| anyhow!("could not fetch sync: {err}"))?;
        if sync_response.keyframe_interval < 0 {
            bail!(
                "sync response contains invalid keyframe interval {}",
                sync_response.keyframe_interval
            );
        }

        let stream_signup = client
            .get_start(sync_response.signup_fragment)
            .await
            .map_err(|err| anyhow!("could not fetch start: {err}"))?;

        Ok(Self {
            client,
            stream_fragment: sync_response.fragment,
            sync_response,
            stream_signup,
            stream_state: StreamState::Start,
            last_deltaframe_fetch: None,
        })
    }

    // NOTE: usually there's approximately 6-9 deltaframes sitting and waiting at the beginning,
    // the number seem to match following math:
    // `(max_tick - end_tick) / tps / keyframe_interval`
    //
    // NOTE: in deadlock when you join a live match as a spectator deadlock will fetch several
    // delta fragments with no interval in between. result of this helps to approximately replicate
    // deadlock's behavior.
    fn catchup_fragments(&self) -> i32 {
        let SyncResponse {
            maxtick,
            endtick,
            tps,
            keyframe_interval,
            ..
        } = self.sync_response;
        (maxtick - endtick) / tps / keyframe_interval
    }

    // TODO: can a user pass buffer to http client to read body into?
    pub async fn next_packet(&mut self) -> Result<Bytes, anyhow::Error> {
        let mut deltaframe_retries = 0;

        // NOTE: loop simply allows to avoid going recursive, which is problematic in async context
        // and cannot be cone without boxed futures.
        loop {
            match self.stream_state {
                StreamState::Start => {
                    self.stream_state = StreamState::Fullframe;
                    log::debug!("entering state: {:?}", self.stream_state);
                    return Ok(self.stream_signup.clone());
                }

                StreamState::Fullframe => {
                    let full = self
                        .client
                        .get_fragment(self.stream_fragment, FragmentType::Full)
                        .await?;

                    let catchup_fragments = self.catchup_fragments();
                    self.stream_state = StreamState::DeltaframesCatchup { catchup_fragments };
                    log::debug!("entering state: {:?}", self.stream_state);

                    return Ok(full);
                }

                StreamState::DeltaframesCatchup {
                    mut catchup_fragments,
                } => {
                    self.stream_fragment += 1;
                    let start = Instant::now();
                    let result = self
                        .client
                        .get_fragment(self.stream_fragment, FragmentType::Delta)
                        .await;
                    if is_result_not_found(&result) {
                        self.stream_state = StreamState::DeltaframesIntervaled;
                        log::debug!("entering state: {:?}", self.stream_state);
                        continue;
                    };

                    let delta = result?;
                    self.last_deltaframe_fetch = Some(start);

                    catchup_fragments -= 1;
                    if catchup_fragments == 0 {
                        self.stream_state = StreamState::DeltaframesIntervaled;
                        log::debug!("entering state: {:?}", self.stream_state);
                    } else {
                        self.stream_state = StreamState::DeltaframesCatchup { catchup_fragments }
                    }

                    return Ok(delta);
                }

                StreamState::DeltaframesIntervaled => {
                    let elapsed = self
                        .last_deltaframe_fetch
                        .map(|i| i.elapsed())
                        .unwrap_or_default();
                    let interval = Duration::from_secs(self.sync_response.keyframe_interval as u64);

                    if elapsed > interval {
                        let catchup_fragments = (elapsed.as_secs() / interval.as_secs()) as i32;
                        self.stream_state = StreamState::DeltaframesCatchup { catchup_fragments };
                        log::debug!("entering state: {:?}", self.stream_state,);
                        continue;
                    }

                    let dur_sleep = interval - elapsed;
                    // TODO: async sleep
                    std::thread::sleep(dur_sleep);

                    self.stream_fragment += 1;
                    let start = Instant::now();
                    let result = self
                        .client
                        .get_fragment(self.stream_fragment, FragmentType::Delta)
                        .await;
                    if is_result_not_found(&result) {
                        self.stream_state = StreamState::DeltaframeRetry;
                        log::debug!("entering state: {:?}", self.stream_state);
                        continue;
                    }

                    let delta = result?;
                    self.last_deltaframe_fetch = Some(start);
                    return Ok(delta);
                }

                // NOTE: this arm is expected to nearly never be visited
                StreamState::DeltaframeRetry => {
                    let dur_sleep =
                        Duration::from_secs(self.sync_response.keyframe_interval as u64);
                    // TODO: async sleep
                    std::thread::sleep(dur_sleep);

                    let start = Instant::now();
                    let result = self
                        .client
                        .get_fragment(self.stream_fragment, FragmentType::Delta)
                        .await;
                    if is_result_not_found(&result) {
                        deltaframe_retries += 1;
                        if deltaframe_retries > MAX_DELTAFRAME_RETRIES {
                            bail!(
                                "could not fetch detlaframe (stream fragment {}; tries {})",
                                self.stream_fragment,
                                deltaframe_retries,
                            );
                        }
                        continue;
                    }

                    let delta = result?;
                    self.last_deltaframe_fetch = Some(start);

                    self.stream_state = StreamState::DeltaframesIntervaled;
                    log::debug!("entering state: {:?}", self.stream_state);

                    return Ok(delta);
                }
            }
        }
    }
}
