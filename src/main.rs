use anyhow::Context;
use maxminddb::{ Mmap, Reader, geoip2 };
use serde_json::{ Value, json };
use std::net::{ IpAddr };
use std::path::PathBuf;
use std::sync::Arc;
use std::env;
use tokio::net::TcpListener;
use clap::{ Parser };

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    bind: Option<String>,

    mmdb: PathBuf,
}

use axum::{ extract::{ Path, State }, http::StatusCode, routing::{ get }, Json, Router };
use tracing_subscriber::{ layer::SubscriberExt, util::SubscriberInitExt };

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tracing_subscriber
        ::registry()
        .with(
            tracing_subscriber::EnvFilter
                ::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Arc::new(maxminddb::Reader::open_mmap(args.mmdb)?);

    let app = Router::new()
        .route("/{ip}", get(resolve_ip::<GeoIpRepository>))
        .with_state(AppState { geo: GeoIpRepository { db } });

    let bind = args.bind.unwrap_or("127.0.0.1:3000".to_string());
    let listener = TcpListener::bind(bind).await.unwrap();

    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

#[derive(Clone)]
struct AppState {
    geo: GeoIpRepository,
}

async fn resolve_ip<T>(
    State(state): State<AppState>,
    Path(ip): Path<String>
) -> Result<Json<Value>, StatusCode>
    where T: GeoIpRepo
{
    let Ok(country) = state.geo.resolve_ip(ip) else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok(Json(country))
}

trait GeoIpRepo: Send + Sync {
    fn resolve_ip(&self, ip: String) -> anyhow::Result<Value>;
}

#[derive(Clone)]
struct GeoIpRepository {
    db: Arc<Reader<Mmap>>,
}

impl GeoIpRepo for GeoIpRepository {
    fn resolve_ip(&self, ip: String) -> anyhow::Result<Value> {
        let ip: IpAddr = ip.parse()?;
        let db = self.db.clone();

        let Some(country) = db.lookup::<geoip2::Country>(ip)? else {
            return Ok(json!({}));
        };

        let continent = country.continent.context("country")?;
        let country = country.country.context("continent")?;

        Ok(
            json!({
                "country": country.names.context("country")?.get("en").context("country name")?,
                "country_code": country.iso_code.context("country code")?,
                "continent": continent.names.context("continent")?.get("en").context("continent name")?
            })
        )
    }
}
