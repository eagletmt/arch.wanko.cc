#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use futures::StreamExt as _;

    let bucket = std::env::var("BUCKET").expect("BUCKET variable is missing");
    let index_input = IndexInput {
        repositories: vec![
            "aur-eagletmt".to_owned(),
            "vim-latest".to_owned(),
            "ruby-trunk".to_owned(),
        ],
    };

    let mut package_inputs = futures::stream::FuturesUnordered::new();
    for repo in &index_input.repositories {
        package_inputs.push(fetch_repository_input(bucket.clone(), repo.clone()));
    }

    let mut handlebars = handlebars::Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars.register_template_file("index", "templates/index.html")?;
    handlebars.register_template_file("repository", "templates/repository.html")?;

    let mut html_uploads = futures::stream::FuturesUnordered::new();
    html_uploads.push(upload_html(
        bucket.clone(),
        "index.html".to_owned(),
        handlebars.render("index", &index_input)?,
    ));
    while let Some(package_input) = package_inputs.next().await {
        let package_input = package_input?;
        html_uploads.push(upload_html(
            bucket.clone(),
            format!("{}/index.html", package_input.name),
            handlebars.render("repository", &package_input)?,
        ));
    }
    while let Some(r) = html_uploads.next().await {
        r?;
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct IndexInput {
    repositories: Vec<String>,
}

#[derive(serde::Serialize)]
struct RepositoryInput {
    name: String,
    packages: Vec<PackageInput>,
}
#[derive(serde::Serialize)]
struct PackageInput {
    name: String,
    version: String,
    filename: String,
    builddate_timestamp: i64,
    builddate_str: String,
}

async fn fetch_repository_input(
    bucket: String,
    name: String,
) -> Result<RepositoryInput, anyhow::Error> {
    use futures::TryStreamExt as _;
    use rusoto_s3::S3 as _;
    use std::io::Read as _;

    let s3_client = rusoto_s3::S3Client::new(Default::default());
    let resp = s3_client
        .get_object(rusoto_s3::GetObjectRequest {
            bucket: bucket.to_owned(),
            key: format!("{}/os/x86_64/{}.db", name, name),
            ..Default::default()
        })
        .await?;
    let body = resp
        .body
        .unwrap()
        .map_ok(|b| bytes::BytesMut::from(&b[..]))
        .try_concat()
        .await?;
    let gz_reader = flate2::read::GzDecoder::new(body.as_ref());
    let mut tar_reader = tar::Archive::new(gz_reader);
    let mut packages = Vec::new();
    for entry in tar_reader.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        match path.file_name() {
            Some(filename) if filename == "desc" => {
                let mut desc_body = String::new();
                entry.read_to_string(&mut desc_body)?;
                packages.push(parse_desc(&desc_body)?);
            }
            _ => {}
        }
    }
    Ok(RepositoryInput {
        name: name.to_owned(),
        packages,
    })
}

fn parse_desc(body: &str) -> Result<PackageInput, anyhow::Error> {
    let mut name = None;
    let mut version = None;
    let mut filename = None;
    let mut builddate = None;

    let mut key = "";
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with('%') && line.ends_with('%') {
            key = &line[1..line.len() - 1];
        } else if line.is_empty() {
            key = "";
        } else {
            match key {
                "NAME" => {
                    name = Some(line.to_owned());
                }
                "VERSION" => {
                    version = Some(line.to_owned());
                }
                "FILENAME" => {
                    filename = Some(line.to_owned());
                }
                "BUILDDATE" => {
                    builddate = Some(line.parse()?);
                }
                _ => {}
            }
        }
    }

    if name.is_none() {
        return Err(anyhow::anyhow!("Failed to find NAME from desc file"));
    }
    if version.is_none() {
        return Err(anyhow::anyhow!("Failed to find VERSION from desc file"));
    }
    if filename.is_none() {
        return Err(anyhow::anyhow!("Failed to find FILENAME from desc file"));
    }
    if builddate.is_none() {
        return Err(anyhow::anyhow!("Failed to find BUILDDATE from desc file"));
    }
    Ok(PackageInput {
        name: name.unwrap(),
        version: version.unwrap(),
        filename: filename.unwrap(),
        builddate_timestamp: builddate.unwrap(),
        builddate_str: chrono::DateTime::<chrono::Utc>::from_utc(
            chrono::NaiveDateTime::from_timestamp(builddate.unwrap(), 0),
            chrono::Utc,
        )
        .to_rfc3339(),
    })
}

async fn upload_html(bucket: String, key: String, body: String) -> Result<(), anyhow::Error> {
    use md5::Digest as _;
    use rusoto_s3::S3 as _;

    let s3_client = rusoto_s3::S3Client::new(Default::default());
    let content_md5 = Some(base64::encode(md5::Md5::digest(body.as_bytes())));
    s3_client
        .put_object(rusoto_s3::PutObjectRequest {
            bucket,
            key,
            content_type: Some("text/html; charset=utf-8".to_owned()),
            content_md5,
            body: Some(body.into_bytes().into()),
            ..Default::default()
        })
        .await?;
    Ok(())
}
