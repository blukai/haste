use std::error::Error;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use bytes::Bytes;
use serde::Deserialize;

use crate::httpclient::HttpClient;

// thanks to Bulbasaur (/ johnpyp) for bringing up tv broadcasts in discord, see
// https://discord.com/channels/1275127765879754874/1276578605836668969/1289323757403504734; and
// for beginning implementing support for them in https://github.com/blukai/haste/pull/2.

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

/// replica of valve/stream's request headers. fell free to use those when constructing your
/// [`HttpClient`].
pub fn default_headers(app_id: u32) -> Result<http::HeaderMap, http::header::InvalidHeaderValue> {
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

    use http::header::{self, HeaderMap, HeaderValue};

    let mut headers: HeaderMap = Default::default();

    headers.insert(
        header::USER_AGENT,
        format!("Valve/Steam HTTP Client 1.0 ({})", app_id).try_into()?,
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

// ----
// http broadcast client

#[derive(Debug, Deserialize)]
pub struct SyncResponse {
    pub tick: i32, // tick can also be referred as start tick (nStartTick)
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
pub enum BroadcastHttpClientError<HttpClientError: Error + Send + Sync + 'static> {
    #[error("could not build request")]
    BuildRequestError(#[source] http::Error),
    #[error("http client error")]
    HttpClientError(#[from] HttpClientError),
    #[error("http status code error ({0})")]
    StatusCode(http::StatusCode),
    #[error("could not deserialize json")]
    JsonError(#[source] serde_json::Error),
}

pub struct BroadcasHttpClient<'client, C: HttpClient + 'client> {
    http_client: C,
    base_url: String,
    _marker: PhantomData<&'client ()>,
}

// TODO: don't ask for app_id + don't set default headers
impl<'client, C: HttpClient + 'client> BroadcasHttpClient<'client, C> {
    pub fn new(http_client: C, base_url: impl Into<String>) -> Self {
        Self {
            http_client,
            base_url: base_url.into(),
            _marker: PhantomData,
        }
    }

    async fn get(
        &self,
        url: &str,
    ) -> Result<http::Response<Result<Bytes, C::Error>>, BroadcastHttpClientError<C::Error>> {
        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri(url)
            .body(Bytes::default())
            .map_err(BroadcastHttpClientError::BuildRequestError)?;
        let response = self.http_client.execute(request).await?;
        if response.status().is_client_error() || response.status().is_server_error() {
            Err(BroadcastHttpClientError::StatusCode(response.status()))
        } else {
            Ok(response)
        }
    }

    // `SendGet( request, new CSyncRequest( m_SyncParams, nResync ) )`
    // call within the
    // `void CDemoStreamHttp::SendSync( int nResync )`
    pub async fn get_sync(&self) -> Result<SyncResponse, BroadcastHttpClientError<C::Error>> {
        let url = format!("{}/sync", &self.base_url);
        serde_json::from_slice(&self.get(&url).await?.into_body()?)
            .map_err(BroadcastHttpClientError::JsonError)
    }

    // `SendGet( CFmtStr( "/%d/start", m_SyncResponse.nSignupFragment ), new CStartRequest( ) )`
    // call within the
    // `bool CDemoStreamHttp::OnSync( int nResync )`.
    pub async fn get_start(
        &self,
        signup_fragment: i32,
    ) -> Result<Bytes, BroadcastHttpClientError<C::Error>> {
        assert!(signup_fragment >= 0);
        let url = format!("{}/{}/start", &self.base_url, signup_fragment);
        Ok(self.get(&url).await?.into_body()?)
    }

    // void CDemoStreamHttp::RequestFragment( int nFragment, FragmentTypeEnum_t nType )
    pub async fn get_fragment(
        &self,
        fragment: i32,
        typ: FragmentType,
    ) -> Result<Bytes, BroadcastHttpClientError<C::Error>> {
        let path = match typ {
            FragmentType::Delta => "delta",
            FragmentType::Full => "full",
        };
        let url = format!("{}/{}/{}", self.base_url, fragment, path);
        Ok(self.get(&url).await?.into_body()?)
    }
}

// ----
// http broadcast

// NOTE: on Bytes... Bytes type provides zero-copy cloning, meaning that cloned Bytes objects will
// reference the same underlying memory.
//
// TODO: is there a way to let the consumer supply their own buffer so that http client can write
// bodies directly into it?

// StreamStateEnum_t (not 1:1, but similar enough);
#[derive(Debug)]
enum StreamState {
    Start,
    Fullframe,
    Deltaframes {
        num_retries: u32,
        fetch_after: Instant,
        catchup: bool,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum BroadcastHttpError<HttpClientError: Error + Send + Sync + 'static> {
    #[error("could not get sync")]
    GetSyncError(#[source] BroadcastHttpClientError<HttpClientError>),
    #[error("could not get start")]
    GetStartError(#[source] BroadcastHttpClientError<HttpClientError>),
    #[error("could not {typ:?} fragment")]
    GetFragmentError {
        typ: FragmentType,
        #[source]
        source: BroadcastHttpClientError<HttpClientError>,
    },
}

pub struct BroadcastHttp<'client, C: HttpClient + 'client> {
    client: BroadcasHttpClient<'client, C>,
    stream_state: StreamState,
    stream_fragment: i32,
    keyframe_interval: Duration,
    sync_response: SyncResponse,
    stream_signup: Bytes,
}

impl<'client, C: HttpClient + 'client> BroadcastHttp<'client, C> {
    pub async fn start_streaming(
        http_client: C,
        base_url: impl Into<String>,
    ) -> Result<Self, BroadcastHttpError<C::Error>> {
        let client = BroadcasHttpClient::new(http_client, base_url);

        // in-game stream state flow:
        // -> STATE_START
        // -> STREAM_MAP_LOADED
        // -> STREAM_WAITING_FOR_KEYFRAME
        // -> STREAM_FULLFRAME
        // -> STREAM_BEFORE_DELTAFRAMES
        // -> STREAM_DELTAFRAMES

        let sync_response = client
            .get_sync()
            .await
            .map_err(BroadcastHttpError::GetSyncError)?;

        // bool CDemoStreamHttp::OnSync( int nResync )
        // DevMsg( "Broadcast: Buffering stream tick %d fragment %d signup fragment %d\n", m_SyncResponse.nStartTick, m_SyncResponse.nSignupFragment, m_SyncResponse.nSignupFragment );
        // m_nState = STATE_START;
        let stream_signup = client
            .get_start(sync_response.signup_fragment)
            .await
            .map_err(BroadcastHttpError::GetStartError)?;

        Ok(Self {
            client,
            stream_state: StreamState::Start,
            stream_fragment: sync_response.fragment,
            keyframe_interval: Duration::from_secs(sync_response.keyframe_interval as u64),
            sync_response,
            stream_signup,
        })
    }

    pub fn sync_response(&self) -> &SyncResponse {
        &self.sync_response
    }

    async fn next_deltaframe(&mut self) -> Result<Bytes, BroadcastHttpError<C::Error>> {
        // NOTE: loop simply allows to avoid going recursive, which is problematic in async context
        // and cannot be done without boxed futures.
        loop {
            let StreamState::Deltaframes {
                num_retries,
                fetch_after,
                catchup,
            } = self.stream_state
            else {
                unreachable!();
            };

            if !catchup {
                // this branch most likely never will be taken :thinking:
                if fetch_after.elapsed() > Duration::ZERO {
                    self.stream_state = StreamState::Deltaframes {
                        num_retries,
                        fetch_after,
                        catchup: true,
                    };
                    log::debug!("entering state: {:?}", self.stream_state);
                    continue;
                }

                let sleep_dur = fetch_after.duration_since(Instant::now());
                // TODO: async sleep
                std::thread::sleep(sleep_dur);
            }

            let start = Instant::now();
            match self
                .client
                .get_fragment(self.stream_fragment, FragmentType::Delta)
                .await
            {
                Ok(delta) => {
                    // NOTE: when state transitions from StreamState::Fullframe into
                    // StreamState::Deltaframes stream_fragment must not be incremented. both, full
                    // fragment and delta framgnet, are needed; otherwise it'll not be possible to
                    // parse packet entities.
                    self.stream_fragment += 1;
                    self.stream_state = StreamState::Deltaframes {
                        num_retries: 0,
                        fetch_after: start + self.keyframe_interval,
                        catchup,
                    };
                    log::debug!("entering state: {:?}", self.stream_state);

                    return Ok(delta);
                }

                Err(BroadcastHttpClientError::StatusCode(http::StatusCode::NOT_FOUND)) => {
                    if num_retries >= MAX_DELTAFRAME_RETRIES {
                        return Err(BroadcastHttpError::GetFragmentError {
                            typ: FragmentType::Delta,
                            source: BroadcastHttpClientError::StatusCode(
                                http::StatusCode::NOT_FOUND,
                            ),
                        });
                    }

                    self.stream_state = StreamState::Deltaframes {
                        num_retries: num_retries + 1,
                        fetch_after: start + self.keyframe_interval,
                        catchup: false,
                    };
                    log::debug!("entering state: {:?}", self.stream_state);

                    continue;
                }

                Err(source) => {
                    return Err(BroadcastHttpError::GetFragmentError {
                        typ: FragmentType::Delta,
                        source,
                    });
                }
            }
        }
    }

    // TODO: can a user pass buffer to http client to read body into?
    pub async fn next_packet(&mut self) -> Result<Bytes, BroadcastHttpError<C::Error>> {
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
                    .await
                    .map_err(|source| BroadcastHttpError::GetFragmentError {
                        typ: FragmentType::Full,
                        source,
                    })?;

                self.stream_state = StreamState::Deltaframes {
                    num_retries: 0,
                    fetch_after: Instant::now(),
                    catchup: true,
                };
                log::debug!("entering state: {:?}", self.stream_state);

                return Ok(full);
            }

            StreamState::Deltaframes { .. } => self.next_deltaframe().await,
        }
    }
}
