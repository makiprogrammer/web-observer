# Observer web crawler

[...]

## Crawling mechanism

The crawler keeps a list of already-visited and yet-to-visit URLs.
The main cycle ends when specified website counter is reached, e.g. 100k websites.
Each iteration of this cycle begins with a selection of a domain (or a few domains
when parallelization is enabled). First, the `robots.txt` file is fetched and following
requests to subpages are made according to specified rules in `robots.txt` for that domain.
Newly discovered URLs are added to general yet-to-visit queue. If a new URL matches
the current domain, the URL is added to the list of yet-to-visit list for this domain
and request is made within this cycle iteration. In addition, a specified friendly delay is added
in between each request. If parallelization is enabled, this delay may be smaller.
