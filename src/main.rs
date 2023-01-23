use reqwest::{Client, Url};
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::time::Duration;
use texting_robots::{get_robots_url, Robot};

/// Creates and returns a web Client instance from Reqwest crate.
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

fn find_links(html: &String, url: &Url) -> Vec<Url> {
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
        .map(|url_string| {
            let mut url = Url::parse(&url_string).expect("should create valid URL");
            url.set_fragment(None);
            url.set_query(None);
            url
        })
        .collect()
}

#[tokio::main]
async fn main() {
    // TODO: load startup urls from a file
    // TODO: loading error handling
    // TODO: parallelization
    // TODO: 4xx Too Many Requests

    // basic settings
    // let REQUEST_DELAY_SECONDS = 1;
    const MAXIMUM_WEBSITES: u32 = 20;
    let mut counter: u32 = 0;

    let mut init_queue: Vec<String> = Vec::new();
    init_queue.push(String::from("https://en.wikipedia.org/wiki/Brussels"));
    init_queue.push(String::from("https://www.nytimes.com/"));

    let mut visited: HashSet<String> = HashSet::new();
    let mut yet_to_visit = Vec::with_capacity(1000);

    let client = create_request_client();

    // request startup pages and add new links to the yet-to-visit list
    for (i, startup_url) in init_queue.iter().enumerate() {
        println!("Requesting startup page #{}: {}", i + 1, startup_url);
        let mut url = Url::parse(&startup_url).unwrap();
        url.set_fragment(None);

        visited.insert(url.as_str().to_string());

        let html = request(&client, &url).await.unwrap();
        // parse the HTML document and add links to the yet-to-visit list
        for another_url in find_links(&html, &url) {
            if !visited.contains(&another_url.as_str().to_string()) {
                yet_to_visit.push(another_url);
            }
        }
    }

    // now, the main and long loop
    while yet_to_visit.len() > 0 && counter < MAXIMUM_WEBSITES {
        // pick one url from the yet-to-visit list
        let domain = yet_to_visit.first().unwrap().domain().unwrap().to_owned();

        let mut urls_with_same_domain: Vec<Url> = Vec::new();

        // filter out all URLs with the same domain - remove them from the yet-to-visit list
        let mut i = 0;
        while i < yet_to_visit.len() {
            if yet_to_visit[i].domain().unwrap() == domain {
                urls_with_same_domain.push(yet_to_visit.swap_remove(i));
            } else {
                i += 1;
            }
        }

        // fetch and parse robots.txt
        let robots_url = get_robots_url(format!("https://{}", domain).as_str());
        if robots_url.is_err() {
            // ParseError occurred
			println!("Error finding robots.txt for domain {}", domain);
            continue;
        }
        let robots_url = Url::parse(robots_url.unwrap().as_str()).unwrap();
        let robots_txt = request(&client, &robots_url).await.unwrap();
        let robot = Robot::new("alex-observer/0.1.0", robots_txt.as_bytes());
        if robot.is_err() {
            // error parsing the robots.txt
			println!("Error parsing robots.txt for domain {}", domain);
            continue;
        }
        let robot = robot.unwrap();

        // fetch all the urls with the same domain
        let mut same_domain_counter = 0;
        while urls_with_same_domain.len() > 0 {
            let url = urls_with_same_domain.pop().unwrap();
			// check if the URL is allowed to crawl in robots.txt
            if !robot.allowed(url.path()) {
                continue;
            }

            // request the document
            let html = request(&client, &url).await.unwrap();
            visited.insert(url.as_str().to_string());
            counter += 1;
            same_domain_counter += 1;
            println!(
                "Requesting {} ({} on domain {}) {}",
                counter,
                same_domain_counter,
                domain,
                url.path()
            );

            // parse the document and iterate over the links
            for another_url in find_links(&html, &url) {
                if visited.contains(&another_url.to_string()) {
                    continue;
                }
                if another_url.domain().unwrap() == domain {
                    urls_with_same_domain.push(another_url);
                } else {
                    yet_to_visit.push(another_url);
                }
            }

            // TODO: wait some time
        }
    }
}
