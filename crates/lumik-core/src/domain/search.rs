//! Consulta de la búsqueda de fotos (spec `photo-search`): operadores al estilo
//! Gmail y texto libre. `SearchQuery::parse` y `SearchQuery::to_query` hacen la
//! ida y vuelta entre el cuadro de búsqueda y el panel avanzado, en español o en
//! inglés. Puro: no sabe de SQL ni del índice.

use serde::{Deserialize, Serialize};

/// Idioma en que se escriben los operadores (se leen en los dos).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryLang {
    Es,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "value", rename_all = "snake_case")]
pub enum StarsFilter {
    /// Exactamente `n` estrellas.
    Exact(u8),
    /// `n` o más estrellas.
    AtLeast(u8),
    /// `n` o menos estrellas.
    AtMost(u8),
}

/// Un año o un mes de captura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Period {
    Year { year: i32 },
    Month { year: i32, month: u32 },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Palabras o frases (entre comillas) de texto libre, normalizadas.
    pub text: Vec<String>,
    /// Tags que deben estar (normalizados).
    pub tags: Vec<String>,
    /// Tags que no deben estar.
    pub not_tags: Vec<String>,
    pub stars: Option<StarsFilter>,
    /// Colores (1 rojo … 5 púrpura); basta con que la foto tenga uno.
    pub colors: Vec<u8>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub period: Option<Period>,
    /// Captura antes de esta fecha (`AAAA-MM-DD`, sin incluirla).
    pub before: Option<String>,
    /// Captura después de esta fecha (`AAAA-MM-DD`, sin incluirla).
    pub after: Option<String>,
    pub project: Option<String>,
    pub approved: bool,
    pub delivered: bool,
    pub pair: bool,
    pub include_archived: bool,
}

const COLORS_ES: [&str; 5] = ["rojo", "amarillo", "verde", "azul", "purpura"];
const COLORS_EN: [&str; 5] = ["red", "yellow", "green", "blue", "purple"];

/// Minúsculas y sin acentos, para comparar sin distinguirlos.
pub fn normalize(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

/// Separa por espacios, respetando las comillas (`camara:"X T5"`, `"frase libre"`).
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for c in input.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                current.push(c);
            }
            c if c.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn unquote(value: &str) -> &str {
    value.trim_matches('"')
}

fn quote_if_needed(value: &str) -> String {
    if value.chars().any(char::is_whitespace) {
        format!("\"{value}\"")
    } else {
        value.to_string()
    }
}

/// `AAAA-MM-DD` con mes y día plausibles.
fn valid_date(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    matches!(parts.as_slice(), [y, m, d]
        if y.len() == 4 && m.len() == 2 && d.len() == 2
            && y.parse::<i32>().is_ok()
            && m.parse::<u32>().is_ok_and(|m| (1..=12).contains(&m))
            && d.parse::<u32>().is_ok_and(|d| (1..=31).contains(&d)))
}

fn parse_period(value: &str) -> Option<Period> {
    match value.split('-').collect::<Vec<_>>().as_slice() {
        [y] if y.len() == 4 => Some(Period::Year { year: y.parse().ok()? }),
        [y, m] if y.len() == 4 && m.len() == 2 => {
            let month: u32 = m.parse().ok()?;
            (1..=12).contains(&month).then_some(Period::Month { year: y.parse().ok()?, month })
        }
        _ => None,
    }
}

fn parse_stars(value: &str) -> Option<u8> {
    value.parse::<u8>().ok().filter(|n| *n <= 5)
}

/// `>=4`, `>4`, `<=2`, `<2`, `=3`, `:3` → filtro de estrellas. `>` y `<` se
/// guardan como "o más" / "o menos" del número siguiente o anterior.
fn parse_stars_filter(rest: &str) -> Option<StarsFilter> {
    let (op, value) = [">=", "<=", ">", "<", "=", ":"]
        .iter()
        .find_map(|op| rest.strip_prefix(op).map(|v| (*op, v)))?;
    let n = parse_stars(value)?;
    match op {
        ">=" => Some(StarsFilter::AtLeast(n)),
        "<=" => Some(StarsFilter::AtMost(n)),
        ">" => (n < 5).then(|| StarsFilter::AtLeast(n + 1)),
        "<" => (n > 0).then(|| StarsFilter::AtMost(n - 1)),
        _ => Some(StarsFilter::Exact(n)),
    }
}

fn parse_color(value: &str) -> Option<u8> {
    let v = normalize(value);
    COLORS_ES
        .iter()
        .position(|c| *c == v)
        .or_else(|| COLORS_EN.iter().position(|c| *c == v))
        .map(|i| i as u8 + 1)
}

impl SearchQuery {
    /// Interpreta lo escrito en el cuadro. Un operador desconocido o con un
    /// valor inválido se toma como texto libre.
    pub fn parse(input: &str) -> Self {
        let mut q = SearchQuery::default();
        for token in tokenize(input) {
            if !q.apply_operator(&token) {
                let word = normalize(unquote(&token));
                if !word.is_empty() {
                    q.text.push(word);
                }
            }
        }
        q
    }

    /// Aplica `token` si es un operador válido; `false` si no lo es.
    fn apply_operator(&mut self, token: &str) -> bool {
        let lower = token.to_lowercase();
        for key in ["estrellas", "stars"] {
            if let Some(rest) = lower.strip_prefix(key) {
                if rest.starts_with(['>', '<', '=', ':']) {
                    let Some(filter) = parse_stars_filter(rest) else { return false };
                    self.stars = Some(filter);
                    return true;
                }
            }
        }
        let Some((key, raw)) = token.split_once(':') else { return false };
        let value = unquote(raw);
        if value.is_empty() {
            return false;
        }
        match normalize(key).as_str() {
            "tag" => self.tags.push(normalize(value)),
            "-tag" => self.not_tags.push(normalize(value)),
            "color" => match parse_color(value) {
                Some(c) => self.colors.push(c),
                None => return false,
            },
            "camara" | "camera" => self.camera = Some(value.to_string()),
            "lente" | "lens" => self.lens = Some(value.to_string()),
            "en" | "in" => match parse_period(value) {
                Some(p) => self.period = Some(p),
                None => return false,
            },
            "antes" | "before" if valid_date(value) => self.before = Some(value.to_string()),
            "despues" | "after" if valid_date(value) => self.after = Some(value.to_string()),
            "proyecto" | "project" => self.project = Some(value.to_string()),
            "es" | "is" => match normalize(value).as_str() {
                "aprobada" | "approved" => self.approved = true,
                "entregada" | "delivered" => self.delivered = true,
                "par" | "pair" => self.pair = true,
                _ => return false,
            },
            "incluir" | "include" => match normalize(value).as_str() {
                "archivados" | "archived" => self.include_archived = true,
                _ => return false,
            },
            _ => return false,
        }
        true
    }

    /// Escribe la consulta en el cuadro, en `lang`: primero los operadores y al
    /// final el texto libre (como lo arma el panel).
    pub fn to_query(&self, lang: QueryLang) -> String {
        let es = lang == QueryLang::Es;
        let pick = |a: &'static str, b: &'static str| if es { a } else { b };
        let mut parts: Vec<String> = Vec::new();
        for t in &self.tags {
            parts.push(format!("tag:{}", quote_if_needed(t)));
        }
        for t in &self.not_tags {
            parts.push(format!("-tag:{}", quote_if_needed(t)));
        }
        match self.stars {
            Some(StarsFilter::AtLeast(n)) => parts.push(format!("{}>={n}", pick("estrellas", "stars"))),
            Some(StarsFilter::AtMost(n)) => parts.push(format!("{}<={n}", pick("estrellas", "stars"))),
            Some(StarsFilter::Exact(n)) => parts.push(format!("{}={n}", pick("estrellas", "stars"))),
            None => {}
        }
        for c in &self.colors {
            let names = if es { COLORS_ES } else { COLORS_EN };
            if let Some(name) = names.get(usize::from(*c).wrapping_sub(1)) {
                parts.push(format!("color:{name}"));
            }
        }
        if let Some(v) = &self.camera {
            parts.push(format!("{}:{}", pick("camara", "camera"), quote_if_needed(v)));
        }
        if let Some(v) = &self.lens {
            parts.push(format!("{}:{}", pick("lente", "lens"), quote_if_needed(v)));
        }
        match self.period {
            Some(Period::Year { year }) => parts.push(format!("{}:{year}", pick("en", "in"))),
            Some(Period::Month { year, month }) => parts.push(format!("{}:{year}-{month:02}", pick("en", "in"))),
            None => {}
        }
        if let Some(v) = &self.after {
            parts.push(format!("{}:{v}", pick("despues", "after")));
        }
        if let Some(v) = &self.before {
            parts.push(format!("{}:{v}", pick("antes", "before")));
        }
        if let Some(v) = &self.project {
            parts.push(format!("{}:{}", pick("proyecto", "project"), quote_if_needed(v)));
        }
        if self.approved {
            parts.push(pick("es:aprobada", "is:approved").to_string());
        }
        if self.delivered {
            parts.push(pick("es:entregada", "is:delivered").to_string());
        }
        if self.pair {
            parts.push(pick("es:par", "is:pair").to_string());
        }
        if self.include_archived {
            parts.push(pick("incluir:archivados", "include:archived").to_string());
        }
        for w in &self.text {
            parts.push(quote_if_needed(w));
        }
        parts.join(" ")
    }

    /// Sin ningún criterio: no hay nada que buscar.
    pub fn is_empty(&self) -> bool {
        self == &SearchQuery::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combined_query_from_the_spec() {
        let q = SearchQuery::parse("tag:calle -tag:boda estrellas>=4 en:2025");
        assert_eq!(q.tags, ["calle"]);
        assert_eq!(q.not_tags, ["boda"]);
        assert_eq!(q.stars, Some(StarsFilter::AtLeast(4)));
        assert_eq!(q.period, Some(Period::Year { year: 2025 }));
        assert!(q.text.is_empty());
    }

    #[test]
    fn each_operator() {
        let q = SearchQuery::parse(
            "estrellas:3 color:Rojo color:purple camara:\"X-T5\" lente:\"XF 23mm\" en:2025-06 \
             despues:2025-01-01 antes:2025-12-31 proyecto:\"Boda Ana\" es:aprobada is:delivered es:par \
             incluir:archivados",
        );
        assert_eq!(q.stars, Some(StarsFilter::Exact(3)));
        assert_eq!(q.colors, [1, 5]);
        assert_eq!(q.camera.as_deref(), Some("X-T5"));
        assert_eq!(q.lens.as_deref(), Some("XF 23mm"));
        assert_eq!(q.period, Some(Period::Month { year: 2025, month: 6 }));
        assert_eq!(q.after.as_deref(), Some("2025-01-01"));
        assert_eq!(q.before.as_deref(), Some("2025-12-31"));
        assert_eq!(q.project.as_deref(), Some("Boda Ana"));
        assert!(q.approved && q.delivered && q.pair && q.include_archived);
        assert!(q.text.is_empty());
    }

    #[test]
    fn stars_comparisons() {
        let stars = |q: &str| SearchQuery::parse(q).stars;
        assert_eq!(stars("estrellas=4"), Some(StarsFilter::Exact(4)));
        assert_eq!(stars("estrellas:4"), Some(StarsFilter::Exact(4)));
        assert_eq!(stars("estrellas>4"), Some(StarsFilter::AtLeast(5)));
        assert_eq!(stars("estrellas>=4"), Some(StarsFilter::AtLeast(4)));
        assert_eq!(stars("estrellas<3"), Some(StarsFilter::AtMost(2)));
        assert_eq!(stars("estrellas<=3"), Some(StarsFilter::AtMost(3)));
        assert_eq!(stars("stars>2"), Some(StarsFilter::AtLeast(3)));
        // Imposibles o fuera de rango: texto libre.
        for q in ["estrellas>5", "estrellas<0", "estrellas=6", "estrellas>=x"] {
            assert_eq!(stars(q), None, "{q}");
            assert_eq!(SearchQuery::parse(q).text.len(), 1, "{q}");
        }
        assert_eq!(SearchQuery::parse("estrellas<=2").to_query(QueryLang::Es), "estrellas<=2");
        assert_eq!(SearchQuery::parse("estrellas:4").to_query(QueryLang::Es), "estrellas=4");
    }

    #[test]
    fn english_operators_are_read_too() {
        let q = SearchQuery::parse("stars>=5 camera:X100V lens:23mm in:2024 before:2024-06-01 after:2024-01-01 project:XV is:approved include:archived");
        assert_eq!(q.stars, Some(StarsFilter::AtLeast(5)));
        assert_eq!(q.camera.as_deref(), Some("X100V"));
        assert_eq!(q.period, Some(Period::Year { year: 2024 }));
        assert!(q.approved && q.include_archived);
    }

    #[test]
    fn free_text_is_normalized_and_quotes_make_a_phrase() {
        let q = SearchQuery::parse("Niños  \"en la PLAYA\" Atardecer");
        assert_eq!(q.text, ["ninos", "en la playa", "atardecer"]);
        assert_eq!(normalize("  Ñandú Ópera "), "nandu opera");
    }

    #[test]
    fn invalid_operators_are_free_text() {
        let q = SearchQuery::parse("estrellas>=x estrellas>=9 color:naranja en:25 antes:2025-13-01 es:rara foo:bar tag:");
        assert!(q.stars.is_none() && q.colors.is_empty() && q.period.is_none() && q.before.is_none());
        assert_eq!(q.text.len(), 8);
        assert!(q.text.contains(&"foo:bar".to_string()));
    }

    #[test]
    fn round_trip_in_both_languages() {
        let original = SearchQuery::parse(
            "tag:retrato -tag:boda estrellas>=4 color:verde camara:\"X-T5\" en:2025-06 es:aprobada es:par \
             incluir:archivados \"en la playa\" atardecer",
        );
        for lang in [QueryLang::Es, QueryLang::En] {
            let text = original.to_query(lang);
            assert_eq!(SearchQuery::parse(&text), original, "ida y vuelta en {lang:?}: {text}");
        }
        assert_eq!(
            SearchQuery::parse("tag:retrato estrellas>=4").to_query(QueryLang::Es),
            "tag:retrato estrellas>=4"
        );
        assert_eq!(
            SearchQuery::parse("es:aprobada en:2025").to_query(QueryLang::En),
            "in:2025 is:approved"
        );
    }

    #[test]
    fn empty_query() {
        assert!(SearchQuery::parse("   ").is_empty());
        assert!(!SearchQuery::parse("calle").is_empty());
    }
}
