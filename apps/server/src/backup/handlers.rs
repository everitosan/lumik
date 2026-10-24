//! `POST /v1/projects/{project_id}/photos/{photo_id}/backup` — sube el JPEG a los
//! remotes del fotógrafo y actualiza el estado en el catálogo.

use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::middleware::AuthPhotographer;
use crate::error::{ApiError, ApiResult};
use crate::rclone::Rclone;
use crate::AppState;

#[derive(Serialize)]
pub struct BackupItemResult {
    pub provider: String,
    pub status: String, // "uploaded" | "failed"
    pub remote_path: Option<String>,
    pub error: Option<String>,
}

/// Remote conectado (credenciales cifradas).
#[derive(sqlx::FromRow)]
struct RemoteRow {
    provider: String,
    name: String,
    token: Option<String>,
}

pub async fn backup_photo(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Path((project_id, photo_id)): Path<(Uuid, Uuid)>,
    mut multipart: Multipart,
) -> ApiResult<Json<Value>> {
    // ---- 1. Parseo del multipart ----
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut targets: Option<String> = None;
    let mut quality: Option<String> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("multipart inválido: {e}")))?
    {
        match field.name() {
            Some("file") => {
                if filename.is_none() {
                    filename = field.file_name().map(|s| s.to_string());
                }
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("archivo inválido: {e}")))?;
                file_bytes = Some(data.to_vec());
            }
            Some("filename") => filename = Some(field.text().await.unwrap_or_default()),
            Some("targets") => targets = Some(field.text().await.unwrap_or_default()),
            Some("quality") => quality = Some(field.text().await.unwrap_or_default()),
            _ => {
                let _ = field.bytes().await;
            }
        }
    }
    let file_bytes = file_bytes.ok_or_else(|| ApiError::BadRequest("falta el campo 'file'".into()))?;
    let filename = filename.ok_or_else(|| ApiError::BadRequest("falta 'filename'".into()))?;

    // ---- 2. Verifica que el proyecto es del fotógrafo y asegura la fila photo ----
    let owns: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM project WHERE id = $1 AND photographer_id = $2")
            .bind(project_id)
            .bind(auth.0)
            .fetch_optional(&state.pool)
            .await?;
    if owns.is_none() {
        return Err(ApiError::NotFound);
    }
    // Upsert de la foto (permite respaldar archivos de _exported que no provienen
    // de un import, p.ej. exports renombrados). No toca backup_*.
    sqlx::query(
        "INSERT INTO photo (id, project_id, filename) VALUES ($1, $2, $3) \
         ON CONFLICT (id) DO UPDATE SET filename = EXCLUDED.filename, updated_at = now()",
    )
    .bind(photo_id)
    .bind(project_id)
    .bind(&filename)
    .execute(&state.pool)
    .await?;

    // ---- 3. Ruta en la nube (espeja el disco) ----
    // Si se recomprime, el archivo remoto será JPEG → ajusta la extensión.
    let quality_preset = quality.as_deref().and_then(quality_preset);
    let effective_filename = if quality_preset.is_some() {
        to_jpg_name(&filename)
    } else {
        filename.clone()
    };
    let dir = ensure_backup_dir(&state, project_id).await?;
    let dest = format!("{dir}/{effective_filename}");

    // ---- 4. Remotes del fotógrafo (opcionalmente filtrados por targets) ----
    let prepared = resolve_remotes(&state, auth.0, targets).await?;
    if prepared.is_empty() {
        return Err(ApiError::BadRequest(
            "no hay proveedores conectados para respaldar".into(),
        ));
    }

    // ---- 5. Subida (rclone es bloqueante → spawn_blocking) ----
    let rclone_bin = state.cfg.rclone_bin.clone();
    let dest_cl = dest.clone();
    let (results, sha256, size): (Vec<BackupItemResult>, String, i64) =
        tokio::task::spawn_blocking(move || {
        use sha2::{Digest, Sha256};
        let rc = Rclone::new(rclone_bin);
        // Recomprime en el server si se pidió una calidad (ahorra espacio en la nube).
        let bytes = match quality_preset {
            Some((max_edge, q)) => convert_jpeg(&file_bytes, max_edge, q).unwrap_or(file_bytes),
            None => file_bytes,
        };
        // Integridad: hash y tamaño de lo que realmente se sube. `rclone copyto`
        // verifica el hash origen↔destino al copiar, así que un backup 'uploaded'
        // con estos valores confirma que el remoto coincide con estos bytes.
        let size = bytes.len() as i64;
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        let tmp = std::env::temp_dir().join(format!("lumik-up-{}", Uuid::new_v4()));
        if let Err(e) = std::fs::write(&tmp, &bytes) {
            // Si no se puede escribir el temporal, todos fallan.
            let results = prepared
                .into_iter()
                .map(|(provider, _, _)| BackupItemResult {
                    provider,
                    status: "failed".into(),
                    remote_path: None,
                    error: Some(format!("temp: {e}")),
                })
                .collect();
            return (results, sha256, size);
        }
        let mut results = Vec::new();
        for (provider, name, cfg) in prepared {
            let attempt = rc
                .config_for(&name, &cfg)
                .and_then(|conf| rc.copy_to(&conf, &name, &tmp, &dest_cl));
            match attempt {
                Ok(()) => results.push(BackupItemResult {
                    provider,
                    status: "uploaded".into(),
                    remote_path: Some(format!("{name}:{dest_cl}")),
                    error: None,
                }),
                Err(e) => results.push(BackupItemResult {
                    provider,
                    status: "failed".into(),
                    remote_path: None,
                    error: Some(e),
                }),
            }
        }
        let _ = std::fs::remove_file(&tmp);
        (results, sha256, size)
    })
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("tarea de subida: {e}")))?;

    // ---- 6. Actualiza el catálogo ----
    let ok: Vec<&BackupItemResult> = results.iter().filter(|r| r.status == "uploaded").collect();
    let all_ok = !results.is_empty() && ok.len() == results.len();
    let status = if all_ok { "uploaded" } else { "failed" };
    let backup_url = ok.first().and_then(|r| r.remote_path.clone());
    let providers = ok
        .iter()
        .map(|r| r.provider.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let backup_date: Option<DateTime<Utc>> = if ok.is_empty() { None } else { Some(Utc::now()) };
    // Registro de integridad solo si al menos un remoto verificó la subida.
    let (int_sha, int_size): (Option<&str>, Option<i64>) = if ok.is_empty() {
        (None, None)
    } else {
        (Some(sha256.as_str()), Some(size))
    };

    sqlx::query(
        "UPDATE photo SET backup_status = $1, backup_url = COALESCE($2, backup_url), \
         provider = NULLIF($3, ''), backup_date = COALESCE($4, backup_date), \
         backup_sha256 = COALESCE($5, backup_sha256), backup_size = COALESCE($6, backup_size), \
         backup_retries = CASE WHEN $1 = 'failed' THEN backup_retries + 1 ELSE backup_retries END, \
         updated_at = now() WHERE id = $7",
    )
    .bind(status)
    .bind(&backup_url)
    .bind(&providers)
    .bind(backup_date)
    .bind(int_sha)
    .bind(int_size)
    .bind(photo_id)
    .execute(&state.pool)
    .await?;

    tracing::info!(
        photographer_id = %auth.0, %photo_id, status, dest = %dest,
        "backup de foto"
    );

    Ok(Json(json!({
        "results": results,
        "backup_status": status,
        "backup_date": backup_date,
        "sha256": int_sha,
        "size": int_size,
    })))
}

/// Ruta calculada a partir del nombre/fecha actuales: `Lumik/{año}/{mes}/{día}_{slug}/_exported`.
fn computed_dest_dir(session_date: Option<NaiveDate>, slug: &str, created_at: DateTime<Utc>) -> String {
    let date = session_date.unwrap_or_else(|| created_at.date_naive());
    format!(
        "Lumik/{}/{}/{}_{}/_exported",
        date.format("%Y"),
        date.format("%m"),
        date.format("%d"),
        slug
    )
}

/// Directorio destino en la nube del proyecto, **anclado**: si ya hay `backup_dir`
/// guardado, se reutiliza (path pegajoso); si no, se calcula del nombre/fecha
/// actuales y se guarda como ancla. Así renombrar no parte la carpeta.
async fn ensure_backup_dir(state: &AppState, project_id: Uuid) -> ApiResult<String> {
    let row: (Option<String>, Option<NaiveDate>, String, DateTime<Utc>) = sqlx::query_as(
        "SELECT backup_dir, session_date, slug, created_at FROM project WHERE id = $1",
    )
    .bind(project_id)
    .fetch_one(&state.pool)
    .await?;
    if let Some(dir) = row.0 {
        return Ok(dir);
    }
    let computed = computed_dest_dir(row.1, &row.2, row.3);
    sqlx::query("UPDATE project SET backup_dir = $1 WHERE id = $2")
        .bind(&computed)
        .bind(project_id)
        .execute(&state.pool)
        .await?;
    Ok(computed)
}

/// Remotes del fotógrafo (opcionalmente filtrados por `targets` csv), con su
/// config rclone ya descifrada: `(provider, name, cfg)`.
async fn resolve_remotes(
    state: &AppState,
    photographer_id: Uuid,
    targets: Option<String>,
) -> ApiResult<Vec<(String, String, Value)>> {
    let target_set: Option<Vec<String>> = targets
        .filter(|t| !t.is_empty() && t != "all")
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect());

    let remotes: Vec<RemoteRow> =
        sqlx::query_as("SELECT provider, name, token FROM remote WHERE photographer_id = $1")
            .bind(photographer_id)
            .fetch_all(&state.pool)
            .await?;

    let mut prepared: Vec<(String, String, Value)> = Vec::new();
    for r in remotes {
        if let Some(set) = &target_set {
            if !set.contains(&r.provider) {
                continue;
            }
        }
        let token = r
            .token
            .as_deref()
            .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("remote sin credenciales")))?;
        let plain = state.crypto.decrypt(token).map_err(ApiError::Internal)?;
        let cfg: Value = serde_json::from_str(&plain)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("config de remote inválida: {e}")))?;
        prepared.push((r.provider, r.name, cfg));
    }
    Ok(prepared)
}

#[derive(Deserialize)]
pub struct PrepareReq {
    /// Proveedores destino (csv) o `"all"`/ausente = todos.
    pub targets: Option<String>,
}

/// `POST /v1/projects/{id}/backup/prepare` — crea el directorio `_exported/` en
/// cada remote destino ANTES de subir en paralelo, para no duplicar la carpeta
/// (Mega permite nombres repetidos). Idempotente.
pub async fn prepare_backup(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Path(project_id): Path<Uuid>,
    Json(req): Json<PrepareReq>,
) -> ApiResult<Json<Value>> {
    // Verifica dueño del proyecto.
    let owns: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM project WHERE id = $1 AND photographer_id = $2")
            .bind(project_id)
            .bind(auth.0)
            .fetch_optional(&state.pool)
            .await?;
    if owns.is_none() {
        return Err(ApiError::NotFound);
    }

    let dir = ensure_backup_dir(&state, project_id).await?;
    let prepared = resolve_remotes(&state, auth.0, req.targets).await?;
    if prepared.is_empty() {
        return Err(ApiError::BadRequest(
            "no hay proveedores conectados para respaldar".into(),
        ));
    }

    let rclone_bin = state.cfg.rclone_bin.clone();
    let dir_cl = dir.clone();
    let created = tokio::task::spawn_blocking(move || {
        let rc = Rclone::new(rclone_bin);
        let mut ok = 0usize;
        for (provider, name, cfg) in prepared {
            match rc
                .config_for(&name, &cfg)
                .and_then(|conf| rc.mkdir(&conf, &name, &dir_cl))
            {
                Ok(()) => ok += 1,
                Err(e) => tracing::warn!(%provider, error = %e, "mkdir remoto falló"),
            }
        }
        ok
    })
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("tarea mkdir: {e}")))?;

    tracing::info!(photographer_id = %auth.0, %project_id, created, dir = %dir, "prepare backup (mkdir)");
    Ok(Json(json!({ "prepared": created })))
}

/// Proveedores cuyos enlaces compartibles soportan expiración (rclone `--expire`).
/// Solo estos generan deep-link; el resto abre la web del proveedor.
const EXPIRING_PROVIDERS: &[&str] = &["dropbox", "onedrive"];
/// Vigencia del enlace generado.
const LINK_EXPIRE: &str = "168h"; // 7 días

#[derive(Deserialize)]
pub struct LinkReq {
    pub provider: String,
}

/// `POST /v1/projects/{id}/backup/link` — genera un enlace **expirable** a la
/// carpeta `_exported/` del proyecto en el proveedor indicado. Si el proveedor no
/// soporta expiración, responde `{ "url": null }` (el cliente abre su web).
pub async fn folder_link(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Path(project_id): Path<Uuid>,
    Json(req): Json<LinkReq>,
) -> ApiResult<Json<Value>> {
    // Solo proveedores con expiración; nunca creamos enlaces permanentes.
    if !EXPIRING_PROVIDERS.contains(&req.provider.as_str()) {
        return Ok(Json(json!({ "url": null, "reason": "no_expiring_support" })));
    }

    let owns: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM project WHERE id = $1 AND photographer_id = $2")
            .bind(project_id)
            .bind(auth.0)
            .fetch_optional(&state.pool)
            .await?;
    if owns.is_none() {
        return Err(ApiError::NotFound);
    }

    let dir = ensure_backup_dir(&state, project_id).await?;
    let remote = resolve_remotes(&state, auth.0, Some(req.provider.clone()))
        .await?
        .into_iter()
        .next();
    let Some((_provider, name, cfg)) = remote else {
        return Err(ApiError::BadRequest("proveedor no conectado".into()));
    };

    let rclone_bin = state.cfg.rclone_bin.clone();
    let url = tokio::task::spawn_blocking(move || {
        let rc = Rclone::new(rclone_bin);
        rc.config_for(&name, &cfg)
            .and_then(|conf| rc.link(&conf, &name, &dir, LINK_EXPIRE))
    })
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("tarea link: {e}")))?;

    match url {
        Ok(url) => {
            tracing::info!(photographer_id = %auth.0, %project_id, provider = %req.provider, "enlace de carpeta generado");
            Ok(Json(json!({ "url": url })))
        }
        Err(e) => {
            tracing::warn!(provider = %req.provider, error = %e, "rclone link falló");
            // Fallback silencioso: el cliente abre la web del proveedor.
            Ok(Json(json!({ "url": null, "reason": "link_failed" })))
        }
    }
}

/// `POST /v1/projects/{id}/backup/relocate` — mueve la carpeta del backup en la
/// nube para que coincida con el nombre/fecha actuales del proyecto (tras un
/// rename). Actualiza el ancla `backup_dir` y los `backup_url`.
pub async fn relocate_backup(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Path(project_id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let row: Option<(Option<String>, Option<NaiveDate>, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT backup_dir, session_date, slug, created_at FROM project \
         WHERE id = $1 AND photographer_id = $2",
    )
    .bind(project_id)
    .bind(auth.0)
    .fetch_optional(&state.pool)
    .await?;
    let Some((stored, session_date, slug, created_at)) = row else {
        return Err(ApiError::NotFound);
    };
    let desired = computed_dest_dir(session_date, &slug, created_at);

    // Sin backup previo: solo fija el ancla al nombre actual.
    let Some(stored) = stored else {
        sqlx::query("UPDATE project SET backup_dir = $1 WHERE id = $2")
            .bind(&desired)
            .bind(project_id)
            .execute(&state.pool)
            .await?;
        return Ok(Json(json!({ "moved": false })));
    };
    if stored == desired {
        return Ok(Json(json!({ "moved": false })));
    }

    let prepared = resolve_remotes(&state, auth.0, None).await?;
    let rclone_bin = state.cfg.rclone_bin.clone();
    let (from, to) = (stored.clone(), desired.clone());
    let move_res: Result<(), String> = tokio::task::spawn_blocking(move || {
        let rc = Rclone::new(rclone_bin);
        for (_provider, name, cfg) in prepared {
            let conf = rc.config_for(&name, &cfg)?;
            rc.move_dir(&conf, &name, &from, &to)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("tarea move: {e}")))?;

    match move_res {
        Ok(()) => {
            sqlx::query("UPDATE project SET backup_dir = $1 WHERE id = $2")
                .bind(&desired)
                .bind(project_id)
                .execute(&state.pool)
                .await?;
            sqlx::query(
                "UPDATE photo SET backup_url = replace(backup_url, $1, $2) \
                 WHERE project_id = $3 AND backup_url IS NOT NULL",
            )
            .bind(&stored)
            .bind(&desired)
            .bind(project_id)
            .execute(&state.pool)
            .await?;
            tracing::info!(%project_id, from = %stored, to = %desired, "carpeta de backup movida");
            Ok(Json(json!({ "moved": true, "from": stored, "to": desired })))
        }
        Err(e) => {
            tracing::warn!(%project_id, error = %e, "relocate de carpeta falló");
            Err(ApiError::Internal(anyhow::anyhow!(
                "no se pudo mover la carpeta: {e}"
            )))
        }
    }
}

/// `DELETE /v1/projects/{id}/backup` — borra el respaldo del proyecto: su
/// carpeta en cada remote del fotógrafo y sus fotos del catálogo. El proyecto
/// sigue en el catálogo (el desktop lo vuelve a publicar) y el siguiente
/// respaldo sube todo de nuevo. Si un remote falla, el catálogo no se toca, para
/// poder reintentar.
pub async fn delete_backup(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Path(project_id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT backup_dir FROM project WHERE id = $1 AND photographer_id = $2")
            .bind(project_id)
            .bind(auth.0)
            .fetch_optional(&state.pool)
            .await?;
    let Some((stored,)) = row else {
        return Err(ApiError::NotFound);
    };

    // Sin ancla no se subió nada: no hay carpeta que borrar.
    let mut purged = 0usize;
    if let Some(dir) = stored {
        let prepared = resolve_remotes(&state, auth.0, None).await?;
        let rclone_bin = state.cfg.rclone_bin.clone();
        let purge_res: Result<usize, String> = tokio::task::spawn_blocking(move || {
            let rc = Rclone::new(rclone_bin);
            let mut purged = 0;
            for (_provider, name, cfg) in prepared {
                let conf = rc.config_for(&name, &cfg)?;
                if rc.purge(&conf, &name, &dir)? {
                    purged += 1;
                }
                // La carpeta del proyecto (padre de `_exported`) se va si quedó vacía.
                if let Some((parent, _)) = dir.rsplit_once('/') {
                    let _ = rc.rmdir(&conf, &name, parent);
                }
            }
            Ok(purged)
        })
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("tarea purge: {e}")))?;
        purged = purge_res.map_err(|e| {
            tracing::warn!(%project_id, error = %e, "borrar respaldo falló");
            ApiError::Internal(anyhow::anyhow!("no se pudo borrar la carpeta en la nube: {e}"))
        })?;
    }

    let deleted = sqlx::query("DELETE FROM photo WHERE project_id = $1")
        .bind(project_id)
        .execute(&state.pool)
        .await?
        .rows_affected();
    sqlx::query(
        "UPDATE project SET backup_dir = NULL, workflow_status = NULL, updated_at = now() WHERE id = $1",
    )
        .bind(project_id)
        .execute(&state.pool)
        .await?;

    tracing::info!(%project_id, deleted, purged, "respaldo del proyecto borrado");
    Ok(Json(json!({ "deleted_photos": deleted, "purged_remotes": purged })))
}

/// Mapea el nivel de calidad a `(lado mayor máx en px, calidad JPEG)`.
/// `None` (o "original") → sin recomprimir.
fn quality_preset(q: &str) -> Option<(u32, u8)> {
    match q {
        "high" => Some((3840, 90)),
        "medium" => Some((2048, 82)),
        "low" => Some((1280, 75)),
        _ => None, // "original" u desconocido
    }
}

/// Cambia la extensión de un nombre a `.jpg` (para archivos recomprimidos).
fn to_jpg_name(filename: &str) -> String {
    match filename.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.jpg"),
        None => format!("{filename}.jpg"),
    }
}

/// Decodifica, redimensiona (si excede `max_edge`) y re-encodea como JPEG.
/// Devuelve `None` si no se puede decodificar (se usará el original).
///
/// La orientación se aplica a los píxeles: el JPG de cámara de una foto
/// vertical guarda los píxeles en horizontal y la etiqueta Orientation. Los
/// metadatos del original (EXIF, XMP, IPTC y perfil ICC) se copian al
/// resultado con la Orientation en 1, para que la firma del fotógrafo
/// (autor, copyright) llegue a la nube y la foto no se gire dos veces.
fn convert_jpeg(bytes: &[u8], max_edge: u32, jpeg_quality: u8) -> Option<Vec<u8>> {
    use image::codecs::jpeg::JpegEncoder;
    use image::imageops::FilterType;
    use image::{DynamicImage, ImageDecoder, ImageReader};

    let mut decoder = ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_decoder()
        .ok()?;
    let orientation = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).ok()?;
    if let Some(orientation) = orientation {
        img.apply_orientation(orientation);
    }
    let img = if img.width().max(img.height()) > max_edge {
        // Triangle (bilineal): mucho más rápido que Lanczos3 con buena calidad
        // al reducir; el cuello de botella era el filtro.
        img.resize(max_edge, max_edge, FilterType::Triangle)
    } else {
        img
    };
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, jpeg_quality)
        .encode_image(&rgb)
        .ok()?;
    Some(with_metadata_of(bytes, out))
}

const EXIF_HEADER: &[u8] = b"Exif\0\0";
const XMP_HEADER: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const ICC_HEADER: &[u8] = b"ICC_PROFILE\0";
const IPTC_HEADER: &[u8] = b"Photoshop 3.0\0";

/// Segmentos de metadatos de un JPEG (marcador + longitud + contenido), en
/// orden: EXIF y XMP (APP1), ICC (APP2) e IPTC (APP13). Se detiene en el
/// inicio de los datos de imagen (SOS).
fn metadata_segments(jpeg: &[u8]) -> Vec<Vec<u8>> {
    let mut segments = Vec::new();
    if !jpeg.starts_with(&[0xFF, 0xD8]) {
        return segments;
    }
    let mut pos = 2;
    while pos + 4 <= jpeg.len() && jpeg[pos] == 0xFF {
        let marker = jpeg[pos + 1];
        if marker == 0xFF {
            pos += 1; // relleno
            continue;
        }
        if marker == 0xDA || marker == 0xD9 {
            break;
        }
        let len = u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]) as usize;
        let end = pos + 2 + len;
        if len < 2 || end > jpeg.len() {
            break;
        }
        let payload = &jpeg[pos + 4..end];
        let keep = match marker {
            0xE1 => payload.starts_with(EXIF_HEADER) || payload.starts_with(XMP_HEADER),
            0xE2 => payload.starts_with(ICC_HEADER),
            0xED => payload.starts_with(IPTC_HEADER),
            _ => false,
        };
        if keep {
            segments.push(jpeg[pos..end].to_vec());
        }
        pos = end;
    }
    segments
}

/// Pone en 1 la Orientation del IFD0 de un segmento EXIF (APP1). Sin el tag,
/// o con un TIFF malformado, no cambia nada.
fn reset_exif_orientation(segment: &mut [u8]) {
    let Some(tiff) = segment.get_mut(4 + EXIF_HEADER.len()..) else {
        return;
    };
    let big_endian = match tiff.get(..2) {
        Some(b"MM") => true,
        Some(b"II") => false,
        _ => return,
    };
    let read_u16 = |b: &[u8], at: usize| -> Option<u16> {
        let v = [*b.get(at)?, *b.get(at + 1)?];
        Some(if big_endian { u16::from_be_bytes(v) } else { u16::from_le_bytes(v) })
    };
    let read_u32 = |b: &[u8], at: usize| -> Option<u32> {
        let v = [*b.get(at)?, *b.get(at + 1)?, *b.get(at + 2)?, *b.get(at + 3)?];
        Some(if big_endian { u32::from_be_bytes(v) } else { u32::from_le_bytes(v) })
    };
    let Some(ifd0) = read_u32(tiff, 4).map(|v| v as usize) else {
        return;
    };
    let Some(count) = read_u16(tiff, ifd0) else {
        return;
    };
    for i in 0..count as usize {
        let entry = ifd0 + 2 + i * 12;
        if read_u16(tiff, entry) == Some(0x0112) {
            let one = if big_endian { 1u16.to_be_bytes() } else { 1u16.to_le_bytes() };
            if let Some(value) = tiff.get_mut(entry + 8..entry + 10) {
                value.copy_from_slice(&one);
            }
            return;
        }
    }
}

/// Pone en 1 la `tiff:Orientation` de un segmento XMP, como atributo o como
/// elemento. El valor es de un dígito, así que la longitud no cambia.
fn reset_xmp_orientation(segment: &mut [u8]) {
    for (open, close) in [(&b"tiff:Orientation=\""[..], b'"'), (&b"<tiff:Orientation>"[..], b'<')] {
        let mut from = 0;
        while let Some(at) = segment[from..]
            .windows(open.len())
            .position(|w| w == open)
            .map(|p| from + p + open.len())
        {
            if segment.get(at).is_some_and(u8::is_ascii_digit) && segment.get(at + 1) == Some(&close) {
                segment[at] = b'1';
            }
            from = at;
        }
    }
}

/// Inserta en `encoded` (recién codificado, sin metadatos) los metadatos de
/// `original`, con la orientación en 1. Van tras el APP0 (JFIF) del encoder.
fn with_metadata_of(original: &[u8], encoded: Vec<u8>) -> Vec<u8> {
    let mut segments = metadata_segments(original);
    if segments.is_empty() || !encoded.starts_with(&[0xFF, 0xD8]) {
        return encoded;
    }
    for segment in &mut segments {
        let payload = &segment[4..];
        if payload.starts_with(EXIF_HEADER) {
            reset_exif_orientation(segment);
        } else if payload.starts_with(XMP_HEADER) {
            reset_xmp_orientation(segment);
        }
    }
    let mut insert_at = 2;
    if encoded.get(2..4) == Some(&[0xFF, 0xE0]) {
        let len = u16::from_be_bytes([encoded[4], encoded[5]]) as usize;
        insert_at = 4 + len;
    }
    let mut out = Vec::with_capacity(encoded.len() + segments.iter().map(Vec::len).sum::<usize>());
    out.extend_from_slice(&encoded[..insert_at]);
    for segment in &segments {
        out.extend_from_slice(segment);
    }
    out.extend_from_slice(&encoded[insert_at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageDecoder, ImageReader};

    const ARTIST: &[u8] = b"Ana Perez\0";
    const XMP: &str = "<x:xmpmeta><rdf:RDF><rdf:Description tiff:Orientation=\"6\">\
        <dc:creator>Ana Perez</dc:creator></rdf:Description></rdf:RDF></x:xmpmeta>";

    fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0xFF, marker];
        out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    /// JPEG de 40×20 con EXIF (Orientation `orientation` y Artist, en el orden
    /// de bytes `big_endian`) y XMP con `tiff:Orientation` y `dc:creator`.
    fn camera_jpeg(orientation: u16, big_endian: bool) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(40, 20, image::Rgb([200, 30, 30]));
        let mut plain = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut plain, 90)
            .encode_image(&img)
            .unwrap();

        let u16b = |v: u16| if big_endian { v.to_be_bytes() } else { v.to_le_bytes() };
        let u32b = |v: u32| if big_endian { v.to_be_bytes() } else { v.to_le_bytes() };
        // IFD0 en el offset 8 con dos entradas: Artist (ASCII, fuera de línea) y Orientation.
        let artist_at = 8 + 2 + 2 * 12 + 4;
        let mut tiff = if big_endian { b"MM".to_vec() } else { b"II".to_vec() };
        tiff.extend_from_slice(&u16b(42));
        tiff.extend_from_slice(&u32b(8));
        tiff.extend_from_slice(&u16b(2));
        tiff.extend_from_slice(&u16b(0x013B));
        tiff.extend_from_slice(&u16b(2));
        tiff.extend_from_slice(&u32b(ARTIST.len() as u32));
        tiff.extend_from_slice(&u32b(artist_at as u32));
        tiff.extend_from_slice(&u16b(0x0112));
        tiff.extend_from_slice(&u16b(3));
        tiff.extend_from_slice(&u32b(1));
        tiff.extend_from_slice(&u16b(orientation));
        tiff.extend_from_slice(&[0, 0]);
        tiff.extend_from_slice(&u32b(0));
        tiff.extend_from_slice(ARTIST);
        let exif = [EXIF_HEADER, &tiff].concat();
        let xmp = [XMP_HEADER, XMP.as_bytes()].concat();

        let mut out = plain[..2].to_vec(); // SOI
        out.extend(segment(0xE1, &exif));
        out.extend(segment(0xE1, &xmp));
        out.extend_from_slice(&plain[2..]);
        out
    }

    fn orientation_of(jpeg: &[u8]) -> image::metadata::Orientation {
        let mut decoder = ImageReader::new(std::io::Cursor::new(jpeg))
            .with_guessed_format()
            .unwrap()
            .into_decoder()
            .unwrap();
        decoder.orientation().unwrap()
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn convert_jpeg_applies_exif_orientation() {
        // Orientation 6 (girar 90°): una vertical de cámara llega en vertical.
        for big_endian in [true, false] {
            let out = convert_jpeg(&camera_jpeg(6, big_endian), 3840, 90).unwrap();
            let img = image::load_from_memory(&out).unwrap();
            assert_eq!((img.width(), img.height()), (20, 40));
            // Los píxeles ya van girados: la etiqueta queda en 1 para no girar dos veces.
            assert_eq!(orientation_of(&out), image::metadata::Orientation::NoTransforms);
        }
    }

    #[test]
    fn convert_jpeg_keeps_landscape_without_rotation() {
        let out = convert_jpeg(&camera_jpeg(1, true), 3840, 90).unwrap();
        let img = image::load_from_memory(&out).unwrap();
        assert_eq!((img.width(), img.height()), (40, 20));
    }

    #[test]
    fn convert_jpeg_keeps_photographer_metadata() {
        let out = convert_jpeg(&camera_jpeg(6, false), 3840, 90).unwrap();
        assert!(contains(&out, ARTIST), "EXIF Artist");
        assert!(contains(&out, b"<dc:creator>Ana Perez</dc:creator>"), "XMP dc:creator");
        assert!(contains(&out, b"tiff:Orientation=\"1\""), "XMP Orientation");
        assert!(!contains(&out, b"tiff:Orientation=\"6\""));
    }

    #[test]
    fn convert_jpeg_without_metadata_is_unchanged() {
        let img = image::RgbImage::from_pixel(8, 8, image::Rgb([0, 0, 0]));
        let mut plain = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut plain, 90)
            .encode_image(&img)
            .unwrap();
        let out = convert_jpeg(&plain, 3840, 90).unwrap();
        assert!(metadata_segments(&out).is_empty());
    }
}
