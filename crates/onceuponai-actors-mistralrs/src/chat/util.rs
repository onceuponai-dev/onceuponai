use image::DynamicImage;
use tokio::{
    fs::{self, File},
    io::AsyncReadExt,
};

pub async fn parse_image_url(url_unparsed: &str) -> Result<DynamicImage, anyhow::Error> {
    let url = if let Ok(url) = url::Url::parse(url_unparsed) {
        url
    } else if File::open(url_unparsed).await.is_ok() {
        url::Url::from_file_path(std::path::absolute(url_unparsed)?)
            .map_err(|_| anyhow::anyhow!("Could not parse file path: {}", url_unparsed))?
    } else {
        url::Url::parse(&format!("data:image/png;base64,{}", url_unparsed))
            .map_err(|_| anyhow::anyhow!("Could not parse as base64 data: {}", url_unparsed))?
    };

    let bytes = if url.scheme() == "http" || url.scheme() == "https" {
        // Read from http
        match reqwest::get(url.clone()).await {
            Ok(http_resp) => http_resp.bytes().await?.to_vec(),
            Err(e) => anyhow::bail!(e),
        }
    } else if url.scheme() == "file" {
        let path = url
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("Could not parse file path: {}", url))?;

        if let Ok(mut f) = File::open(&path).await {
            // Read from local file
            let metadata = fs::metadata(&path).await?;
            let mut buffer = vec![0; metadata.len() as usize];
            f.read_exact(&mut buffer).await?;
            buffer
        } else {
            anyhow::bail!("Could not open file at path: {}", url);
        }
    } else if url.scheme() == "data" {
        // Decode with base64
        let data_url = data_url::DataUrl::process(url.as_str())?;
        data_url.decode_to_vec()?.0
    } else {
        anyhow::bail!("Unsupported URL scheme: {}", url.scheme());
    };

    Ok(image::load_from_memory(&bytes)?)
}
