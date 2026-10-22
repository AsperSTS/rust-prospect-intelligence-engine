//! Small on-demand street map. No geocoding, credentials, JavaScript or prefetch.
use futures_util::{StreamExt, TryStreamExt, stream};
use std::{
    fmt::Write,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub left: f64,
    pub top: f64,
}

/// Only tiles intersecting the 896 × 640 view. Web Mercator excludes the poles.
pub fn visible_tiles(lat: f64, lon: f64, zoom: u8) -> crate::Result<Vec<Tile>> {
    if !lat.is_finite()
        || !lon.is_finite()
        || lat.abs() > 85.05112878
        || lon.abs() > 180.0
        || !(12..=18).contains(&zoom)
    {
        return Err(crate::invalid("Coordenadas o escala del mapa inválidas"));
    }
    let n = 2u32.pow(u32::from(zoom));
    let world = f64::from(n);
    let x = (lon + 180.0) / 360.0 * world;
    let rad = lat.to_radians();
    let y = (1.0 - (rad.tan() + 1.0 / rad.cos()).ln() / std::f64::consts::PI) / 2.0 * world;
    let left = x * 256.0 - 448.0;
    let top = y * 256.0 - 320.0;
    let mut tiles = Vec::new();
    for row in (top / 256.0).floor() as i64..((top + 640.0) / 256.0).ceil() as i64 {
        if !(0..i64::from(n)).contains(&row) {
            continue;
        }
        for column in (left / 256.0).floor() as i64..((left + 896.0) / 256.0).ceil() as i64 {
            tiles.push(Tile {
                z: zoom,
                x: column.rem_euclid(i64::from(n)) as u32,
                y: row as u32,
                left: column as f64 * 256.0 - left,
                top: row as f64 * 256.0 - top,
            });
        }
    }
    Ok(tiles)
}

const MAX_TILE: usize = 512 * 1024;
const CACHE_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
fn valid_png(bytes: &[u8]) -> bool {
    bytes.len() >= 24
        && bytes.len() <= MAX_TILE
        && bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        && &bytes[12..16] == b"IHDR"
        && bytes[16..24] == [0, 0, 1, 0, 0, 0, 1, 0]
}
fn image_url(bytes: &[u8]) -> String {
    let mut url = String::with_capacity(15 + bytes.len() * 3);
    url.push_str("data:image/png,");
    for byte in bytes {
        write!(&mut url, "%{byte:02X}").expect("writing to String");
    }
    url
}
async fn tile_image(client: &reqwest::Client, cache: &Path, tile: &Tile) -> crate::Result<String> {
    let path = cache.join(format!("{}-{}-{}.png", tile.z, tile.x, tile.y));
    if let Ok(meta) = std::fs::metadata(&path)
        && meta.len() <= MAX_TILE as u64
        && meta.modified()?.elapsed().unwrap_or_default() < CACHE_AGE
        && let Ok(bytes) = std::fs::read(&path)
        && valid_png(&bytes)
    {
        return Ok(image_url(&bytes));
    }
    let response = client
        .get(format!(
            "https://tile.openstreetmap.org/{}/{}/{}.png",
            tile.z, tile.x, tile.y
        ))
        .send()
        .await
        .map_err(|_| crate::invalid("Mapa no disponible. Revisa tu conexión y vuelve a intentar."))?
        .error_for_status()
        .map_err(|_| crate::invalid("OpenStreetMap no está disponible en este momento."))?;
    let mut body = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|_| crate::invalid("No se pudo leer el mapa"))?;
        if bytes.len() + chunk.len() > MAX_TILE {
            return Err(crate::invalid("Imagen del mapa demasiado grande"));
        }
        bytes.extend_from_slice(&chunk);
    }
    if !valid_png(&bytes) {
        return Err(crate::invalid(
            "El servidor no devolvió una imagen válida del mapa",
        ));
    }
    std::fs::create_dir_all(cache)?;
    // A completed valid tile only; temporary names avoid concurrent partial cache reads.
    let temporary = path.with_extension(format!("{}.tmp", crate::operation_id()));
    std::fs::write(&temporary, &bytes)?;
    if let Err(error) = std::fs::rename(&temporary, &path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(image_url(&bytes))
}
pub async fn load_map(
    home: PathBuf,
    lat: f64,
    lon: f64,
    zoom: u8,
) -> crate::Result<Vec<(Tile, String)>> {
    let tiles = visible_tiles(lat, lon, zoom)?;
    let client = reqwest::Client::builder()
        .user_agent(concat!(
            "ProspectIntelligence/",
            env!("CARGO_PKG_VERSION"),
            " personal desktop map"
        ))
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| crate::invalid("No se pudo iniciar el mapa"))?;
    let cache = home.join("map-cache");
    stream::iter(tiles)
        .map(|tile| {
            let client = client.clone();
            let cache = cache.clone();
            async move {
                let image = tile_image(&client, &cache, &tile).await?;
                Ok((tile, image))
            }
        })
        .buffer_unordered(4)
        .try_collect::<Vec<(Tile, String)>>()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tile_view_covers_selected_location_and_stays_bounded() {
        for zoom in 12..=18 {
            let tiles = visible_tiles(19.0433, -98.2019, zoom).unwrap();
            assert!((12..=20).contains(&tiles.len()));
            assert!(tiles.iter().any(|t| t.left <= 448.0
                && t.left + 256.0 > 448.0
                && t.top <= 320.0
                && t.top + 256.0 > 320.0));
            assert!(
                tiles
                    .iter()
                    .all(|t| t.x < 2u32.pow(u32::from(zoom)) && t.y < 2u32.pow(u32::from(zoom)))
            );
        }
    }
    #[test]
    fn invalid_coordinates_and_zoom_never_generate_network_requests() {
        for (lat, lon, z) in [
            (f64::NAN, 0.0, 15),
            (0.0, f64::INFINITY, 15),
            (90.0, 0.0, 15),
            (0.0, 181.0, 15),
            (19.0, -98.0, 20),
        ] {
            assert!(visible_tiles(lat, lon, z).is_err());
        }
        assert!(visible_tiles(0.0, 180.0, 15).is_ok());
        assert!(visible_tiles(85.05, -180.0, 12).is_ok());
        assert!(!valid_png(b"<html>error</html>"));
    }
}
