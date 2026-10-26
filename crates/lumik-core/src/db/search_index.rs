//! Índice de búsqueda en la base global (spec `photo-search`): una copia de las
//! fotos de cada proyecto visto por este equipo. Se reescribe por proyecto
//! completo en una transacción.

use super::{DbResult, GlobalDatabase};
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

/// Proyecto tal como queda en el índice.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchProjectRow {
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub session_date: Option<String>,
    pub created_at: String,
    pub archived: bool,
    pub device_uuid: Option<String>,
    pub device_name: Option<String>,
    pub name_norm: String,
    pub text_norm: String,
    pub generation: i64,
    /// Firma de `_exported/` (nombres, tamaños y fechas) al indexar.
    pub exports_signature: String,
}

/// Foto tal como queda en el índice.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchPhotoRow {
    pub photo_id: String,
    pub file_name: String,
    pub capture_date: Option<String>,
    pub stars: i32,
    /// `,1,3,`
    pub colors: String,
    pub tags: Option<String>,
    /// `,calle,retrato,`
    pub tags_norm: String,
    pub text_norm: String,
    pub culled: bool,
    pub delivered: bool,
    pub is_pair: bool,
    pub camera: Option<String>,
    pub camera_norm: String,
    pub lens: Option<String>,
    pub lens_norm: String,
    pub rotation: i32,
    pub thumb: Option<Vec<u8>>,
    /// Vector de contenido (`f32` little-endian) y el modelo que lo generó.
    pub embedding: Option<Vec<u8>>,
    pub embedding_model: Option<String>,
}

/// Archivo de `_exported/` tal como queda en el índice.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchExportRow {
    pub export_id: String,
    pub file_name: String,
    /// Foto a la que está ligado por nombre (`photo.jpg_path`), si alguna.
    pub photo_id: Option<String>,
    /// `tamaño:fecha` del archivo, para saber si cambió.
    pub signature: String,
    pub thumb: Option<Vec<u8>>,
}

/// Firma y miniatura ya indexadas de un exportado.
pub type ExportCache = (String, Option<Vec<u8>>);

/// Lo que ya hay en el índice de una foto y se puede conservar al reindexar.
#[derive(Debug, Clone, Default)]
pub struct IndexedPhotoCache {
    pub rotation: i32,
    pub thumb: Option<Vec<u8>>,
}

impl GlobalDatabase {
    /// Generación y firma de `_exported/` con las que se indexó el proyecto.
    pub fn indexed_state(&self, project_id: &str) -> DbResult<Option<(i64, String)>> {
        let conn = self.conn();
        Ok(conn
            .query_row(
                "SELECT indexed_generation, exports_signature FROM search_project WHERE project_id = ?1",
                [project_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?)
    }

    /// Generación con la que se indexó el proyecto, o `None` si no está.
    pub fn indexed_generation(&self, project_id: &str) -> DbResult<Option<i64>> {
        let conn = self.conn();
        Ok(conn
            .query_row(
                "SELECT indexed_generation FROM search_project WHERE project_id = ?1",
                [project_id],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Miniatura y rotación ya indexadas de las fotos del proyecto.
    pub fn indexed_photo_cache(&self, project_id: &str) -> DbResult<HashMap<String, IndexedPhotoCache>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT m.photo_id, m.rotation, m.thumb
             FROM search_media m JOIN search_photo p ON p.photo_id = m.photo_id
             WHERE p.project_id = ?1",
        )?;
        let rows = stmt
            .query_map([project_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    IndexedPhotoCache { rotation: r.get(1)?, thumb: r.get(2)? },
                ))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(rows)
    }

    /// Firma y miniatura ya indexadas de los exportados del proyecto.
    pub fn indexed_export_cache(&self, project_id: &str) -> DbResult<HashMap<String, ExportCache>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT export_id, signature, thumb FROM search_export WHERE project_id = ?1")?;
        let rows = stmt
            .query_map([project_id], |r| Ok((r.get::<_, String>(0)?, (r.get(1)?, r.get(2)?))))?
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(rows)
    }

    /// Reemplaza el proyecto, sus fotos y sus exportados en el índice.
    pub fn write_project_index(
        &self,
        project: &SearchProjectRow,
        photos: &[SearchPhotoRow],
        exports: &[SearchExportRow],
    ) -> DbResult<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO search_project (project_id, name, description, session_date, created_at, archived,
                                         device_uuid, device_name, name_norm, text_norm, indexed_generation, indexed_at,
                                         exports_signature)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(project_id) DO UPDATE SET
                name = excluded.name, description = excluded.description,
                session_date = excluded.session_date, created_at = excluded.created_at,
                archived = excluded.archived,
                device_uuid = COALESCE(excluded.device_uuid, search_project.device_uuid),
                device_name = COALESCE(excluded.device_name, search_project.device_name),
                name_norm = excluded.name_norm, text_norm = excluded.text_norm, indexed_generation = excluded.indexed_generation,
                indexed_at = excluded.indexed_at, exports_signature = excluded.exports_signature",
            params![
                project.project_id,
                project.name,
                project.description,
                project.session_date,
                project.created_at,
                project.archived as i32,
                project.device_uuid,
                project.device_name,
                project.name_norm,
                project.text_norm,
                project.generation,
                now,
                project.exports_signature,
            ],
        )?;
        // Borra también su search_media (ON DELETE CASCADE).
        tx.execute("DELETE FROM search_photo WHERE project_id = ?1", [&project.project_id])?;
        {
            let mut insert = tx.prepare(
                "INSERT INTO search_photo (photo_id, project_id, file_name, capture_date, stars, colors, tags,
                                           tags_norm, text_norm, culled, delivered, is_pair, camera, camera_norm,
                                           lens, lens_norm, embedding_model)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            )?;
            let mut insert_media = tx.prepare(
                "INSERT INTO search_media (photo_id, rotation, thumb, embedding) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for p in photos {
                insert.execute(params![
                    p.photo_id,
                    project.project_id,
                    p.file_name,
                    p.capture_date,
                    p.stars,
                    p.colors,
                    p.tags,
                    p.tags_norm,
                    p.text_norm,
                    p.culled as i32,
                    p.delivered as i32,
                    p.is_pair as i32,
                    p.camera,
                    p.camera_norm,
                    p.lens,
                    p.lens_norm,
                    p.embedding_model,
                ])?;
                insert_media.execute(params![p.photo_id, p.rotation, p.thumb, p.embedding])?;
            }
        }
        tx.execute("DELETE FROM search_export WHERE project_id = ?1", [&project.project_id])?;
        {
            let mut insert = tx.prepare(
                "INSERT INTO search_export (export_id, project_id, file_name, photo_id, signature, thumb)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for e in exports {
                insert.execute(params![e.export_id, project.project_id, e.file_name, e.photo_id, e.signature, e.thumb])?;
            }
        }
        tx.commit()?;
        self.index_changed();
        Ok(())
    }

    /// Quita un proyecto y sus fotos del índice (al eliminarlo).
    pub fn remove_project_index(&self, project_id: &str) -> DbResult<()> {
        let conn = self.conn();
        // search_media se va por ON DELETE CASCADE.
        conn.execute("DELETE FROM search_photo WHERE project_id = ?1", [project_id])?;
        conn.execute("DELETE FROM search_export WHERE project_id = ?1", [project_id])?;
        conn.execute("DELETE FROM search_project WHERE project_id = ?1", [project_id])?;
        self.index_changed();
        Ok(())
    }

    /// Cantidad de fotos indexadas (para el estado del índice).
    pub fn indexed_photo_count(&self) -> DbResult<i64> {
        let conn = self.conn();
        Ok(conn.query_row("SELECT COUNT(*) FROM search_photo", [], |r| r.get(0))?)
    }
}

/// Una foto encontrada, sin su miniatura (se pide aparte, solo para las visibles).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SearchHit {
    pub photo_id: String,
    pub project_id: String,
    pub project_name: String,
    pub project_date: Option<String>,
    /// Último disco donde se vio el proyecto (para "conecta…").
    pub device_name: Option<String>,
    pub file_name: String,
    pub capture_date: Option<String>,
    pub stars: i32,
    pub colors: Vec<u8>,
    pub tags: Option<String>,
    pub culled: bool,
    pub delivered: bool,
    pub is_pair: bool,
    /// El disco del proyecto está conectado (lo llena `SearchPhotos`).
    pub connected: bool,
    /// Parecido con el texto buscado, si salió por contenido.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f32>,
}

/// Vectores de un modelo cargados del índice, contiguos para recorrerlos rápido.
pub struct VectorCache {
    version: u64,
    model: String,
    dim: usize,
    ids: Vec<String>,
    data: Vec<f32>,
}

/// Fotos por analizar de un proyecto, entre las que pasan los filtros.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UnanalyzedProject {
    pub project_id: String,
    pub count: usize,
}

/// Columnas de un resultado (ver `hit_from_row`).
const HIT_SELECT: &str = "SELECT p.photo_id, p.project_id, pr.name, pr.session_date, pr.device_name, p.file_name,
        p.capture_date, p.stars, p.colors, p.tags, p.culled, p.delivered, p.is_pair
 FROM search_photo p JOIN search_project pr ON pr.project_id = p.project_id";

fn hit_from_row(r: &rusqlite::Row) -> rusqlite::Result<SearchHit> {
    let colors: String = r.get(8)?;
    Ok(SearchHit {
        photo_id: r.get(0)?,
        project_id: r.get(1)?,
        project_name: r.get(2)?,
        project_date: r.get(3)?,
        device_name: r.get(4)?,
        file_name: r.get(5)?,
        capture_date: r.get(6)?,
        stars: r.get(7)?,
        colors: colors.split(',').filter_map(|c| c.parse().ok()).collect(),
        tags: r.get(9)?,
        culled: r.get::<_, i32>(10)? != 0,
        delivered: r.get::<_, i32>(11)? != 0,
        is_pair: r.get::<_, i32>(12)? != 0,
        connected: false,
        similarity: None,
    })
}

/// Escapa `%`, `_` y `\` para usar `value` dentro de un `LIKE … ESCAPE '\'`.
fn like_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

/// Condiciones SQL y sus parámetros para una consulta; con `with_text` en
/// falso, sin las del texto libre (la búsqueda por contenido lo usa aparte).
fn where_clause(q: &crate::domain::search::SearchQuery, with_text: bool) -> (String, Vec<String>) {
    use crate::domain::search::{normalize, Period, StarsFilter};
    let mut conds: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    let mut arg = |value: String| {
        args.push(value);
        format!("?{}", args.len())
    };

    if !q.include_archived {
        conds.push("pr.archived = 0".into());
    }
    for t in &q.tags {
        conds.push(format!("p.tags_norm LIKE {} ESCAPE '\\'", arg(format!("%,{},%", like_escape(&normalize(t))))));
    }
    for t in &q.not_tags {
        conds.push(format!("p.tags_norm NOT LIKE {} ESCAPE '\\'", arg(format!("%,{},%", like_escape(&normalize(t))))));
    }
    match q.stars {
        Some(StarsFilter::AtLeast(n)) => conds.push(format!("p.stars >= {n}")),
        Some(StarsFilter::Exact(n)) => conds.push(format!("p.stars = {n}")),
        Some(StarsFilter::AtMost(n)) => conds.push(format!("p.stars <= {n}")),
        None => {}
    }
    if !q.colors.is_empty() {
        let any: Vec<String> = q.colors.iter().map(|c| format!("p.colors LIKE '%,{c},%'")).collect();
        conds.push(format!("({})", any.join(" OR ")));
    }
    if let Some(v) = &q.camera {
        conds.push(format!("p.camera_norm LIKE {} ESCAPE '\\'", arg(format!("%{}%", like_escape(&normalize(v))))));
    }
    if let Some(v) = &q.lens {
        conds.push(format!("p.lens_norm LIKE {} ESCAPE '\\'", arg(format!("%{}%", like_escape(&normalize(v))))));
    }
    if let Some(period) = q.period {
        let (from, to) = match period {
            Period::Year { year } => (format!("{year}-01-01"), format!("{}-01-01", year + 1)),
            Period::Month { year, month: 12 } => (format!("{year}-12-01"), format!("{}-01-01", year + 1)),
            Period::Month { year, month } => (format!("{year}-{month:02}-01"), format!("{year}-{:02}-01", month + 1)),
        };
        conds.push(format!("p.capture_date >= {} AND p.capture_date < {}", arg(from), arg(to)));
    }
    if let Some(v) = &q.before {
        conds.push(format!("p.capture_date < {}", arg(v.clone())));
    }
    if let Some(v) = &q.after {
        // Después del día, sin incluirlo.
        conds.push(format!("p.capture_date > {}", arg(format!("{v} 23:59:59"))));
    }
    if let Some(v) = &q.project {
        conds.push(format!("pr.name_norm LIKE {} ESCAPE '\\'", arg(format!("%{}%", like_escape(&normalize(v))))));
    }
    if q.approved {
        conds.push("p.culled = 1".into());
    }
    if q.delivered {
        conds.push("p.delivered = 1".into());
    }
    if q.pair {
        conds.push("p.is_pair = 1".into());
    }
    for w in q.text.iter().filter(|_| with_text) {
        let a = arg(format!("%{}%", like_escape(&normalize(w))));
        conds.push(format!("(p.text_norm LIKE {a} ESCAPE '\\' OR pr.text_norm LIKE {a} ESCAPE '\\')"));
    }
    let sql = if conds.is_empty() { "1".to_string() } else { conds.join(" AND ") };
    (sql, args)
}

impl GlobalDatabase {
    /// Fotos del índice que cumplen la consulta, de la captura más reciente a la
    /// más antigua. Devuelve hasta `limit` y si había más.
    pub fn search_photos(
        &self,
        q: &crate::domain::search::SearchQuery,
        limit: usize,
    ) -> DbResult<(Vec<SearchHit>, bool)> {
        if q.is_empty() {
            return Ok((Vec::new(), false));
        }
        let (where_sql, args) = where_clause(q, true);
        let sql = format!(
            "{HIT_SELECT}
             WHERE {where_sql}
             ORDER BY p.capture_date IS NULL, p.capture_date DESC, p.file_name
             LIMIT {}",
            limit + 1
        );
        let conn = self.conn();
        let mut stmt = conn.prepare(&sql)?;
        let mut hits = stmt
            .query_map(rusqlite::params_from_iter(args.iter()), hit_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        let truncated = hits.len() > limit;
        hits.truncate(limit);
        Ok((hits, truncated))
    }

    /// Avisa que el índice cambió: la próxima búsqueda por contenido recarga los vectores.
    fn index_changed(&self) {
        self.index_version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Las fotos del índice cuyo vector de `model` más se parece a `query` (normalizado):
    /// las que llegan a `min`, de más a menos parecidas, hasta `max`.
    pub fn nearest_photos(&self, model: &str, query: &[f32], min: f32, max: usize) -> DbResult<Vec<(String, f32)>> {
        let version = self.index_version.load(std::sync::atomic::Ordering::SeqCst);
        let mut cache = self.vectors.lock().unwrap();
        if !matches!(&*cache, Some(c) if c.version == version && c.model == model) {
            *cache = Some(self.load_vectors(model, version)?);
        }
        let cache = cache.as_ref().unwrap();
        if cache.dim != query.len() {
            return Ok(Vec::new());
        }
        let mut scored: Vec<(usize, f32)> = cache
            .data
            .chunks_exact(cache.dim)
            .map(|v| crate::domain::embedding::similarity(v, query))
            .enumerate()
            .filter(|(_, s)| *s >= min)
            .collect();
        // Solo se ordenan los `max` mejores.
        if scored.len() > max {
            scored.select_nth_unstable_by(max, |a, b| b.1.total_cmp(&a.1));
            scored.truncate(max);
        }
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
        Ok(scored.into_iter().map(|(i, s)| (cache.ids[i].clone(), s)).collect())
    }

    fn load_vectors(&self, model: &str, version: u64) -> DbResult<VectorCache> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT p.photo_id, m.embedding FROM search_photo p JOIN search_media m ON m.photo_id = p.photo_id
             WHERE p.embedding_model = ?1 AND m.embedding IS NOT NULL",
        )?;
        let mut cache = VectorCache { version, model: model.to_string(), dim: 0, ids: Vec::new(), data: Vec::new() };
        let rows = stmt.query_map([model], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?)))?;
        for row in rows {
            let (id, bytes) = row?;
            let Some(v) = crate::domain::embedding::vector_from_bytes(&bytes) else { continue };
            if cache.dim == 0 {
                cache.dim = v.len();
            }
            if v.len() == cache.dim && cache.dim > 0 {
                cache.ids.push(id);
                cache.data.extend_from_slice(&v);
            }
        }
        Ok(cache)
    }

    /// De `candidates` (id y parecido, en orden), las fotos que pasan los
    /// filtros de `q` sin contar su texto libre, en el mismo orden y hasta `limit`.
    pub fn content_hits(
        &self,
        q: &crate::domain::search::SearchQuery,
        candidates: &[(String, f32)],
        limit: usize,
    ) -> DbResult<Vec<SearchHit>> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let (where_sql, mut args) = where_clause(q, false);
        let ids: Vec<&str> = candidates.iter().map(|(id, _)| id.as_str()).collect();
        args.push(serde_json::to_string(&ids).unwrap_or_default());
        let sql = format!(
            "{HIT_SELECT}
             WHERE ({where_sql}) AND p.photo_id IN (SELECT value FROM json_each(?{}))",
            args.len()
        );
        let conn = self.conn();
        let mut stmt = conn.prepare(&sql)?;
        let mut found: HashMap<String, SearchHit> = stmt
            .query_map(rusqlite::params_from_iter(args.iter()), hit_from_row)?
            .map(|h| h.map(|h| (h.photo_id.clone(), h)))
            .collect::<Result<_, _>>()?;
        Ok(candidates
            .iter()
            .filter_map(|(id, score)| found.remove(id).map(|h| SearchHit { similarity: Some(*score), ..h }))
            .take(limit)
            .collect())
    }

    /// Por proyecto, cuántas fotos que pasan los filtros de `q` (sin su texto
    /// libre) no tienen vector de `model`.
    pub fn unanalyzed_photos(&self, q: &crate::domain::search::SearchQuery, model: &str) -> DbResult<Vec<UnanalyzedProject>> {
        let (where_sql, mut args) = where_clause(q, false);
        args.push(model.to_string());
        let sql = format!(
            "SELECT p.project_id, COUNT(*) FROM search_photo p JOIN search_project pr ON pr.project_id = p.project_id
             WHERE ({where_sql}) AND (p.embedding_model IS NULL OR p.embedding_model != ?{})
             GROUP BY p.project_id ORDER BY COUNT(*) DESC",
            args.len()
        );
        let conn = self.conn();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), |r| {
            Ok(UnanalyzedProject { project_id: r.get(0)?, count: r.get::<_, i64>(1)? as usize })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Miniatura del índice de una foto.
    pub fn search_thumbnail(&self, photo_id: &str) -> DbResult<Option<Vec<u8>>> {
        let conn = self.conn();
        Ok(conn
            .query_row("SELECT thumb FROM search_media WHERE photo_id = ?1", [photo_id], |r| r.get(0))
            .optional()?
            .flatten())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::search::SearchQuery;

    fn photo(id: &str, date: &str) -> SearchPhotoRow {
        SearchPhotoRow {
            photo_id: id.into(),
            file_name: format!("{id}.RAF"),
            capture_date: Some(date.into()),
            stars: 0,
            colors: ",".into(),
            tags: None,
            tags_norm: ",".into(),
            text_norm: format!("{}.raf", id.to_lowercase()),
            culled: false,
            delivered: false,
            is_pair: false,
            camera: None,
            camera_norm: String::new(),
            lens: None,
            lens_norm: String::new(),
            rotation: 0,
            thumb: Some(b"mini".to_vec()),
            embedding: None,
            embedding_model: None,
        }
    }

    fn project(id: &str, name: &str, archived: bool) -> SearchProjectRow {
        SearchProjectRow {
            project_id: id.into(),
            name: name.into(),
            description: None,
            session_date: None,
            created_at: "2025-01-01".into(),
            archived,
            device_uuid: Some("u".into()),
            device_name: Some("Samsung T7".into()),
            name_norm: crate::domain::search::normalize(name),
            text_norm: crate::domain::search::normalize(name),
            generation: 0,
            exports_signature: String::new(),
        }
    }

    /// Índice de ejemplo con dos proyectos y uno archivado.
    fn sample() -> (std::path::PathBuf, GlobalDatabase) {
        let dir = std::env::temp_dir().join(format!("lumik_search_{}_{}", std::process::id(), uuid::Uuid::new_v4().as_simple()));
        let global = GlobalDatabase::new(dir.join("lumik.db")).unwrap();
        let mut calle = photo("calle4", "2025-06-15 10:00:00");
        calle.tags = Some("calle,retrato".into());
        calle.tags_norm = ",calle,retrato,".into();
        calle.text_norm = "calle retrato calle4.raf".into();
        calle.stars = 4;
        calle.colors = ",1,".into();
        calle.culled = true;
        calle.camera = Some("X-T5".into());
        calle.camera_norm = "x-t5".into();
        let mut boda = photo("boda5", "2025-03-01 12:00:00");
        boda.tags_norm = ",calle,boda,".into();
        boda.stars = 5;
        boda.delivered = true;
        boda.is_pair = true;
        let old = photo("vieja", "2024-12-31 23:00:00");
        global.write_project_index(&project("p1", "Boda Martínez", false), &[calle, boda, old], &[]).unwrap();
        let mut archived = photo("archivada", "2025-07-01 00:00:00");
        archived.tags_norm = ",calle,".into();
        global.write_project_index(&project("p2", "Viejo", true), &[archived], &[]).unwrap();
        (dir, global)
    }

    fn ids(global: &GlobalDatabase, query: &str) -> Vec<String> {
        global.search_photos(&SearchQuery::parse(query), 100).unwrap().0.into_iter().map(|h| h.photo_id).collect()
    }

    #[test]
    fn each_operator_and_combinations() {
        let (dir, g) = sample();
        assert_eq!(ids(&g, "tag:calle"), ["calle4", "boda5"], "más reciente primero, sin archivados");
        assert_eq!(ids(&g, "tag:calle -tag:boda estrellas>=4 en:2025"), ["calle4"]);
        assert_eq!(ids(&g, "estrellas:5"), ["boda5"]);
        assert_eq!(ids(&g, "estrellas=4"), ["calle4"]);
        assert_eq!(ids(&g, "estrellas>4"), ["boda5"]);
        assert_eq!(ids(&g, "estrellas<4"), ["vieja"]);
        assert_eq!(ids(&g, "color:rojo"), ["calle4"]);
        assert_eq!(ids(&g, "camara:\"x-t5\""), ["calle4"]);
        assert_eq!(ids(&g, "en:2025-03"), ["boda5"]);
        assert_eq!(ids(&g, "en:2024"), ["vieja"]);
        assert_eq!(ids(&g, "antes:2025-01-01"), ["vieja"]);
        assert_eq!(ids(&g, "despues:2025-03-01"), ["calle4"], "sin incluir el día");
        assert_eq!(ids(&g, "es:aprobada"), ["calle4"]);
        assert_eq!(ids(&g, "es:entregada es:par"), ["boda5"]);
        assert_eq!(ids(&g, "proyecto:martinez"), ["calle4", "boda5", "vieja"]);
        assert_eq!(ids(&g, "tag:calle incluir:archivados"), ["archivada", "calle4", "boda5"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn free_text_matches_photo_or_project() {
        let (dir, g) = sample();
        assert_eq!(ids(&g, "RETRATO"), ["calle4"]);
        assert_eq!(ids(&g, "boda calle4"), ["calle4"], "todas las palabras: una en el proyecto y otra en la foto");
        assert_eq!(ids(&g, "100%"), Vec::<String>::new(), "los comodines de LIKE no se interpretan");
        assert_eq!(ids(&g, "   "), Vec::<String>::new());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn limit_reports_truncation_and_hits_carry_project_info() {
        let (dir, g) = sample();
        let (hits, truncated) = g.search_photos(&SearchQuery::parse("proyecto:boda"), 2).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(truncated);
        assert_eq!(hits[0].project_name, "Boda Martínez");
        assert_eq!(hits[0].device_name.as_deref(), Some("Samsung T7"));
        assert_eq!(hits[0].colors, [1]);
        assert_eq!(g.search_thumbnail("calle4").unwrap().as_deref(), Some(&b"mini"[..]));
        assert_eq!(g.search_thumbnail("nada").unwrap(), None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
