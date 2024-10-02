use std::marker::PhantomData;

use anyhow::anyhow;
use bytes::Bytes;
use serde::Deserialize;

use crate::HttpClient;

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
    #[error("http client error: {0}")]
    HttpClientError(HttpClientError),
    #[error("could not build request: {0}")]
    BuildRequestError(http::Error),
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

    async fn get(
        &self,
        uri: String,
    ) -> Result<http::Response<Bytes>, HttpBroadcastClientError<C::Error>> {
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

        self.http_client
            .execute(request)
            .await
            .map_err(HttpBroadcastClientError::HttpClientError)
    }

    // `SendGet( request, new CSyncRequest( m_SyncParams, nResync ) )`
    // call within the
    // `void CDemoStreamHttp::SendSync( int nResync )`
    pub async fn get_sync(&self) -> Result<SyncResponse, HttpBroadcastClientError<C::Error>> {
        let mut uri = self.base_uri.to_string();
        uri.push_str("/sync");

        let response = self.get(uri).await?;
        let body = response.into_body();

        serde_json::from_slice(&body).map_err(HttpBroadcastClientError::DeserializeJsonError)
    }

    // `SendGet( CFmtStr( "/%d/start", m_SyncResponse.nSignupFragment ), new CStartRequest( ) )`
    // call within the
    // `bool CDemoStreamHttp::OnSync( int nResync )`.
    pub async fn get_start(
        &self,
        signup_fragment: i32,
    ) -> Result<Bytes, HttpBroadcastClientError<C::Error>> {
        assert!(signup_fragment >= 0);

        let mut uri = self.base_uri.to_string();
        uri.push_str(&format!("/{signup_fragment}/start"));

        let response = self.get(uri).await?;
        Ok(response.into_body())
    }

    // void CDemoStreamHttp::RequestFragment( int nFragment, FragmentTypeEnum_t nType )
    pub async fn get_fragment(
        &self,
        fragment: i32,
        typ: FragmentType,
    ) -> Result<Bytes, HttpBroadcastClientError<C::Error>> {
        let mut uri = self.base_uri.to_string();
        uri.push_str(&format!("/{fragment}/{}", typ.as_path_part()));

        let response = self.get(uri).await?;
        Ok(response.into_body())
    }
}

// http broadcast
// ----

// useful links to dig into:
// - https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Broadcast
// - https://github.com/saul/demofile-net/pull/93
// - https://github.com/FlowingSPDG/gotv-plus-go

// example requests (ordered):
// - http://dist1-ord1.steamcontent.com/tv/18895867/sync
// - http://dist1-ord1.steamcontent.com/tv/18895867/0/start
// - http://dist1-ord1.steamcontent.com/tv/18895867/295/full
// - http://dist1-ord1.steamcontent.com/tv/18895867/296/delta

// how csgo gets to packet:
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

// enum StreamStateEnum_t
enum StreamState {
    Fullframe,
    Deltaframes,
}

enum Fragment {
    Delta(Bytes),
    Full(Bytes),
}

pub struct HttpBroadcast<'client, C: HttpClient + 'client> {
    client: HttpBroadcastClient<'client, C>,
    stream_fragment: i32,
    stream_signup: Bytes,
    sync_response: SyncResponse,
    stream_state: StreamState,
    fragment_cache: Vec<Fragment>,
}

// TODO: how to handle non-2xx status codes?

impl<'client, C: HttpClient + 'client> HttpBroadcast<'client, C> {
    pub async fn start_streaming(
        http_client: C,
        base_uri: impl Into<String>,
        app_id: AppId,
    ) -> Result<Self, anyhow::Error> {
        let client = HttpBroadcastClient::new(http_client, base_uri, app_id);

        // TODO: do we need valve-like state?

        // CDemoStreamHttp.m_nState = STATE_SYNC;
        // CBroadcastPlayer.m_nStreamState = STREAM_SYNC;
        let sync_response = client
            .get_sync()
            .await
            .map_err(|err| anyhow!("could not get sync: {err}"))?;

        // CDemoStreamHttp.m_nState = STATE_START;
        // CBroadcastPlayer.m_nStreamState = nResync ? STREAM_FULLFRAME : STREAM_START;
        let stream_signup = client
            .get_start(sync_response.signup_fragment.unwrap_or(0))
            .await
            .map_err(|err| anyhow!("could not get sync: {err}"))?;

        // now call prepare_packet

        Ok(Self {
            client,
            stream_fragment: sync_response.fragment.unwrap_or(0),
            stream_signup,
            sync_response,
            stream_state: StreamState::Fullframe,
            fragment_cache: vec![],
        })
    }

    // bool CBroadcastPlayer::PreparePacket( void )
    pub async fn prepare_packet(&mut self) -> Result<(), anyhow::Error> {
        // TODO: if not at eof - return
        match self.stream_state {
            StreamState::Fullframe => {
                let full = self
                    .client
                    .get_fragment(self.stream_fragment, FragmentType::Full)
                    .await?;
                self.fragment_cache.push(Fragment::Full(full));
                self.stream_state = StreamState::Deltaframes;
            }
            StreamState::Deltaframes => {
                let delta = self
                    .client
                    .get_fragment(self.stream_fragment, FragmentType::Delta)
                    .await?;
                self.fragment_cache.push(Fragment::Delta(delta));
                // TODO : count the ticks, not fragments (?)
                self.stream_fragment += 1;
            }
        }
        Ok(())
    }
}
