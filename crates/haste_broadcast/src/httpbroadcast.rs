use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail};
use bytes::Bytes;
use serde::Deserialize;

use crate::HttpClient;

// useful links to dig into:
// - https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Broadcast
// - https://github.com/saul/demofile-net/pull/93
// - https://github.com/FlowingSPDG/gotv-plus-go

// example requests (ordered):
// - http://dist1-ord1.steamcontent.com/tv/18895867/sync
// - http://dist1-ord1.steamcontent.com/tv/18895867/0/start
// - http://dist1-ord1.steamcontent.com/tv/18895867/295/full
// - http://dist1-ord1.steamcontent.com/tv/18895867/296/delta

// how csgo gets to a packet:
// - ...
// - _Host_RunFrame_Client
// - CL_ReadPackets
// - CNetChan::ProcessPlayback:
//   while ( ( packet = demoplayer->ReadPacket() ) != NULL )
// - CBroadcastPlayer::ReadPacket:
//   while ( !bStopReading ) {
//     if ( !PreparePacket() ) return NULL; // packet is not ready
//     ...
// - CBroadcastPlayer::PreparePacket
//
// CDemoStreamHttp flow:
// - CDemoStreamHttp::StartStreaming
// - SendSync
// - CDemoStreamHttp::OnSync
//   SendGet( CFmtStr( "/%d/start", m_SyncResponse.nSignupFragment ), new CStartRequest( ) )
// - CDemoStreamHttp::OnStart
//   m_pClient->OnDemoStreamStart( GetStreamStartReference(), 0 )

#[derive(Debug)]
#[repr(u32)]
pub enum AppId {
    Deadlock = 1422450,
}

// request-response stuff
// ----

#[derive(Debug, Deserialize)]
pub struct SyncResponse {
    pub tick: Option<i32>,
    pub endtick: Option<i32>,
    pub maxtick: Option<i32>,
    pub rtdelay: Option<f32>,
    pub rcvage: Option<f32>,
    pub fragment: Option<i32>,
    pub signup_fragment: Option<i32>,
    pub tps: Option<i32>,
    pub keyframe_interval: Option<i32>,
    pub map: Option<String>,
    pub protocol: Option<i32>,
}

#[derive(Debug)]
pub enum FragmentType {
    Delta,
    Full,
}

impl FragmentType {
    fn as_path_part(&self) -> &'static str {
        match self {
            FragmentType::Delta => "delta",
            FragmentType::Full => "full",
        }
    }
}

// http broadcast client
// ----

// from wiresharking deadlock requests:
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
    use http::header;

    let mut headers: http::HeaderMap<http::HeaderValue> = Default::default();

    let user_agent = &format!("Valve/Steam HTTP Client 1.0 ({})", app_id as u32);
    let accept = "text/html,*/*;q=0.9";
    let accept_encoding = "gzip,identity,*;q=0";
    let accept_charset = "ISO-8859-1,utf-8,*;q=0.7";

    headers.insert(header::USER_AGENT, user_agent.try_into()?);
    headers.insert(header::ACCEPT, accept.try_into()?);
    headers.insert(header::ACCEPT_ENCODING, accept_encoding.try_into()?);
    headers.insert(header::ACCEPT_CHARSET, accept_charset.try_into()?);

    Ok(headers)
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
                // NOTE: it is okay to call expect here because `default_headers` is an internal
                // function and it is possible to guarantee that it'll not be bad. if this fails
                // bail out loudly - this is not a user error, the whole thing is broken.
                .expect("default headers are fucked"),
            _marker: PhantomData,
        }
    }

    async fn execute_get(
        &self,
        uri: String,
    ) -> Result<http::Response<Result<Bytes, C::Error>>, HttpBroadcastClientError<C::Error>> {
        let mut request_builder = http::Request::builder();
        *request_builder
            .headers_mut()
            // NOTE: this should not fail, the request was just constructed, it contains no errors.
            .expect("could not get request headers") = self.default_headers.clone();
        let request = request_builder
            .method(http::Method::GET)
            .uri(uri)
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
    pub async fn fetch_sync(&self) -> Result<SyncResponse, HttpBroadcastClientError<C::Error>> {
        let uri = format!("{}/sync", &self.base_uri);
        serde_json::from_slice(&self.execute_get(uri).await?.into_body()?)
            .map_err(HttpBroadcastClientError::DeserializeJsonError)
    }

    // `SendGet( CFmtStr( "/%d/start", m_SyncResponse.nSignupFragment ), new CStartRequest( ) )`
    // call within the
    // `bool CDemoStreamHttp::OnSync( int nResync )`.
    pub async fn fetch_start(
        &self,
        signup_fragment: i32,
    ) -> Result<Bytes, HttpBroadcastClientError<C::Error>> {
        assert!(signup_fragment >= 0);
        let uri = format!("{}/{}/start", &self.base_uri, signup_fragment);
        Ok(self.execute_get(uri).await?.into_body()?)
    }

    // void CDemoStreamHttp::RequestFragment( int nFragment, FragmentTypeEnum_t nType )
    pub async fn fetch_fragment(
        &self,
        fragment: i32,
        typ: FragmentType,
    ) -> Result<Bytes, HttpBroadcastClientError<C::Error>> {
        let uri = format!("{}/{}/{}", self.base_uri, fragment, typ.as_path_part());
        Ok(self.execute_get(uri).await?.into_body()?)
    }
}

// http broadcast
// ----

const DEFAULT_KEYFRAME_INTERVAL: i32 = 3;
const MAX_DELTAFRAME_RETRIES: u32 = 5;

// enum StreamStateEnum_t
#[derive(Debug)]
enum StreamState {
    Start,
    Fullframe,
    BufferingDeltaframes, // non-valve
    Deltaframes,
}

// NOTE: on Bytes... Bytes type provides zero-copy cloning, meaning that cloned Bytes objects will
// reference the same underlying memory.

pub struct HttpBroadcast<'client, C: HttpClient + 'client> {
    client: HttpBroadcastClient<'client, C>,
    stream_fragment: i32,
    sync_response: SyncResponse,
    stream_signup: Bytes,
    stream_state: StreamState,
    last_delta_fetch: Option<Instant>,
}

impl<'client, C: HttpClient + 'client> HttpBroadcast<'client, C> {
    pub async fn start_streaming(
        http_client: C,
        base_uri: impl Into<String>,
        app_id: AppId,
    ) -> Result<Self, anyhow::Error> {
        let client = HttpBroadcastClient::new(http_client, base_uri, app_id);

        // CDemoStreamHttp.m_nState = STATE_SYNC;
        // CBroadcastPlayer.m_nStreamState = STREAM_SYNC;
        let sync_response = client
            .fetch_sync()
            .await
            .map_err(|err| anyhow!("could not fetch sync: {err}"))?;

        // CDemoStreamHttp.m_nState = STATE_START;
        // CBroadcastPlayer.m_nStreamState = nResync ? STREAM_FULLFRAME : STREAM_START;
        let stream_signup = client
            .fetch_start(sync_response.signup_fragment.unwrap_or(0))
            .await
            .map_err(|err| anyhow!("could not fetch start: {err}"))?;

        Ok(Self {
            client,
            stream_fragment: sync_response.fragment.unwrap_or(0),
            sync_response,
            stream_signup,
            stream_state: StreamState::Start,
            last_delta_fetch: None,
        })
    }

    fn keyframe_interval(&self) -> i32 {
        let keyframe_interval = self
            .sync_response
            .keyframe_interval
            .unwrap_or(DEFAULT_KEYFRAME_INTERVAL);
        assert!(keyframe_interval >= 0);
        keyframe_interval
    }

    // bool CBroadcastPlayer::PreparePacket( void )
    pub async fn prepare_packet(&mut self) -> Result<Bytes, anyhow::Error> {
        // NOTE: loop simply allows to avoid going recursive, which is problematic in async context
        // and cannot be cone without boxed futures.
        let mut num_deltaframe_retry: u32 = 0;
        loop {
            // TODO: logging. first!
            dbg!(&self.stream_state);
            match self.stream_state {
                StreamState::Start => {
                    self.stream_state = StreamState::Fullframe;
                    break Ok(self.stream_signup.clone());
                }

                StreamState::Fullframe => {
                    let full = self
                        .client
                        .fetch_fragment(self.stream_fragment, FragmentType::Full)
                        .await?;
                    self.stream_state = StreamState::BufferingDeltaframes;
                    break Ok(full);
                }

                // NOTE: usually there's approximately 6-9 delta frames sitting and waiting at the
                // beginning. the number seem to match following math:
                // `(sync_response.max_tick - sync_response.end_tick) / sync_response.tps`.
                //
                // what differs StreamState::BufferingDeltaframes from StreamState::Deltaframes is
                // that it doesn't wait before fetching next frame and if error occurs it switches
                // state into StreamState::Deltaframes.
                StreamState::BufferingDeltaframes => 'buffering: loop {
                    let start = Instant::now();
                    match self
                        .client
                        .fetch_fragment(self.stream_fragment + 1, FragmentType::Delta)
                        .await
                    {
                        Ok(delta) => {
                            self.stream_fragment += 1;
                            self.last_delta_fetch = Some(start);
                            return Ok(delta);
                        }
                        Err(_) => {
                            self.stream_state = StreamState::Deltaframes;
                            break 'buffering;
                        }
                    }
                },

                StreamState::Deltaframes => {
                    if let Some(last_delta_fetch) = self.last_delta_fetch {
                        let elapsed = last_delta_fetch.elapsed();
                        let interval = Duration::from_secs(self.keyframe_interval() as u64);

                        // TODO: might want to adjust this (maybe more then 2 keyframe intervals?)
                        if elapsed > interval && num_deltaframe_retry == 0 {
                            // TODO: do not switch into buffering deltaframes state if retrying
                            // because of 404
                            self.stream_state = StreamState::BufferingDeltaframes;
                            continue;
                        }

                        let sleep_dur = interval - elapsed;
                        // TODO: async sleep
                        std::thread::sleep(sleep_dur);
                    }

                    let start = Instant::now();
                    match self
                        .client
                        .fetch_fragment(self.stream_fragment + 1, FragmentType::Delta)
                        .await
                    {
                        Ok(delta) => {
                            self.stream_fragment += 1;
                            self.last_delta_fetch = Some(start);
                            break Ok(delta);
                        }
                        Err(HttpBroadcastClientError::StatusCode(http::StatusCode::NOT_FOUND)) => {
                            num_deltaframe_retry += 1;
                            if num_deltaframe_retry > MAX_DELTAFRAME_RETRIES {
                                bail!(
                                    "could not fetch detla (fragment {}, retries {})",
                                    self.stream_fragment + 1,
                                    num_deltaframe_retry - 1
                                );
                            }
                            continue;
                        }
                        Err(err) => {
                            break Err(err.into());
                        }
                    }
                }
            }
        }
    }
}
