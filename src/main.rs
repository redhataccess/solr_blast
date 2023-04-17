use clap::Parser;
use solr_blast::SolrClient;

/// Simple program to post files to Solr
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// collection name defaults to DEFAULT_SOLR_COLLECTION if not specified
    #[arg(short, default_value_t = std::env::var("DEFAULT_SOLR_COLLECTION").unwrap_or(String::from("solr")))]
    collection: String,

    /// filetypes to include in the post
    #[arg(short, long, default_value_t = String::from("xml,json,csv,pdf,doc,docx,ppt,pptx,xls,xlsx,odt,odp,ods,ott,otp,ots,rtf,htm,html,txt,log"))]
    filetypes: String,

    /// files|globs|directorys to post to solr
    // #[derive(Copy, Clone)]
    source: Vec<String>,

    /// base Solr update URL (overrides collection, host, and port)
    #[arg(long)]
    url: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let mut url = "http://localhost:8983/solr/portal";

    // get url arg
    if let Some(u) = args.url.as_deref() {
        url = u;
    }

    let client = SolrClient::new(url);

    let glob = args.source.get(0).unwrap().as_str();

    client.post_from_glob(glob, false).await;

    println!("done")
}
