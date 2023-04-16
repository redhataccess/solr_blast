pub struct SolrClient {
    pub url: String,
    pub client: reqwest::Client,
}

impl SolrClient {
    pub fn new(url: &str) -> SolrClient {
        SolrClient {
            url: url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn ping(&self) -> Result<(), reqwest::Error> {
        let url = format!("{}/admin/ping", self.url);
        let res = self.client.get(&url).send().await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
