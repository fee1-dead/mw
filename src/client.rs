use std::borrow::Cow;
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize, Serializer};

use crate::Result;

pub struct ClientBuilder {
    url: String,
    ua: Option<Cow<'static, str>>,
    configure_client: Option<Box<dyn FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder>>,
}

impl ClientBuilder {
    pub fn new(url: impl Into<String>) -> ClientBuilder {
        ClientBuilder {
            url: url.into(),
            ua: None,
            configure_client: None,
        }
    }
    pub fn user_agent(mut self, ua: impl Into<Cow<'static, str>>) -> Self {
        self.ua = Some(ua.into());
        self
    }
    pub fn configure_client(
        mut self,
        configure_client: Box<dyn FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder>,
    ) -> Self {
        self.configure_client = Some(configure_client);
        self
    }
    fn make_client(
        ua: Option<Cow<'static, str>>,
        configure_client: Option<Box<dyn FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder>>,
        extra_headers: impl FnOnce(HeaderMap) -> crate::Result<HeaderMap>,
        cookies: bool,
    ) -> crate::Result<reqwest::Client> {
        let ua = ua.as_deref().unwrap_or(crate::UA);
        let mut client = reqwest::Client::builder();
        #[cfg(not(target_arch = "wasm32"))]
        {
            client = client.cookie_store(cookies).user_agent(ua);
        }
        let headers = HeaderMap::new();

        /* TODO #[cfg(target_arch = "wasm32")]
        {
            headers.insert("Api-User-Agent", HeaderValue::from_str(ua)?);
        } */

        let headers = extra_headers(headers)?;
        client = client.default_headers(headers);

        if let Some(configure) = configure_client {
            client = configure(client);
        }

        Ok(client.build()?)
    }
    pub fn anonymous(self) -> crate::Result<Client> {
        let inner = Self::make_client(self.ua, self.configure_client, Ok, false)?;
        Ok(Client {
            inner,
            url: self.url,
        })
    }
    /* pub fn login_botpassword(self, username: String, password: String) {

    }*/
    pub async fn login_oauth(self, token: &str) -> crate::Result<(Client, String)> {
        let inner = Self::make_client(
            self.ua,
            self.configure_client,
            |mut headers| {
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {token}"))?,
                );
                Ok(headers)
            },
            false,
        )?;

        let client = Client {
            inner,
            url: self.url,
        };

        let name = client.verify_logged_in().await?;

        Ok((client, name))
    }
}

#[derive(Clone, Copy)]
pub struct ClientRef<'a> {
    client: &'a reqwest::Client,
    url: &'a str,
}

impl ClientRef<'_> {
    pub fn get(&self, params: impl Params) -> reqwest::RequestBuilder {
        self.client.get(self.url).query(&Standard(params))
    }

    pub fn post(&self, params: impl Params) -> reqwest::RequestBuilder {
        self.client.post(self.url).form(&Standard(params))
    }

    /// Returns the user name this bot is logged in as.
    pub async fn verify_logged_in(&self) -> crate::Result<String> {
        #[derive(Deserialize)]
        struct Response {
            query: Query,
        }
        #[derive(Deserialize)]
        struct Query {
            userinfo: UserInfo,
        }
        #[derive(Deserialize)]
        struct UserInfo {
            // id: usize,
            name: String,
        }

        let r: Response = self
            .get([("action", "query"), ("meta", "userinfo"), ("uiprop", "")])
            .send()
            .await?
            .json()
            .await?;
        let name = r.query.userinfo.name;
        if Ipv4Addr::from_str(&name).is_ok() || Ipv6Addr::from_str(&name).is_ok() {
            return Err(crate::Error::Unauthorized);
        }
        Ok(name)
    }

    pub async fn get_token(&self, token: impl AsRef<str>) -> reqwest::Result<String> {
        Ok(self.get_tokens([token]).await?.pop().unwrap())
    }

    pub async fn get_tokens(
        &self,
        tokens: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> reqwest::Result<Vec<String>> {
        let mut t = String::new();
        #[derive(Deserialize)]
        pub struct Query {
            tokens: HashMap<String, String>,
        }
        #[derive(Deserialize)]
        pub struct Response {
            query: Query,
        }
        for s in tokens {
            if !t.is_empty() {
                t.push('|');
            }
            t.push_str(s.as_ref());
        }

        let mut res = self
            .get([("action", "query"), ("meta", "tokens"), ("type", &t)])
            .send()
            .await?
            .json::<Response>()
            .await?;

        Ok(t.split('|')
            .map(|x| res.query.tokens.remove(&format!("{x}token")).unwrap())
            .collect())
    }
}

#[derive(Clone)]
pub struct Client {
    inner: reqwest::Client,
    url: String,
}

pub trait Params {
    fn len(&self) -> usize;
    fn serialize_into<S: SerializeSeq>(&self, seq: &mut S) -> Result<(), S::Error>;
}

impl<T: Serialize> Params for [T] {
    fn len(&self) -> usize {
        <[_]>::len(self)
    }
    fn serialize_into<S: SerializeSeq>(&self, seq: &mut S) -> Result<(), S::Error> {
        for x in self {
            seq.serialize_element(x)?;
        }
        Ok(())
    }
}

impl<T: Serialize, const N: usize> Params for [T; N] {
    fn len(&self) -> usize {
        N
    }
    fn serialize_into<S: SerializeSeq>(&self, seq: &mut S) -> Result<(), S::Error> {
        for x in self {
            seq.serialize_element(x)?;
        }
        Ok(())
    }
}

impl<P: ?Sized + Params> Params for &'_ P {
    fn len(&self) -> usize {
        P::len(self)
    }
    fn serialize_into<S: SerializeSeq>(&self, seq: &mut S) -> Result<(), S::Error> {
        P::serialize_into(self, seq)
    }
}

struct Standard<T: Params>(T);

impl<T: Params> Serialize for Standard<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(2 + self.0.len()))?;
        seq.serialize_element(&("format", "json"))?;
        seq.serialize_element(&("formatversion", "2"))?;
        self.0.serialize_into(&mut seq)?;
        seq.end()
    }
}

impl Client {
    pub fn inner(&self) -> &reqwest::Client {
        &self.inner
    }
    pub fn with_url<'a>(&'a self, url: &'a str) -> ClientRef<'a> {
        ClientRef {
            client: &self.inner,
            url,
        }
    }
    pub fn get(&self, params: impl Params) -> reqwest::RequestBuilder {
        self.with_url(&self.url).get(params)
    }

    pub fn post(&self, params: impl Params) -> reqwest::RequestBuilder {
        self.with_url(&self.url).post(params)
    }

    pub async fn verify_logged_in(&self) -> crate::Result<String> {
        self.with_url(&self.url).verify_logged_in().await
    }

    pub async fn get_token(&self, token: impl AsRef<str>) -> reqwest::Result<String> {
        self.with_url(&self.url).get_token(token).await
    }

    pub async fn get_tokens(
        &self,
        tokens: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> reqwest::Result<Vec<String>> {
        self.with_url(&self.url).get_tokens(tokens).await
    }
}
