use std::error::Error;
use std::future::Future;

use bytes::Bytes;

// http client
// ----

// NOTE: http::Response's body (Bytes) is wrapped into Result because aparantely there are
// situations when server speifies the gzip encoding within the content-encoding header but does
// not actually encode the body (curl with --trace flag is helpful). very xd, thank you valve.

// TODO: might want to box futures, see:
// https://docs.rs/futures/latest/futures/future/type.BoxFuture.html
//
// pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait HttpClient {
    type Error: Error + Send + Sync + 'static;

    fn execute(
        &self,
        request: http::Request<Bytes>,
    ) -> impl Future<Output = Result<http::Response<Result<Bytes, Self::Error>>, Self::Error>>;
}

// reqwest impl
// ----

#[cfg(feature = "reqwest")]
mod reqwest_impl {
    use bytes::Bytes;

    use super::HttpClient;

    impl HttpClient for reqwest::Client {
        type Error = reqwest::Error;

        async fn execute(
            &self,
            request: http::Request<Bytes>,
        ) -> Result<http::Response<Result<Bytes, Self::Error>>, Self::Error> {
            let (parts, body) = request.into_parts();
            let mut request = self
                .request(parts.method, parts.uri.to_string())
                .body(body)
                .headers(parts.headers);
            #[cfg(not(target_arch = "wasm32"))]
            {
                request = request.version(parts.version);
            }

            // reqwest's Response is so fucking obnoxiously gate keeping
            let mut response = request.send().await?;

            let mut result = http::Response::builder().status(response.status());
            #[cfg(not(target_arch = "wasm32"))]
            {
                result = result.version(response.version());
            }
            // NOTE: expects should never be called - otherwise this is either http or reqwest
            // library error.
            std::mem::swap(
                response.headers_mut(),
                result.headers_mut().expect("could not get result headers"),
            );
            let result = result
                .body(response.bytes().await)
                .expect("could not transpose body");
            Ok(result)
        }
    }
}
