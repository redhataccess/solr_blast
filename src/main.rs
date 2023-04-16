use solr_blast::SolrClient;

#[tokio::main]
async fn main() {
    println!("solr blast!");

    let client = SolrClient::new("http://localhost:8983/solr");

    let res = client.ping().await;
    match res {
        Ok(_) => println!("ping ok"),
        Err(e) => println!("ping error: {}", e),
    }
}
