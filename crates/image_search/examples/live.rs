fn main() -> anyhow::Result<()> {
    let client = image_search::SearchClient::new()?;
    let page = client.search("pixel art cat", 0)?;
    anyhow::ensure!(!page.images.is_empty(), "No image results");
    let mut successful = None;
    for item in page.images.iter().take(8) {
        if let (Ok(thumb), Ok(image)) = (client.thumbnail(item), client.original(item)) {
            successful = Some((item, thumb, image));
            break;
        }
    }
    let (item, thumb, image) = successful
        .ok_or_else(|| anyhow::anyhow!("No downloadable image among the first results"))?;
    println!(
        "LIVE_SEARCH_PASS: {} results; first image {}; thumbnail {} bytes; original {} bytes; license {}",
        page.images.len(),
        item.title,
        thumb.len(),
        image.len(),
        item.license
    );
    Ok(())
}
