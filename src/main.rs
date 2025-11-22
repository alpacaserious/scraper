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

    if url.contains("/thumbnails.php") {
        download_album(&url, &client).await;
    } else if url.contains("/index.php?cat=") {
        download_category(&url, &client).await;
    } else {
        println!("Unsupported URL");
    }
}

async fn download_category(url: &str, client: &Client) {
    let base_idx = url
        .find("/index.php")
        .expect("URL should contain '/index.php'");
    let base_url = url.split_at(base_idx).0;

    let albums = get_alb_links(url, client, 1).await;
    println!("Found {} albums", albums.len());

    for (i, a) in albums.iter().enumerate() {
        print!("\rDownloading [{}] of [{}]", i + 1, albums.len());
        download_album(&format!("{base_url}/{a}"), client).await;
    }
}

async fn get_alb_links(url: &str, client: &Client, page_idx: usize) -> Vec<String> {
    let res = client.get(url).send().await.expect("GET request succesful");

    let body = res.text().await.expect("get the response text");
    let document = Html::parse_document(&body);
    let alb_links = Selector::parse(".alblink > a").expect("parsed to find album links");

    let next_page_links = match get_next_page(&document, page_idx) {
        Some(n) => {
            let next_url = format!("{url}&page={n}");
            Box::pin(get_alb_links(&next_url, client, page_idx + 1)).await
        }
        None => vec![],
    };

    document
        .select(&alb_links)
        .filter_map(|l| l.attr("href"))
        .map(std::string::ToString::to_string)
        .chain(next_page_links)
        .collect()
}

async fn download_album(url: &str, client: &Client) {
    let base_idx = url
        .find("/thumbnails")
        .expect("URL should contain '/thumbnails'");
    let base_url = url.split_at(base_idx).0;

    let res = client.get(url).send().await.expect("GET request succesful");
    let body = res.text().await.expect("get the response text");
    let html = Html::parse_document(&body);

    let mut links = get_links_from_html(url, &html, 1).await;

    for l in &mut links {
        l.remove_matches("thumb_");
        l.insert(0, '/');
        l.insert_str(0, base_url);
    }

    println!("Found {} images", links.len());

    let title = get_title(&html).await;
    std::fs::create_dir(&title).expect("created gallery folder");

    for (i, link) in links.iter().enumerate() {
        print!("\rDownloading [{}] of [{}]", i + 1, links.len());
        get_image(link, client, &title, i).await;
    }
    println!();
}

async fn get_title(html: &Html) -> String {
    let titles = Selector::parse("td > h2").expect("parsed to find thumbnail link");

    let title = html
        .select(&titles)
        .next_back()
        .map(|t| t.inner_html())
        .unwrap();
    let title = title.replace('"', "");
    title.replace("/", "_")
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

async fn get_links_from_html(url: &str, html: &Html, page_idx: usize) -> Vec<String> {
    println!("Getting links from page {page_idx}");

    let img_links = Selector::parse(".thumbnails > table > tbody > tr > td > a > img")
        .expect("parsed to find thumbnail link");

    let next_page_links = match get_next_page(&html, page_idx) {
        Some(n) => {
            let next_url = format!("{url}&page={n}");
            Box::pin(get_links_from_html(&next_url, html, page_idx + 1)).await
        }
        None => vec![],
    };

    html.select(&img_links)
        .filter_map(|l| l.attr("src"))
        .map(std::string::ToString::to_string)
        .chain(next_page_links)
        .collect()
}

async fn get_image(url: &str, client: &Client, gallery: &str, idx: usize) {
    let res = client.get(url).send().await.expect("GET request succesful");
    let data = res.bytes().await.expect("get the response bytes");

    let mut f = File::create(format!("{gallery}/{}.jpg", idx + 1)).expect("created file");
    f.write_all(&data).expect("wrote the bytes to file");
}
