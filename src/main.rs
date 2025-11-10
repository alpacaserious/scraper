#![feature(string_remove_matches)]
#![warn(clippy::pedantic)]
use std::{fs::File, io::Write};

use reqwest::Client;
use scraper::{Html, Selector};

// 1st page:
//     https://daisy-ridley.org/photos/thumbnails.php?album=444
// More pages:
//     https://daisy-ridley.org/photos/thumbnails.php?album=444?page=n
// Thumbnail link from gallery:
//     https://daisy-ridley.org/photos/albums/userpics/10001/thumb_574591655_daisy-1.jpg
// Photo:
//     https://daisy-ridley.org/photos/albums/userpics/10001/574591655_daisy-1.jpg
#[tokio::main]
async fn main() {
    let url = std::env::args().nth(1).expect("URL should be provided");
    let client = Client::new();

    download_gallery(&url, &client).await;
}

async fn download_gallery(url: &str, client: &Client) {
    let base_idx = url
        .find("/thumbnails")
        .expect("URL should contain '/thumbnails'");
    let base_url = url.split_at(base_idx).0;

    let mut links = get_links_from_url(&url, &client, 1).await;

    for l in &mut links {
        l.remove_matches("thumb_");
        l.insert(0, '/');
        l.insert_str(0, base_url);
        println!("{l}");
    }

    let title = get_title(&url, &client).await;
    println!("{title}");

    // for (i, link) in links.iter().enumerate() {
    //     get_image(link, &client, title, i).await;
    // }
}

async fn get_title(url: &str, client: &Client) -> String {
    let res = client.get(url).send().await.expect("GET request succesful");

    let body = res.text().await.expect("get the response text");
    let document = Html::parse_document(&body);
    let titles = Selector::parse("td > h2").expect("parsed to find thumbnail link");

    let title = document
        .select(&titles)
        .last()
        .map(|t| t.inner_html())
        .unwrap();
    title.replace('"', &String::new())
}

/// Finds if there is another following page from the current HTML layout
fn get_next_page(html: &Html, page_idx: usize) -> Option<usize> {
    let links = Selector::parse(".navmenu > a").expect("parsed to find next page button");
    html.select(&links)
        .map(|l| l.attr("href").expect("href"))
        .map(|l| l.rsplit('=').next().expect("link should have a page query"))
        .next_back()
        .map(|l| l.parse::<usize>().expect("parsed page as usize"))
        .take_if(|i| *i == page_idx + 1)
}

async fn get_links_from_url(url: &str, client: &Client, page_idx: usize) -> Vec<String> {
    println!("Getting links from page {page_idx}");
    let res = client.get(url).send().await.expect("GET request succesful");

    let body = res.text().await.expect("get the response text");
    let document = Html::parse_document(&body);
    let img_links = Selector::parse(".thumbnails > table > tbody > tr > td > a > img")
        .expect("parsed to find thumbnail link");

    let next_page_links = match get_next_page(&document, page_idx) {
        Some(n) => {
            let next_url = format!("{url}?page={n}");
            Box::pin(get_links_from_url(&next_url, client, page_idx + 1)).await
        }
        None => vec![],
    };

    document
        .select(&img_links)
        .filter_map(|l| l.attr("src"))
        .map(|l| l.to_string())
        .chain(next_page_links)
        .collect::<Vec<_>>()
}

async fn get_image(url: &str, client: &Client, gallery: &str, idx: usize) {
    let res = client.get(url).send().await.expect("GET request succesful");
    let data = res.bytes().await.expect("get the response bytes");

    let mut f = File::create(format!("{gallery}/{idx}.jpg")).expect("created file");
    f.write_all(&data).expect("wrote the bytes to file");
}
