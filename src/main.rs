use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;

fn create_request_client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .user_agent("alex-observer/0.1.0")
        .build()
        .expect("should create a request client")
}

async fn request(client: &Client) -> Option<String> {
    let req = client.get("https://en.wikipedia.org/wiki/Brussels").build();
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

fn parse_document(html: &String) {
    let document = Html::parse_document(html);
    // let selector = Selector::parse("h1, h2, h3, h4, h5, h6, p").unwrap();
    let href_selector = Selector::parse("a").unwrap();
    for element in document.select(&href_selector) {
        // println!("{}", element.text().collect::<String>()); // text representation of the page
        let href = element.value().attr("href");
        if href.is_none() {
            continue;
        };
        let href = href.unwrap();

        // skip fragments
        if href.starts_with("#") {
            continue;
        }
        println!("{}", href)
    }
}

#[tokio::main]
async fn main() {
    let client = create_request_client();
    // let there be parser for robots.txt
    let brussels = request(&client).await.unwrap();
    parse_document(&brussels);
}
