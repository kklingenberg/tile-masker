mod masker;

use actix_web::{
    error::InternalError,
    http::{header::ContentType, StatusCode},
    web, App, HttpResponse, HttpServer, Result as ActixResult,
};
use actix_web_prom::PrometheusMetricsBuilder;
use anyhow::{bail, Result};
use clap::Parser;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::{fs, fs::DirEntry};
use tracing::info;
use url::Url;

/// Tile-masking interface for XYZ png tiles.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// TCP port to listen on
    #[arg(short, long, env, default_value_t = 10000)]
    port: u16,

    /// Base URL to proxy png files from.
    #[arg(short, long, env)]
    base_url: Option<Url>,

    /// Volume to serve png files from.
    #[arg(short, long, env)]
    volume: Option<PathBuf>,

    /// Log level
    #[arg(long, env, default_value_t = tracing::Level::INFO)]
    log_level: tracing::Level,
}

#[derive(Deserialize)]
struct MaskQuery {
    mask: Option<String>,
}

impl MaskQuery {
    /// Parses this query into a set of u32 colors.
    fn clean(&self) -> BTreeSet<u32> {
        self.mask
            .clone()
            .unwrap_or_default()
            .trim()
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter_map(|s| u32::from_str_radix(s, 16).ok())
            .collect()
    }
}

/// Mask the given file fetched from a remote source
async fn mask_remote(
    base_url: web::Data<Url>,
    path: web::Path<String>,
    query: web::Query<MaskQuery>,
) -> ActixResult<HttpResponse> {
    let with_suffix = format!("{}.png", path.as_str());
    let url = base_url
        .join(&with_suffix)
        .map_err(|e| InternalError::new(e, StatusCode::NOT_FOUND))?;
    let body = web::block(move || {
        let mask = query.clean();
        masker::process_remote(url, mask)
    })
    .await??;
    Ok(HttpResponse::Ok()
        .content_type(ContentType::png())
        .insert_header(("Cache-Control", "public, max-age=31536000"))
        .body(body))
}

/// Mask the given file fetched from disk
async fn mask_local(
    volume: web::Data<PathBuf>,
    snapshot: web::Data<BTreeSet<PathBuf>>,
    path: web::Path<String>,
    query: web::Query<MaskQuery>,
) -> ActixResult<HttpResponse> {
    let filepath = volume.join(format!("{}.png", path.as_str()));
    if !snapshot.contains(&filepath) {
        Ok(HttpResponse::NotFound()
            .content_type(ContentType::plaintext())
            .body("file not found"))
    } else {
        let body = web::block(move || {
            let mask = query.clean();
            if !mask.is_empty() {
                masker::process_local(filepath, mask)
            } else {
                std::fs::read(filepath)
            }
        })
        .await??;
        Ok(HttpResponse::Ok()
            .content_type(ContentType::png())
            .insert_header(("Cache-Control", "public, max-age=31536000"))
            .body(body))
    }
}

/// Walk a directory visiting files.
/// Source: <https://doc.rust-lang.org/std/fs/fn.read_dir.html>
fn visit_dirs<F>(dir: &Path, cb: &mut F) -> io::Result<()>
where
    F: FnMut(&DirEntry),
{
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}

#[actix_web::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_max_level(cli.log_level)
        .with_target(false)
        .without_time()
        .init();
    if cli.base_url.is_none() && cli.volume.is_none() {
        bail!("One of --base-url or --volume must be provided");
    } else if cli.base_url.is_some() && cli.volume.is_some() {
        bail!("Only one of --base-url or --volume must be provided");
    }

    let mut snapshot = BTreeSet::new();
    if let Some(v) = &cli.volume {
        visit_dirs(v, &mut |entry: &DirEntry| {
            if matches!(
                entry.path().extension().and_then(|e| e.to_str()),
                Some("png")
            ) {
                snapshot.insert(entry.path().to_path_buf());
            }
        })?;
    }
    let prometheus = PrometheusMetricsBuilder::new("tilemasker")
        .endpoint("/metrics")
        .build()
        .unwrap();
    info!("Listening at port {0}", cli.port);
    HttpServer::new(move || {
        let base_app = App::new().wrap(prometheus.clone());
        if cli.base_url.is_some() {
            base_app
                .app_data(web::Data::new(cli.base_url.clone().unwrap()))
                .route("/{path:.*}.png", web::get().to(mask_remote))
        } else {
            base_app
                .app_data(web::Data::new(cli.volume.clone().unwrap()))
                .app_data(web::Data::new(snapshot.clone()))
                .route("/{path:.*}.png", web::get().to(mask_local))
        }
    })
    .bind(("0.0.0.0", cli.port))?
    .run()
    .await?;
    Ok(())
}
