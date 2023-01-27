use reqwest::{Client, Url};
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::thread;
use std::time::Duration;
use texting_robots::{get_robots_url, Robot};

const MAXIMUM_CRAWLED_WEBSITES: usize = 99999;
const REQUEST_DELAY: Duration = Duration::from_millis(500);

/// Arbitrary processing function.
fn process_html_document(_content: String) {
    // This is highly customizable function.
    // Whole response body as text is passed as `content` parameter.
    // A few examples for the usage: calling external function, training AI model,
    // other types of text processing...

    // To get useful, displayed text from HTML document, try this code:
    // let document = Html::parse_document(html);
    // let selector = Selector::parse("h1, h2, h3, h4, h5, h6, p").unwrap();
    // let mut text = String::new();
    // for element in document.select(&selector) {
    //     text.push_str(&element.text().collect::<Vec<&str>>().join("\n"));
    // }
}

enum CrawlerError {
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
            if startup_url.is_empty() {
                continue;
            }
            let url = Url::parse(&startup_url);
            if url.is_err() {
                println!("Invalid URL: {}", startup_url);
                continue;
            }
            let mut url = url.unwrap();
            url.set_fragment(None);

            println!("Requesting startup page #{}: {}", i + 1, startup_url);
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

            process_html_document(html);
        }
    }

    /// Selects first url in the yet-to-visit url list and crawles all urls
    /// with the same domain. New urls are crawled if they have the same domain,
    /// otherwise, they are added to yet-to-visit list.
    async fn one_domain_crawl(&mut self) -> Result<usize, CrawlerError> {
        let first = self.yet_to_visit.first();
        if first.is_none() {
            return Ok(0);
        }

        // copy the domain of the first URL to owned string
        let domain = first.unwrap().domain().unwrap().to_owned();

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

        while let Some(url) = urls_with_same_domain.pop() {
            if self.counter >= MAXIMUM_CRAWLED_WEBSITES {
                break; // we've reached maximum number of crawled websites
            }

            // important: check if the URL is allowed by robots.txt
            if !robot.allowed(url.path()) {
                continue;
            }

            // request the document
            println!(
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

            process_html_document(html);

            // wait some friendly time in between requests
            thread::sleep(REQUEST_DELAY);
        }

        Ok(same_domain_counter)
    }
}

/// Helper function. Given a HTML string representation, returns a vector of `Url`s
/// referenced in the string slice argument.
fn find_links(html: &str, url: &Url) -> Vec<Url> {
    let document = Html::parse_document(html);
    let href_selector = Selector::parse("a").unwrap();

    let domain = url.domain().unwrap();

    document
        .select(&href_selector)
        .filter_map(|element| {
            let href = element.value().attr("href")?;

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
    // TODO: parallelization - using threads with shared memory? or by messaging?
    // TODO: 4xx Too Many Requests
    // TODO: add unit tests

    // read the env arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("This program requires an input file with initial URLs.");
        println!("Usage: {} <input_file_path>", args[0]);
        return;
    }
    let input_file_path = &args[1];
    let init_queue = fs::read_to_string(input_file_path);
    if let Err(error) = init_queue {
        println!(
            "Error reading input file: {}\n{}",
            input_file_path,
            error.to_string()
        );
        return;
    }
    let init_queue: Vec<String> = init_queue
        .unwrap()
        .split('\n')
        .map(&str::to_string)
        .collect();
    println!("Input file loaded successfully.");

    // startup the crawler
    let mut crawler = Crawler::new();
    crawler.init_crawl(&init_queue).await;

    // now, the main and long loop
    while crawler.yet_to_visit.len() > 0 && crawler.counter < MAXIMUM_CRAWLED_WEBSITES {
        _ = crawler.one_domain_crawl().await;
    }

    println!();
    println!("Crawling ended. Websites crawled: {}", crawler.counter);
}
