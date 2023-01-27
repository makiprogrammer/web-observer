# Observer web crawler

A simple web crawler for static and server-side rendered websites.
For more details, see [Crawling mechanism](#crawling-mechanism) section
or detailed [Documentation](#documentation). The crawler takes a list of initial websites
and then follows links in `<a href="">` html tags. The crawler is able to respect rules specified
in `robots.txt`. Currently, parallelization is not supported, but it is planned to be added in the future.

After each website fetch, a special function `process_html_document` with `String` argument is called.
This function is reserved for customizable processing of the loaded documents.
Its purpose is an end user's decision.

## Setup

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
Alongside the _yet-to-visit_ queue (which is not a real queue, it's more of a common list),
there is an _already-visited_ set, providing information about what websites have been visited.

During the main cycle, first URL in the _yet-to-visit_ queue is selected. Then, we create
a _same-domain-yet-to-visit_ queue with all URLs with the same domain as the first URL.
These URLs are consequently removed from the _yet-to-visit_ queue.
For this domain, we fetch the `robots.txt` file and parse it to get a list of rules for this domain.
A series of requests to all URLs for this domain is made according to the rules in `robots.txt`.
Loaded documents are processed and subsequent links are added to the general _yet-to-visit_ queue
OR to the _same-domain-yet-to-visit_ queue.
Next domain is selected only if _same-domain-yet-to-visit_ queue is empty.

In addition, a specified friendly delay is added
in between each request.

This mechanism is compatible with future parallelization feature development.

## Documentation

All of the code is located in `src/main.rs` source file. Detailed information about the code can be
found in the source file itself.

The `main` function reads environment arguments and locates the input file. It creates a `Crawler` instance
and handles the main crawling cycle. If the maximum number of websites crawled is reached, crawling ends.

The `Crawler` struct has some fields. First, there is an HTTP client from the `Reqwest` crate,
crawled website `counter` as well as above-mentioned fields: `yet_to_visit` list and `visited` set.
Initial websites are requested in `Crawler::init_crawl` method. The main crawling cycle is implemented
in `Crawler::one_domain_crawl`: selection of a domain, requesting `robots.txt` resource and finding
subsequent links.

Additionally, there are a few helper functions, such as `find_links`, which returns a list of links
founded in a HTML document.
