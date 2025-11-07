#![feature(string_remove_matches)]
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
    let url = std::env::args()
        .skip(1)
        .next()
        .expect("URL should be provided");

    let base_idx = url.find("/thumbnails").unwrap();
    let (base_url, sub_url) = url.split_at(base_idx);

    let client = Client::new();
    let _ = get_pages(&url, &client).await;

    // TODO: scrape more pages
    let mut links = get_links_from_url(&url, &client).await;

    for l in &mut links {
        l.remove_matches("thumb_");
        l.insert(0, '/');
        l.insert_str(0, base_url);
        println!("{l}");
    }

    // for (i, link) in links.iter().enumerate() {
    //     get_image(&link, &client, i).await;
    // }
}

async fn get_pages(url: &str, client: &Client) -> Vec<String> {
    let res = client.get(url).send().await.unwrap();

    let body = res.text().await.unwrap();
    let doc = Html::parse_document(&body);
    let links = Selector::parse(".navmenu > a").unwrap();

    doc.select(&links)
        .for_each(|l| println!("{}", l.attr("href").unwrap()));
    vec![]
}

async fn get_links_from_url(url: &str, client: &Client) -> Vec<String> {
    let res = client.get(url).send().await.unwrap();

    let body = res.text().await.unwrap();
    let document = Html::parse_document(&body);
    let img_links = Selector::parse(".thumbnails > table > tbody > tr > td > a > img").unwrap();

    document
        .select(&img_links)
        .filter_map(|l| l.attr("src"))
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
}

async fn get_image(url: &str, client: &Client, idx: usize) {
    let res = client.get(url).send().await.unwrap();
    let data = res.bytes().await.unwrap();

    let mut f = File::create(format!("../downloads/{idx}.jpg")).unwrap();
    f.write_all(&data).unwrap();
}
