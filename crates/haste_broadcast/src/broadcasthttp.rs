use std::error::Error;
use std::io::{self, BufRead, Cursor, Seek, SeekFrom, Write};
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use bytes::buf::Reader;
use bytes::{Buf, Bytes};
use haste_core::demostream::{
    CmdHeader, DecodeCmdError, DemoStream, ReadCmdError, ReadCmdHeaderError,
};
use serde::Deserialize;
use valveprotos::common::{CDemoClassInfo, CDemoFullPacket, CDemoPacket, CDemoSendTables};

use crate::demostream::{
    decode_cmd_class_info, decode_cmd_full_packet, decode_cmd_packet, decode_cmd_send_tables,
    read_cmd_header, scan_for_last_tick,
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
    /// start tick of the current fragment
    pub tick: i32,
    pub endtick: i32,
    pub maxtick: i32,
    /// delay of this fragment from real-time, seconds
    pub rtdelay: f32,
    /// receive age: how many seconds since relay last received data from game server
    pub rcvage: f32,
    pub fragment: i32,
    /// numeric value index of signup fragment
    pub signup_fragment: i32,
    pub tps: i32,
    /// the interval between full keyframes, in seconds
    pub keyframe_interval: i32,
    pub map: String,
    pub protocol: i32,
}

#[derive(Debug)]
pub enum FragmentType {
    Delta,
    Full,
}

// TODO: move all BroadcastHttpClient's methods into BroadcastHttp and get rid of
// BroadcasHttpClient.

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

struct BroadcasHttpClient<'client, C: HttpClient + 'client> {
    http_client: C,
    base_url: String,
    _marker: PhantomData<&'client ()>,
}

impl<'client, C: HttpClient + 'client> BroadcasHttpClient<'client, C> {
    fn new(http_client: C, base_url: impl Into<String>) -> Self {
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
    async fn get_sync(&self) -> Result<SyncResponse, BroadcastHttpClientError<C::Error>> {
        let url = format!("{}/sync", &self.base_url);
        serde_json::from_slice(&self.get(&url).await?.into_body()?)
            .map_err(BroadcastHttpClientError::JsonError)
    }

    // `SendGet( CFmtStr( "/%d/start", m_SyncResponse.nSignupFragment ), new CStartRequest( ) )`
    // call within the
    // `bool CDemoStreamHttp::OnSync( int nResync )`.
    async fn get_start(
        &self,
        signup_fragment: i32,
    ) -> Result<Bytes, BroadcastHttpClientError<C::Error>> {
        assert!(signup_fragment >= 0);
        let url = format!("{}/{}/start", &self.base_url, signup_fragment);
        Ok(self.get(&url).await?.into_body()?)
    }

    // void CDemoStreamHttp::RequestFragment( int nFragment, FragmentTypeEnum_t nType )
    async fn get_fragment(
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

// StreamStateEnum_t (not 1:1, but similar enough);
#[derive(Debug)]
enum StreamState {
    Stop,
    Start,
    Fullframe,
    Deltaframes {
        num_retries: u32,
        fetch_after: Instant,
        catchup: bool,
    },
}

enum StreamBuffer {
    Last(Option<Reader<Bytes>>),
    Seekable(Cursor<Vec<u8>>),
}

pub struct BroadcastHttp<'client, C: HttpClient + 'client> {
    client: BroadcasHttpClient<'client, C>,
    stream_fragment: i32,
    keyframe_interval: Duration,
    signup_fragment: i32,
    sync_response: SyncResponse,
    stream_state: StreamState,
    stream_buffer: StreamBuffer,
    total_ticks: Option<i32>,
}

impl<'client, C: HttpClient + 'client> BroadcastHttp<'client, C> {
    pub async fn start_streaming(
        http_client: C,
        base_url: impl Into<String>,
    ) -> Result<Self, BroadcastHttpClientError<C::Error>> {
        let client = BroadcasHttpClient::new(http_client, base_url);

        let sync_response = client.get_sync().await?;

        Ok(Self {
            client,
            stream_fragment: sync_response.fragment,
            keyframe_interval: Duration::from_secs(sync_response.keyframe_interval as u64),
            signup_fragment: sync_response.signup_fragment,
            sync_response,
            stream_state: StreamState::Start,
            stream_buffer: StreamBuffer::Last(None),
            total_ticks: None,
        })
    }

    /// buffer all packets. this enables seeking on [`DemoStream`].
    pub async fn start_streaming_and_buffer(
        http_client: C,
        base_url: impl Into<String>,
    ) -> Result<Self, BroadcastHttpClientError<C::Error>> {
        let mut this = Self::start_streaming(http_client, base_url).await?;
        this.stream_buffer = StreamBuffer::Seekable(Cursor::default());
        Ok(this)
    }

    pub fn sync_response(&self) -> &SyncResponse {
        &self.sync_response
    }

    async fn handle_start(&mut self) -> Result<Bytes, BroadcastHttpClientError<C::Error>> {
        // bool CDemoStreamHttp::OnSync( int nResync )
        // DevMsg( "Broadcast: Buffering stream tick %d fragment %d signup fragment %d\n", m_SyncResponse.nStartTick, m_SyncResponse.nSignupFragment, m_SyncResponse.nSignupFragment );
        // m_nState = STATE_START;
        let stream_signup = self.client.get_start(self.signup_fragment).await?;

        self.stream_state = StreamState::Fullframe;
        log::debug!("entering state: {:?}", self.stream_state);

        Ok(stream_signup)
    }

    async fn handle_fullframe(&mut self) -> Result<Bytes, BroadcastHttpClientError<C::Error>> {
        let full = self
            .client
            .get_fragment(self.stream_fragment, FragmentType::Full)
            .await?;

        self.stream_state = StreamState::Deltaframes {
            num_retries: 0,
            fetch_after: Instant::now(),
            catchup: true,
        };
        log::debug!("entering state: {:?}", self.stream_state);

        Ok(full)
    }

    async fn handle_deltaframes(&mut self) -> Result<Bytes, BroadcastHttpClientError<C::Error>> {
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
                        return Err(BroadcastHttpClientError::StatusCode(
                            http::StatusCode::NOT_FOUND,
                        ));
                    }

                    self.stream_state = StreamState::Deltaframes {
                        num_retries: num_retries + 1,
                        fetch_after: start + self.keyframe_interval,
                        catchup: false,
                    };
                    log::debug!("entering state: {:?}", self.stream_state);

                    continue;
                }

                Err(source) => return Err(source),
            }
        }
    }

    // TODO: can a consumer pass buffer to http client to read body into to avoid allocations / or
    // what is the alternative?
    pub async fn next_packet(
        &mut self,
    ) -> Option<Result<Bytes, BroadcastHttpClientError<C::Error>>> {
        match match self.stream_state {
            StreamState::Stop => return None,
            StreamState::Start => self.handle_start().await,
            StreamState::Fullframe => self.handle_fullframe().await,
            StreamState::Deltaframes { .. } => self.handle_deltaframes().await,
        } {
            Ok(packet) => {
                match self.stream_buffer {
                    StreamBuffer::Last(ref mut value) => {
                        use bytes::Buf;

                        *value = Some(
                            packet
                                // NOTE: clone is not cloning underlying bytes, but just increases
                                // ref count.
                                .clone()
                                .reader(),
                        );
                    }
                    StreamBuffer::Seekable(ref mut cursor) => {
                        cursor
                            .write_all(packet.as_ref())
                            // TODO: this should not panic. probably it's fine. there are very few
                            // things that could go wrong with Vec<u8> in rust.
                            .expect("could not buffer");
                        // invalidate last tick so that it can be re-scanned if needed.
                        self.total_ticks = None;
                    }
                }

                Some(Ok(packet))
            }

            Err(err) => {
                self.stream_state = StreamState::Stop;
                match err {
                    // tried hard, coudn't fetch. match ended (or maybe became unavailable)
                    BroadcastHttpClientError::StatusCode(http::StatusCode::NOT_FOUND) => None,
                    err => Some(Err(err)),
                }
            }
        }
    }
}

// ----
// demo stream

// NOTE: following panics constitute a developer error.

#[cold]
#[inline(never)]
fn no_packet_panic() -> ! {
    panic!("attempted reading on BroadcastHttp with no packets")
}

#[cold]
#[inline(never)]
fn not_seekable_panic() -> ! {
    panic!("attempted invoking seek-related operation on BroadcastHttp constructed not with `start_streaming_and_buffer`")
}

impl<'client, C: HttpClient + 'client> DemoStream for BroadcastHttp<'client, C> {
    // stream ops
    // ----

    /// panics if [`BroadcastHttp`] was not constructed with `start_reading_and_buffer`. othwerwise
    /// delegated to [`std::io::Cursor`].
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        match self.stream_buffer {
            StreamBuffer::Last(_) => not_seekable_panic(),
            StreamBuffer::Seekable(ref mut c) => c.seek(pos),
        }
    }

    /// panics if [`BroadcastHttp`] was not constructed with `start_reading_and_buffer`.
    fn stream_position(&mut self) -> Result<u64, io::Error> {
        match self.stream_buffer {
            StreamBuffer::Last(_) => not_seekable_panic(),
            StreamBuffer::Seekable(ref mut c) => Ok(c.position()),
        }
    }

    /// panics if [`BroadcastHttp`] was not constructed with `start_reading_and_buffer`.
    fn stream_len(&mut self) -> Result<u64, io::Error> {
        match self.stream_buffer {
            StreamBuffer::Last(_) => not_seekable_panic(),
            StreamBuffer::Seekable(ref c) => Ok(c.get_ref().len() as u64),
        }
    }

    /// panics if `next_packet` never succeded.
    fn is_at_eof(&mut self) -> Result<bool, io::Error> {
        match self.stream_buffer {
            StreamBuffer::Last(None) => no_packet_panic(),
            StreamBuffer::Last(Some(ref r)) => Ok(!r.get_ref().has_remaining()),
            StreamBuffer::Seekable(ref c) => Ok(c.position() as usize >= c.get_ref().len()),
        }
    }

    // cmd header
    // ----

    /// panics if `next_packet` never succeded.
    fn read_cmd_header(&mut self) -> Result<CmdHeader, ReadCmdHeaderError> {
        match self.stream_buffer {
            StreamBuffer::Last(None) => no_packet_panic(),
            StreamBuffer::Last(Some(ref mut r)) => read_cmd_header(r),
            StreamBuffer::Seekable(ref mut c) => read_cmd_header(c),
        }
    }

    // cmd
    // ----

    /// panics if `next_packet` never succeded.
    fn read_cmd(&mut self, cmd_header: &CmdHeader) -> Result<&[u8], ReadCmdError> {
        match self.stream_buffer {
            StreamBuffer::Last(None) => no_packet_panic(),

            StreamBuffer::Last(Some(ref mut r)) => {
                use bytes::Buf;

                let size = cmd_header.body_size as usize;
                let bytes = r.get_mut();

                // it probably could be possible that body of the response was not transferred /
                // read correctly?
                if bytes.remaining() < size {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
                }

                // SAFETY: this is safe because lifetime of the returned slice is tied to the
                // lifetime of r (if i'm not missing anything, am i?).
                let data = unsafe {
                    // NOTE: start is 0 because Reader's advance will increase start position of
                    // the underlying slice
                    let ptr = bytes.as_ref()[0..size].as_ptr();
                    std::slice::from_raw_parts(ptr, size)
                };
                bytes.advance(size);
                Ok(data)
            }

            StreamBuffer::Seekable(ref mut c) => {
                let size = cmd_header.body_size as usize;
                let pos = c.position() as usize;

                // it probably could be possible that body of the response was not transferred /
                // read correctly?
                let remaining = c.get_ref().len() - pos;
                if remaining < size {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
                }

                // NOTE: Cursor's advance will not discard data from the underlying Vec<u8>
                c.consume(size);
                Ok(&c.get_ref()[pos..pos + size])
            }
        }
    }

    #[inline(always)]
    fn decode_cmd_send_tables(data: &[u8]) -> Result<CDemoSendTables, DecodeCmdError> {
        decode_cmd_send_tables(data)
    }

    #[inline(always)]
    fn decode_cmd_class_info(data: &[u8]) -> Result<CDemoClassInfo, DecodeCmdError> {
        decode_cmd_class_info(data)
    }

    #[inline(always)]
    fn decode_cmd_packet(data: &[u8]) -> Result<CDemoPacket, DecodeCmdError> {
        decode_cmd_packet(data)
    }

    #[inline(always)]
    fn decode_cmd_full_packet(data: &[u8]) -> Result<CDemoFullPacket, DecodeCmdError> {
        decode_cmd_full_packet(data)
    }

    // other
    // ----

    /// panics if [`BroadcastHttp`] was not constructed with `start_reading_and_buffer`.
    fn start_position(&self) -> u64 {
        match self.stream_buffer {
            StreamBuffer::Last(_) => not_seekable_panic(),
            StreamBuffer::Seekable(_) => 0,
        }
    }

    /// panics if [`BroadcastHttp`] was not constructed with `start_reading_and_buffer`.
    fn total_ticks(&mut self) -> Result<i32, anyhow::Error> {
        match self.stream_buffer {
            StreamBuffer::Last(_) => not_seekable_panic(),
            StreamBuffer::Seekable(_) => {
                if self.total_ticks.is_none() {
                    self.total_ticks = Some(scan_for_last_tick(self)?);
                }
                Ok(self.total_ticks.unwrap())
            }
        }
    }
}
