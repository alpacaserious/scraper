#![feature(string_remove_matches)]
#![feature(iter_intersperse)]
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

    let mut imgs = get_imgs_from_url(url, client, 1).await;

    for i in &mut imgs {
        i.remove_matches("thumb_");
        i.insert(0, '/');
        i.insert_str(0, base_url);
    }

    println!("Found {} images", imgs.len());

    let path = get_path(&html);
    std::fs::create_dir_all(&path).expect("created gallery folder");

    for (i, link) in imgs.iter().enumerate() {
        print!("\rDownloading [{}] of [{}]", i + 1, imgs.len());
        get_image(link, client, &path, i).await;
    }
    println!();
}

/// From a given HTML, for example returns `Public Appearances/2024/Gallery_Title`
fn get_path(html: &Html) -> String {
    let paths = Selector::parse(".tableh1-statlink > .statlink > a").expect("parsed album path");
    let mut path_elems: Vec<String> = html
        .select(&paths)
        .skip(1)
        .map(|s| s.inner_html())
        .collect();

    // Replace unwanted chars in title
    if let Some(title) = path_elems.last_mut() {
        *title = title.replace('"', "").replace('/', "_");
    }

    let mut path: String = path_elems
        .iter()
        .intersperse(&String::from('/'))
        .cloned()
        .collect();
    path.push('/');

    path
}

/// Finds if there is another following page from the current HTML layout
fn get_next_page(html: &Html, page_idx: usize) -> Option<usize> {
    let links = Selector::parse(".navmenu > a").expect("parsed to find next page button");
    html.select(&links)
        .map(|l| l.attr("href").expect("href"))
        .map(|l| l.rsplit('=').next().expect("link should have a page query"))
        .map(|l| l.parse::<usize>().expect("parsed page as usize"))
        .find(|i| *i == page_idx + 1)
}

async fn get_imgs_from_url(url: &str, client: &Client, page_idx: usize) -> Vec<String> {
    println!("Getting links from page {page_idx}");

    let res = client.get(url).send().await.expect("GET request succesful");
    let body = res.text().await.expect("get the response text");
    let html = Html::parse_document(&body);

    let img_links = Selector::parse(".thumbnails > table > tbody > tr > td > a > img")
        .expect("parsed to find thumbnail link");

    let next_page_links = match get_next_page(&html, page_idx) {
        Some(n) => {
            let next_url = format!("{url}&page={n}");
            Box::pin(get_imgs_from_url(&next_url, client, page_idx + 1)).await
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
