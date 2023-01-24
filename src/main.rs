use reqwest::{Client, Url};
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::time::Duration;
use texting_robots::{get_robots_url, Robot};

const MAXIMUM_CRAWLED_WEBSITES: usize = 99999;

enum CrawlerError {
    // ReqwestError(),
    // UrlParseError(),
    RobotsTxtError(String),
    RobotsTxtParseError(String),
}

/// Main crawling structure capable of crawling the web.
struct Crawler {
    /// The `Reqwest` client instance used to make HTTP requests.
    client: Client,
    /// Set of all already-visited URLs stored as strings.
    visited: HashSet<String>,
    /// List of all URLs that are yet to be visited.
    yet_to_visit: Vec<Url>,
    /// Number of websites that have been crawled (not domains).
    counter: usize,
}

impl Crawler {
    /// Creates a new `Crawler` instance.
    fn new() -> Crawler {
        Crawler {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .user_agent("alex-observer/0.1.0")
                .build()
                .expect("should create a request client"),
            visited: HashSet::new(),
            yet_to_visit: Vec::with_capacity(100),
            counter: 0,
        }
    }

	/// Performs a HTTP request using crawler's client. Returns response text.
    async fn request_website(&self, url: &Url) -> Option<String> {
        // create a request object and then execute it
        let req = self.client.get(url.to_string()).build();
        if req.is_err() {
            return None;
        }
        let req = req.unwrap();
    	let response = self.client.execute(req).await;
		if response.is_err() {
            return None;
        }

        let text = response.unwrap().text().await;
        if text.is_err() {
            return None;
        }
        Some(text.unwrap())
    }

	/// Initializes the crawler with startup urls. Newly-found urls will
	/// be added to yet-to-visit list.
    async fn init_crawl(&mut self, init_queue: &Vec<String>) -> () {
        for (i, startup_url) in init_queue.iter().enumerate() {
            println!("Requesting startup page #{}: {}", i + 1, startup_url);
            let mut url = Url::parse(&startup_url).unwrap();
            url.set_fragment(None);

            let html = self.request_website(&url).await;
            if html.is_none() {
				continue;
            }
			let html = html.unwrap();
			self.visited.insert(url.as_str().to_string());
			
            // parse the HTML document and add links to the yet-to-visit list
            for another_url in find_links(&html, &url) {
                if !self.visited.contains(&another_url.as_str().to_string()) {
                    self.yet_to_visit.push(another_url);
                }
            }
        }
    }

	/// Selects first url in the yet-to-visit url list and crawles all urls
	/// with the same domain. New urls are crawled if they have the same domain,
	/// otherwise, they are added to yet-to-visit list.
    async fn one_domain_crawl(&mut self) -> Result<usize, CrawlerError> {
        if self.yet_to_visit.len() == 0 {
            return Ok(0);
        }

        // pick one domain from the yet-to-visit list
        let domain = self
            .yet_to_visit
            .first()
            .unwrap()
            .domain()
            .unwrap()
            .to_owned();

        let mut urls_with_same_domain: Vec<Url> = Vec::new();
        // filter out all URLs with the same domain - remove them from the yet-to-visit list
        let mut i = 0;
        while i < self.yet_to_visit.len() {
            if self.yet_to_visit[i].domain().unwrap() == domain {
                urls_with_same_domain.push(self.yet_to_visit.swap_remove(i));
            } else {
                i += 1;
            }
        }

        // fetch and parse robots.txt
        let robots_url = get_robots_url(format!("https://{}", domain).as_str());
        if robots_url.is_err() {
            // error finding robots.txt for this domain
            // TODO: maybe continue without constrains?
            return Err(CrawlerError::RobotsTxtError(domain));
        }
        let robots_url = Url::parse(robots_url.unwrap().as_str()).unwrap();
        let robots_txt = self.request_website(&robots_url).await;
        if robots_txt.is_none() {
            return Err(CrawlerError::RobotsTxtError(domain));
        }
        let robots_txt = robots_txt.unwrap();
        let robot = Robot::new("alex-observer/0.1.0", robots_txt.as_bytes());
        if robot.is_err() {
            // error parsing the robots.txt
            return Err(CrawlerError::RobotsTxtParseError(domain));
        }
        let robot = robot.unwrap();

        // fetch all the urls with the same domain
        let mut same_domain_counter: usize = 0;
        while urls_with_same_domain.len() > 0 && self.counter < MAXIMUM_CRAWLED_WEBSITES {
            let url = urls_with_same_domain.pop().unwrap();

            // important: check if the URL is allowed by robots.txt
            if !robot.allowed(url.path()) {
                continue;
            }

            // request the document
            print!(
                "Requesting {} ({} on domain {}) {} ",
                self.counter,
                same_domain_counter,
                domain,
                url.path()
            );
            let html = self.request_website(&url).await;
            if html.is_none() {
                println!("Failed!");
                continue;
            }
            let html = html.unwrap();
            println!("Done!");

            self.visited.insert(url.as_str().to_string());
            self.counter += 1;
            same_domain_counter += 1;

            // parse the document and iterate over the links
            for another_url in find_links(&html, &url) {
                if self.visited.contains(&another_url.to_string()) {
                    continue;
                }
                if another_url.domain().unwrap() == domain {
                    urls_with_same_domain.push(another_url);
                } else {
                    self.yet_to_visit.push(another_url);
                }
            }

            // TODO: wait some time
        }

        Ok(same_domain_counter)
    }
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

    let mut init_queue: Vec<String> = Vec::new();
    init_queue.push(String::from("https://alantrotter.com/"));
    // init_queue.push(String::from("https://en.wikipedia.org/wiki/Brussels"));
    // init_queue.push(String::from("https://www.nytimes.com/"));

    let mut crawler = Crawler::new();
    crawler.init_crawl(&init_queue).await;

    // now, the main and long loop
    while crawler.yet_to_visit.len() > 0 && crawler.counter < MAXIMUM_CRAWLED_WEBSITES {
        _ = crawler.one_domain_crawl().await;
    }

    println!();
    println!("Crawling ended. Websites crawled: {}", crawler.counter);
}
