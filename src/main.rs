use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs;
use std::thread;
use std::time::Duration;
use texting_robots::{get_robots_url, Robot};

const MAXIMUM_WEBSITES_PER_DOMAIN: usize = 256;
const REQUEST_DELAY: Duration = Duration::from_millis(250);
const USER_AGENT: &str = "alex-observer/0.1.0";

/// Arbitrary processing function.
fn process_html_document(_content: String, _document: Html) {
    // This is highly customizable function.
    // Whole response body as `String` is passed as `content` parameter.
    // Parsed HTML document is passed as `document` parameter.
    // A few examples for the usage: calling external function, training AI model,
    // other types of text processing...

    // To get useful displayed text from HTML document, try this code:
    // let selector = Selector::parse("h1, h2, h3, h4, h5, h6, p").unwrap();
    // let mut text = String::new();
    // for element in document.select(&selector) {
    //     text.push_str(&element.text().collect::<Vec<&str>>().join("\n"));
    // }
}

/// Helper struct for storing information about visited domain.
struct DomainInfo {
    /// Number of successfully visited websites with this domain.
    counter: usize,
    /// Parsed `Robot` struct (optional). If not present, every URL is allowed.
    robot: Option<Robot>,
}

impl DomainInfo {
    /// Decides whether the maximum number of websites for this domain has been reached.
    fn reached_limit(&self) -> bool {
        self.counter >= MAXIMUM_WEBSITES_PER_DOMAIN
    }
}

/// Main crawling structure capable of crawling the web.
struct Crawler {
    /// The `Reqwest` client instance used to make HTTP requests.
    client: Client,
    /// Set of all already-visited URLs stored as strings.
    visited: HashSet<String>,
    /// Queue of all URLs that are yet to be visited.
    yet_to_visit: VecDeque<Url>,
    /// Hash map containing information about visited domains, including parsed `robots.txt`.
    domains: HashMap<String, DomainInfo>,
    /// Number of websites that have been crawled (not domains).
    counter: usize,
}

impl Crawler {
    /// Creates a new `Crawler` instance.
    fn new() -> Crawler {
        Crawler {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .user_agent(USER_AGENT)
                .build()
                .expect("should create a request client"),
            visited: HashSet::new(),
            yet_to_visit: VecDeque::new(),
            domains: HashMap::new(),
            counter: 0,
        }
    }

    /// Performs a HTTP request using crawler's client. Returns response text.
    async fn request_website(&self, url: &Url) -> Option<String> {
        // since the response can be arbitrary, we try to make a HEAD request first
        // to determine the Content-Type header - should be "text/html" only
        // and we profit from the speed of HEAD over GET
        let req = self
            .client
            .head(url.as_str())
            .header(ACCEPT, "text/html")
            .build()
            .unwrap();
        // safe to unwrap (already a valid url)
        let res = self.client.execute(req).await;
        if res.is_err() {
            return None; // something bad happened during the request
        }
        let res = res.unwrap(); // safe to unwrap
        let content_type = res.headers().get(CONTENT_TYPE);
        if let Some(content_type) = content_type {
            if !content_type.to_str().unwrap().starts_with("text/html") {
                return None; // we received different Content-Type than "text/html"
            }
        } else {
            return None; // the content-type header was not provided, sadly
        }

        // create a GET request object and then execute it
        let req = self
            .client
            .get(url.as_str())
            .header(ACCEPT, "text/html")
            .build()
            .unwrap(); // safe to unwrap, because the URL is valid
        let res = self.client.execute(req).await;
        if res.is_err() {
            return None; // something bad happened during the request
        }
        let res = res.unwrap();
        let text = res.text().await;
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
                println!(
                    "Something unexpected happened while loading URL: {}",
                    startup_url
                );
                continue;
            }
            let html = html.unwrap();
            self.visited.insert(url.as_str().to_string());

            // parse the HTML document and add links to the yet-to-visit list
            let (document, links) = find_links(&html, &url);
            for another_url in links {
                if !self.visited.contains(&another_url.as_str().to_string()) {
                    self.yet_to_visit.push_back(another_url);
                }
            }

            process_html_document(html, document);
        }
    }

    /// This is the main crawling function. Websites are fetched according
    /// to their order in the yet-to-visit queue. Content in `robots.txt`
    /// is cached. There is a `MAXIMUM_WEBSITES_PER_DOMAIN` limit for every domain,
    /// because some domains contain A LOT of websites (news, blogs, wiki...)
    async fn long_crawl(&mut self) {
        while let Some(url) = self.yet_to_visit.pop_front() {
            let domain = url.domain();
            if domain.is_none() {
                continue; // we don't want to crawl IP-address-like hosts
            }
            let domain = domain.unwrap();

            // if we haven't encountered this domain before
            if !self.domains.contains_key(domain) {
                let domain_info = DomainInfo {
                    counter: 0,
                    robot: self.get_robot_for_domain(&url).await,
                };
                self.domains.insert(domain.to_string(), domain_info);
            }
            let domain_info = self.domains.get(domain).unwrap(); // safe to unwrap

            // crawl this url only when we haven't yet reached domain limit
            if domain_info.reached_limit() {
                continue;
            }
            // is this url allowed?
            let url_allowed = match &domain_info.robot {
                Some(robot) => robot.allowed(url.as_str()),
                None => true,
            };
            if !url_allowed {
                continue;
            }

            // perform a request
            println!(
                "Requesting #{} ({} on domain {}): {} ",
                self.counter + 1,
                domain_info.counter + 1,
                domain,
                url.path()
            );
            let html = self.request_website(&url).await;
            if html.is_none() {
                println!("Failed!");
                continue;
            }
            let html = html.unwrap(); // safe to unwrap

            // update the stats & other variables
            self.visited.insert(url.to_string());
            self.counter += 1;
            let domain_info = self.domains.get_mut(domain).unwrap(); // save to unwrap
            domain_info.counter += 1;

            // parse the document and iterate over the links
            let (document, links) = find_links(&html, &url);
            for another_url in links {
                if self.visited.contains(&another_url.to_string()) {
                    continue;
                }
                self.yet_to_visit.push_back(another_url);
            }
            process_html_document(html, document);

            // wait some time in between requests
            thread::sleep(REQUEST_DELAY);
        }
    }

    async fn get_robot_for_domain(&self, url: &Url) -> Option<Robot> {
        let robots_url = get_robots_url(url.as_str());
        if robots_url.is_err() {
            return None;
        }
        let robots_url = robots_url.unwrap();
        let robots_txt = self.request_robots(&robots_url).await;
        if robots_txt.is_err() {
            return None;
        }
        let robots_txt = robots_txt.unwrap();
        let robot = Robot::new(USER_AGENT, robots_txt.as_bytes());
        if robot.is_err() {
            // error parsing the robots.txt
            return None;
        }
        Some(robot.unwrap())
    }

    async fn request_robots(&self, url: &str) -> Result<String, reqwest::Error> {
        let req = self.client.get(url).header(ACCEPT, "text/plain").build()?;
        let res = self.client.execute(req).await?;
        let text = res.text().await?;
        Ok(text)
    }
}

/// Helper function. Given a HTML string representation, returns a tuple.
/// First value is parsed `scraper::Html` document, second value is a vector of `Url`s
/// referenced in the string slice argument.
fn find_links(html: &str, url: &Url) -> (Html, Vec<Url>) {
    // we could also use Regex url pattern, but I think the document parsing is better
    let document = Html::parse_document(html);
    let href_selector = Selector::parse("body a, body area, body link").unwrap();

    let domain = url.domain().unwrap(); // safe to unwrap

    let links = document
        .select(&href_selector)
        .filter_map(|element| {
            let href = element.value().attr("href")?;

            // ignore stylesheets
            if let Some(rel) = element.value().attr("rel") {
                if rel == "stylesheet" {
                    return None;
                }
            }

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
        .filter_map(|url_string| {
            let url = Url::parse(&url_string);
            if url.is_err() {
                return None; // invalid URL
            }
            let mut url = url.unwrap();
            url.set_fragment(None);
            url.set_query(None);
            Some(url)
        })
        .collect();

    (document, links)
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
    crawler.long_crawl().await;

    println!();
    println!("Crawling ended. Websites crawled: {}", crawler.counter);
}
