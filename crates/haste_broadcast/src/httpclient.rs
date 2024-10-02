use std::future::Future;

use bytes::Bytes;

// http client
// ----

pub trait HttpClient {
    type Error: std::error::Error + Send + Sync + 'static;

    fn execute(
        &self,
        request: http::Request<Bytes>,
    ) -> impl Future<Output = Result<http::Response<Bytes>, Self::Error>>;
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
        ) -> Result<http::Response<Bytes>, Self::Error> {
            let (parts, body) = request.into_parts();
            let http::request::Parts {
                method,
                uri,
                version,
                headers,
                ..
            } = parts;
            let request = self
                .request(method, uri.to_string())
                .version(version)
                .headers(headers)
                .body(body);

            // reqwest's Response is so fucking obnoxiously gate keeping
            let mut response = request.send().await?;

            // NOTE: expects should never be called - otherwise this is either http or reqwest
            // library error.

            let mut result = http::Response::builder()
                .status(response.status())
                .version(response.version());
            std::mem::swap(
                response.headers_mut(),
                result.headers_mut().expect("could not get result headers"),
            );
            let result = result
                .body(response.bytes().await?)
                .expect("could not transpose body");
            Ok(result)
        }
    }
}
