use std::error::Error;
use std::io::{self, Read};
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use bytes::buf::Reader;
use bytes::{Buf, Bytes};
use haste_core::demostream::{CmdHeader, DemoStream};
use serde::Deserialize;
use valveprotos::common::{CDemoClassInfo, CDemoFullPacket, CDemoPacket, CDemoSendTables};

use crate::demostream::{
    decode_cmd_class_info, decode_cmd_packet, decode_cmd_send_tables, read_cmd_header,
    DecodeCmdError, ReadCmdHeaderError,
};
use crate::httpclient::HttpClient;

// thanks to Bulbasaur (/ johnpyp) for bringing up tv broadcasts in discord, see
// https://discord.com/channels/1275127765879754874/1276578605836668969/1289323757403504734; and
// for beginning implementing support for them in https://github.com/blukai/haste/pull/2.

// links to dig into:
// - https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Broadcast
// - https://github.com/saul/demofile-net/pull/93
// - https://github.com/FlowingSPDG/gotv-plus-go

// request examples:
// - http://dist1-ord1.steamcontent.com/tv/18895867/sync
// - http://dist1-ord1.steamcontent.com/tv/18895867/0/start
// - http://dist1-ord1.steamcontent.com/tv/18895867/295/full
// - http://dist1-ord1.steamcontent.com/tv/18895867/296/delta

// csgo sources to dig into:
// - engine/demostreamhttp.cpp
// - engine/cl_broadcast.cpp

// in-game stream state flow (cl_broadcast):
// -> STREAM_START
// -> STREAM_MAP_LOADED
// -> STREAM_WAITING_FOR_KEYFRAME
// -> STREAM_FULLFRAME
// -> STREAM_BEFORE_DELTAFRAMES
// -> STREAM_DELTAFRAMES

const MAX_DELTAFRAME_RETRIES: u32 = 5;

// from wiresharking deadlock:
//   GET /tv/18895867/sync HTTP/1.1\r\n
//   user-agent: Valve/Steam HTTP Client 1.0 (1422450)\r\n
//   Host: dist1-ord1.steamcontent.com\r\n
//   Accept: text/html,*/*;q=0.9\r\n
//   accept-encoding: gzip,identity,*;q=0\r\n
//   accept-charset: ISO-8859-1,utf-8,*;q=0.7\r\n
//   \r\n
//   [Response in frame: 4227]
//   [Full request URI: http://dist1-ord1.steamcontent.com/tv/18895867/sync]
//
/// replica of valve/stream's request headers. fell free to use those when constructing your
/// [`HttpClient`].
pub fn default_headers(app_id: u32) -> Result<http::HeaderMap, http::header::InvalidHeaderValue> {
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
// broadcast http client

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
// broadcast http

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
    #[error("could not get {typ:?} fragment")]
    GetFragmentError {
        typ: FragmentType,
        #[source]
        source: BroadcastHttpClientError<HttpClientError>,
    },
}

pub struct BroadcastHttp<'client, C: HttpClient + 'client> {
    client: BroadcasHttpClient<'client, C>,
    stream_fragment: i32,
    keyframe_interval: Duration,
    signup_fragment: i32,
    sync_response: SyncResponse,
    stream_state: StreamState,
    buf: Option<Reader<Bytes>>,
}

impl<'client, C: HttpClient + 'client> BroadcastHttp<'client, C> {
    pub async fn start_streaming(
        http_client: C,
        base_url: impl Into<String>,
    ) -> Result<Self, BroadcastHttpError<C::Error>> {
        let client = BroadcasHttpClient::new(http_client, base_url);

        let sync_response = client
            .get_sync()
            .await
            .map_err(BroadcastHttpError::GetSyncError)?;

        Ok(Self {
            client,
            stream_fragment: sync_response.fragment,
            keyframe_interval: Duration::from_secs(sync_response.keyframe_interval as u64),
            signup_fragment: sync_response.signup_fragment,
            sync_response,
            stream_state: StreamState::Start,
            buf: None,
        })
    }

    pub fn sync_response(&self) -> &SyncResponse {
        &self.sync_response
    }

    async fn handle_start(&mut self) -> Result<Bytes, BroadcastHttpError<C::Error>> {
        // bool CDemoStreamHttp::OnSync( int nResync )
        // DevMsg( "Broadcast: Buffering stream tick %d fragment %d signup fragment %d\n", m_SyncResponse.nStartTick, m_SyncResponse.nSignupFragment, m_SyncResponse.nSignupFragment );
        // m_nState = STATE_START;
        let stream_signup = self
            .client
            .get_start(self.signup_fragment)
            .await
            .map_err(BroadcastHttpError::GetStartError)?;

        self.stream_state = StreamState::Fullframe;
        log::debug!("entering state: {:?}", self.stream_state);

        Ok(stream_signup)
    }

    async fn handle_fullframe(&mut self) -> Result<Bytes, BroadcastHttpError<C::Error>> {
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

        Ok(full)
    }

    async fn handle_deltaframes(&mut self) -> Result<Bytes, BroadcastHttpError<C::Error>> {
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
                #[cfg(feature = "tokio")]
                tokio::time::sleep(sleep_dur).await;
                #[cfg(not(feature = "tokio"))]
                compile_error!("AAAAGH! can't sleep");
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

    // TODO: can a consumer pass buffer to http client to read body into to avoid allocations / or
    // what is the alternative?
    async fn next_packet(&mut self) -> Result<Bytes, BroadcastHttpError<C::Error>> {
        match self.stream_state {
            StreamState::Start => self.handle_start().await,
            StreamState::Fullframe => self.handle_fullframe().await,
            StreamState::Deltaframes { .. } => self.handle_deltaframes().await,
        }
    }

    pub async fn prepare_packet<'a>(
        &'a mut self,
    ) -> Result<impl Read + 'a, BroadcastHttpError<C::Error>> {
        if !matches!(self.buf.as_ref(), Some(buf) if buf.get_ref().has_remaining()) {
            self.buf = Some(self.next_packet().await?.reader());
        }
        Ok(self.buf.as_mut().unwrap())
    }
}

// ----
// demo stream

impl<'client, C: HttpClient + 'client> DemoStream for BroadcastHttp<'client, C> {
    type ReadCmdHeaderError = ReadCmdHeaderError;
    type ReadCmdError = io::Error;
    type DecodeCmdError = DecodeCmdError;

    // stream ops
    // ----

    fn seek(&mut self, _pos: std::io::SeekFrom) -> Result<u64, io::Error> {
        unimplemented!()
    }

    fn stream_position(&mut self) -> Result<u64, io::Error> {
        unimplemented!()
    }

    fn stream_len(&mut self) -> Result<u64, io::Error> {
        unimplemented!()
    }

    fn is_eof(&mut self) -> Result<bool, io::Error> {
        todo!()
    }

    // cmd header
    // ----
    //
    // cmd headers are broadcasts are similar to demo file cmd headers, but encoding is different.
    //
    // thanks to saul for figuring it out. see
    // https://github.com/saul/demofile-net/blob/7d3d59e478dbd2b000f4efa2dac70ed1bf2e2b7f/src/DemoFile/HttpBroadcastReader.cs#L150

    fn read_cmd_header(
        &mut self,
    ) -> Result<haste_core::demostream::CmdHeader, Self::ReadCmdHeaderError> {
        let mut rdr = pollster::block_on(self.prepare_packet())
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        read_cmd_header(&mut rdr)
    }

    fn unread_cmd_header(&mut self, _cmd_header: &CmdHeader) -> Result<(), io::Error> {
        unimplemented!()
    }

    // cmd
    // ----

    fn read_cmd(&mut self, cmd_header: &CmdHeader) -> Result<&[u8], Self::ReadCmdError> {
        // NOTE: current implementation is somewhat iffy. the idea/assumption is that
        // read_cmd_header was called right before and buf is at the correct position.

        let Some(buf) = self.buf.as_mut() else {
            unreachable!();
        };

        let size = cmd_header.body_size as usize;

        let bytes = buf.get_mut();
        // TODO: it might be not a bad idea to treat this not as an assertion, but as an error
        // because theoretically it could be possible that body of the response was not transferred
        // / read correctly?
        assert!(bytes.remaining() >= size);

        // SAFETY: safe rust does not allowe to return a slice of bytes because slice lives in the
        // current scope where bytes are borrowed, thus unsafe shenanigans are needed to avoid
        // copying. this is actually safe, i think.
        let data = unsafe {
            let ptr = bytes
                // NOTE: start is 0 because Reader advances position of underlying Bytes
                .slice(0..size)
                .as_ref()
                .as_ptr();
            std::slice::from_raw_parts(ptr, size)
        };

        bytes.advance(size);

        Ok(data)
    }

    #[inline(always)]
    fn decode_cmd_send_tables(data: &[u8]) -> Result<CDemoSendTables, Self::DecodeCmdError> {
        decode_cmd_send_tables(data)
    }

    #[inline(always)]
    fn decode_cmd_class_info(data: &[u8]) -> Result<CDemoClassInfo, Self::DecodeCmdError> {
        decode_cmd_class_info(data)
    }

    #[inline(always)]
    fn decode_cmd_packet(data: &[u8]) -> Result<CDemoPacket, Self::DecodeCmdError> {
        decode_cmd_packet(data)
    }

    fn decode_cmd_full_packet(_data: &[u8]) -> Result<CDemoFullPacket, Self::DecodeCmdError> {
        unimplemented!()
    }

    fn skip_cmd(&mut self, _cmd_header: &CmdHeader) -> Result<(), io::Error> {
        unimplemented!()
    }
}
