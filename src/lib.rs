use std::{
    collections::HashSet,
    fs::File,
    io::Read,
    path::Path,
    sync::{Arc, RwLock},
    time::Duration,
};

use futures::StreamExt;
use pbr::ProgressBar;
use rayon::prelude::*;
use regex::Regex;
use reqwest::Client;
use wax::{Glob, WalkEntry, WalkError};

pub struct SolrClient {
    pub url: String,
    pub client: reqwest::Client,
    pub conncurency: u32,
}

impl SolrClient {
    pub fn new(url: &str) -> SolrClient {
        SolrClient {
            url: url.to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .expect("Failed to build reqwest client"),
            conncurency: 8,
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

    pub async fn post_from_glob(&self, glob: &str, ci_mode: bool) {
        // Glob .html files
        let glob = Glob::new(glob).unwrap();
        let files: Vec<Result<WalkEntry, WalkError>> = glob.walk(".").collect();
        let files_to_index_set: HashSet<String>;

        // scope for the MutexGuard accross async/await
        // see: https://rust-lang.github.io/rust-clippy/master/index.html#await_holding_lock
        {
            // files to index
            let files_to_index = Arc::new(RwLock::new(HashSet::<String>::new()));

            // this clone is just so the main thread can hold onto a reference, to then print out later
            let files_to_index_ref = files_to_index.clone();

            // Scan for .html files that need indexing and store them in a vector
            files.par_iter().for_each(|file| match file {
                Ok(entry) => {
                    let path = entry.path();
                    let path_str = path.to_str().unwrap();

                    // read the file content
                    let mut file = File::open(path_str).unwrap();
                    let mut contents = String::new();
                    file.read_to_string(&mut contents).unwrap();

                    // use regex to find any noindex meta tags
                    let noindex_re = Regex::new(r#"meta name="robots" content="noindex""#).unwrap();

                    if !noindex_re.is_match(&contents) {
                        let mut files_to_index_set =
                            files_to_index.write().expect("rwlock poisoned");
                        files_to_index_set.insert(path_str.to_string());
                    }
                }
                Err(e) => println!("error: {:?}", e),
            });

            let rw_lock_files_set = files_to_index_ref.read().expect("rwlock poisoned");
            files_to_index_set = rw_lock_files_set.clone();
        } // MutexGuard is dropped here

        let total_files_to_index = files_to_index_set.len();

        let mut posts = futures::stream::iter(files_to_index_set.into_iter().map(|file| async {
            // get the absolute path of file
            let file_path = Path::new(&file);
            let file_path_absolute = file_path.canonicalize().unwrap();

            // url encode the file path string
            let file_path_encoded = urlencoding::encode(file_path_absolute.to_str().unwrap());

            // read the file into a String
            let mut file = File::open(file).unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();

            // format the solr post url using file_path_encoded as the resource.name & literal.id
            let solr_post_url = format!(
                "http://localhost:8983/solr/portal/update/extract?resource.name={0}&literal.id={0}",
                file_path_encoded
            );

            // use reqwest::Client to post the file to solr using the Apache Tika update/extract handler
            self.client
                .post(solr_post_url)
                .header(reqwest::header::CONTENT_TYPE, "text/html")
                .body(contents)
                .send()
                .await
        }))
        .buffer_unordered(self.conncurency as usize);

        // Start progress bar
        let mut pb = ProgressBar::new(total_files_to_index.try_into().unwrap());
        pb.message("posting to solr");
        if ci_mode {
            // when running in a CI environment only output every 1 sec to cut down on log size
            pb.set_max_refresh_rate(Some(Duration::from_millis(1000)));
        }

        // loop through the stream of futures solr POST requests and increment the progress bar
        while let Some(res) = posts.next().await {
            match res {
                Ok(_) => {
                    pb.inc();
                }
                Err(e) => {
                    eprintln!("{}", e)
                }
            }
        }

        // send GET request to solr to commit the changes
        self.client
            .get("http://localhost:8983/solr/portal/update?commit=true")
            .send()
            .await
            .unwrap();
    }
}
