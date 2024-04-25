mod masker;

use actix_web::{
    http::header::ContentType, web, App, HttpResponse, HttpServer, Result as ActixResult,
};
use actix_web_prom::PrometheusMetricsBuilder;
use anyhow::Result;
use clap::Parser;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::PathBuf;
use tracing::info;

/// Tile-masking interface for XYZ png tiles.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// TCP port to listen on
    #[arg(short, long, env, default_value_t = 10000)]
    port: u16,

    /// Volume to serve png files from.
    #[arg(short, long, env)]
    volume: PathBuf,

    /// Log level
    #[arg(long, env, default_value_t = tracing::Level::INFO)]
    log_level: tracing::Level,
}

#[derive(Deserialize)]
struct MaskQuery {
    mask: Option<String>,
}

/// Mask the given file
async fn mask(
    volume: web::Data<PathBuf>,
    path: web::Path<String>,
    query: web::Query<MaskQuery>,
) -> ActixResult<HttpResponse> {
    let body = web::block(move || {
        let raw_mask = query.mask.clone().unwrap_or_default();
        let mask = raw_mask
            .trim()
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter_map(|s| u32::from_str_radix(s, 16).ok())
            .collect::<BTreeSet<_>>();
        if !mask.is_empty() {
            masker::process(volume.join(format!("{}.png", path.as_str())), mask)
        } else {
            std::fs::read(volume.join(format!("{}.png", path.as_str())))
        }
    })
    .await??;
    Ok(HttpResponse::Ok()
        .content_type(ContentType::png())
        .insert_header(("Cache-Control", "public, max-age=31536000"))
        .body(body))
}

#[actix_web::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_max_level(cli.log_level)
        .with_target(false)
        .without_time()
        .init();

    let prometheus = PrometheusMetricsBuilder::new("tilemasker")
        .endpoint("/metrics")
        .build()
        .unwrap();
    info!("Listening at port {0}", cli.port);
    HttpServer::new(move || {
        App::new()
            .wrap(prometheus.clone())
            .app_data(web::Data::new(cli.volume.clone()))
            .route("/{path:.*}.png", web::get().to(mask))
    })
    .bind(("0.0.0.0", cli.port))?
    .run()
    .await?;
    Ok(())
}
