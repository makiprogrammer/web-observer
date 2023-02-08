# Observer web crawler

A simple web crawler for static and server-side rendered websites.
For more details, see [Crawling mechanism](#crawling-mechanism) section
or detailed [Documentation](#documentation). The crawler takes a list of initial websites
and then follows links in `<a href="">` html tags. The crawler is able to respect rules specified
in `robots.txt`. Currently, parallelization is not supported, but it is planned to be added in the future.

After each successful website fetch, a special function `process_html_document` is called. The arguments are
`content: String` and the equivalent parsed `document: scraper::Html`.
This function is reserved for customizable processing of the loaded documents.
Its purpose is an end user's decision.

## Setup & startup

To startup this project, you need to have Rust tools and Cargo installed.
You can find instructions on how to install Rust [here](https://www.rust-lang.org/tools/install).

To build/run the project, use the `cargo` CLI. To build the project, run `cargo build`.
To run the project, run `cargo run -- input_file_path.txt`. Or, you can use pre-defined
inital input file using `cargo run -- init_queue.txt`.

## Crawling mechanism

On startup, crawler reads a list of initial websites from a specified input file as a command-line argument.
Each line in the input file is treated as a separate website. Then, the crawler starts to request
these input websites. The loaded documents are processed and all links in `<a href="">` html tags
are extracted to the _yet-to-visit_ queue.
Alongside the _yet-to-visit_ queue,
there is an _already-visited_ set, providing information about what websites have been visited.

During the main cycle, first URL in the _yet-to-visit_ queue is selected.
Since the crawler is a robot, it should respect the `robots.txt` file. Parsed contents of this file
are cached with a counter for each domain - there should be a limit on how many websites
can we visit for each domain, because there are many servers with A LOT of websites (news articles, wikis...).
Otherwise, the crawling process would take many hours.
If the limit for the domain isn't yet reached AND the `robots.txt` file allows the crawler to visit
the website (if none is present, it's allowed), we request the website and update the statistics.
Subsequent links are added again to the _yet-to-visit_ queue and requested url is added to _already-visited_ set.

In addition, a specified delay is added in between each request. This step may be omitted.
This above-mentioned mechanism is compatible with future parallelization feature development.

## Error handling

There is no custom error struct nor enum. If an initial URL is invalid, a short message is written
to the console. Other invalid URLs (parsed from later documents) are just simply ignored.
If a request is unsuccessful, it will not be repeated. If there is an unsupported Content-Type response header
(unsupported is everything except `text/html`), the website is ignored.

## Documentation

All of the code is located in [`main.rs`](src/main.rs) source file. Detailed information about the code can be
found in the source file itself.

The `main` function reads environment arguments and locates the input file. It creates a `Crawler` instance
and initiates the crawling functions.

The `Crawler` struct has some fields. First, there is an HTTP client from the `Reqwest` crate,
crawled website `counter` as well as above-mentioned fields: `yet_to_visit` queue and `visited` set.
Initial websites are requested in `Crawler::init_crawl` method. The main crawling cycle is implemented
in `Crawler::long_crawl`: requesting one website at a time, handling `robots.txt` resources and finding
subsequent links.

Additionally, there are a few helper functions, such as `find_links`, which returns a list of links
founded in a HTML document.
