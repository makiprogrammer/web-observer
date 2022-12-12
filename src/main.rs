use reqwest::{Client, Url};
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::time::Duration;

fn create_request_client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .user_agent("alex-observer/0.1.0")
        .build()
        .expect("should create a request client")
}

async fn request(client: &Client, url: &Url) -> Option<String> {
    let req = client.get(url.to_string()).build();
    if req.is_err() {
        return None;
    }
    let req = req.unwrap();
    let future_response = client.execute(req);
    let text = future_response
        .await
        .expect("request should have been performed successfully")
        .text()
        .await
        .expect("response should have been parsed successfully");

    Some(text)
}

// fn html_document_to_text(html: &String) -> String {
//     let document = Html::parse_document(html);
//     let selector = Selector::parse("h1, h2, h3, h4, h5, h6, p").unwrap();
//     let mut text = String::new();
//     for element in document.select(&selector) {
//         text.push_str(&element.text().collect::<Vec<&str>>().join("\n"));
//     }
//     text
// }

fn find_links(html: &String, url: &Url) -> Vec<String> {
    let document = Html::parse_document(html);
    let href_selector = Selector::parse("a").unwrap();

    let domain = url.domain().unwrap();

    document
        .select(&href_selector)
        .filter_map(|element| {
            let href = element.value().attr("href");
            if href.is_none() {
                return None;
            };
            let href = href.unwrap();

            // skip fragments
            if href.starts_with("#") {
                return None;
            }
            if href.starts_with("//") {
                return Some(format!("https:{}", href));
            }
            if href.starts_with("/") {
                return Some(format!("https://{}{}", domain, href));
            }
            if !href.starts_with("https://") {
                return None;
            }
            Some(href.to_string())
        })
        .collect()
}

#[tokio::main]
async fn main() {
    let client = create_request_client();

    let mut queue = VecDeque::new();
    queue.push_back(String::from("https://en.wikipedia.org/wiki/Brussels"));

    let mut visited: HashSet<String> = HashSet::new();
    let mut counter: u32 = 0;

    while 0 < queue.len() && queue.len() < 9999 {
        // get the oldest value from the queue
        let url = queue.pop_front().unwrap();
        let url = Url::parse(&url).unwrap();

        // print some status
        counter += 1;
        println!("Page #{} / {}: {}", counter, queue.len(), url);

        // request the document
        let html = request(&client, &url).await.unwrap();
        // parse the document and iterate over the links
        for link in find_links(&html, &url) {
            if visited.insert(sha256::digest(link.clone())) {
                queue.push_back(link);
            }
        }
    }
}
