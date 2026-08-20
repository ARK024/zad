use parking_lot::RwLock;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub index: usize,
    pub preview: String,
    pub narrator: String,
    pub chapter: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PageAyahs {
    #[serde(rename = "surahTitle")]
    pub surah_title: String,
    #[serde(rename = "ayahTextHtml")]
    pub ayah_text_html: String,
    #[serde(rename = "firstAyahHtml")]
    pub first_ayah_html: String,
}

#[derive(Default)]
struct DataInner {
    hadiths: Vec<Value>,
    chapter_map: HashMap<i64, String>,
    search_index: Vec<SearchEntry>,
    quran: Vec<Value>,
}

struct SearchEntry {
    index: usize,
    haystack: String,
    preview: String,
    narrator: String,
    chapter: String,
}

#[derive(Clone, Default)]
pub struct DataLoader {
    inner: Arc<RwLock<DataInner>>,
}

impl DataLoader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads `Riyadh_AlSaliheen_V2.json`. Tries `data_dir` on disk first,
    /// falls back to the compile-time embedded copy.
    pub fn load_hadith_data(&self, data_dir: &Path) -> bool {
        let external_path = data_dir.join("Riyadh_AlSaliheen_V2.json");
        let raw: std::borrow::Cow<'_, str> = if external_path.exists() {
            match std::fs::read_to_string(&external_path) {
                Ok(s) => {
                    log::info!("Loaded hadith data from disk: {:?}", external_path);
                    std::borrow::Cow::Owned(s)
                }
                Err(e) => {
                    log::warn!("Failed to read {:?}: {}, falling back to embedded data", external_path, e);
                    std::borrow::Cow::Borrowed(include_str!("../../data/Riyadh_AlSaliheen_V2.json"))
                }
            }
        } else {
            log::debug!("External hadith file not found at {:?}, using embedded data", external_path);
            std::borrow::Cow::Borrowed(include_str!("../../data/Riyadh_AlSaliheen_V2.json"))
        };
        
        let parsed: Value = match serde_json::from_str(&raw) {
            Ok(v) => {
                log::debug!("Successfully parsed hadith JSON ({} bytes)", raw.len());
                v
            }
            Err(e) => {
                log::error!("Failed to parse hadith JSON: {}", e);
                log::error!("JSON file size: {} bytes", raw.len());
                return false;
            }
        };

        let mut inner = self.inner.write();
        inner.chapter_map.clear();
        inner.hadiths.clear();
        inner.search_index.clear();

        if parsed.is_array() {
            inner.hadiths = parsed.as_array().cloned().unwrap_or_default();
            log::info!("Loaded {} hadiths from array format", inner.hadiths.len());
        } else {
            if let Some(chs) = parsed.get("chapters").and_then(|v| v.as_array()) {
                for ch in chs {
                    if let Some(id) = ch.get("id").and_then(|v| v.as_i64()) {
                        let name = ch
                            .get("arabic")
                            .and_then(|v| v.as_str())
                            .or_else(|| ch.get("chapter").and_then(|v| v.as_str()))
                            .or_else(|| ch.get("name").and_then(|v| v.as_str()))
                            .unwrap_or("")
                            .to_string();
                        inner.chapter_map.insert(id, name);
                    }
                }
                log::debug!("Loaded {} chapters", inner.chapter_map.len());
            }
            let mut hadiths: Vec<Value> = parsed
                .get("hadiths")
                .or_else(|| parsed.get("hadithsData"))
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();
            
            // Sort hadiths by chapter and id
            hadiths.sort_by(|a, b| {
                let ac = a.get("chapterId").and_then(|v| v.as_i64()).unwrap_or(0);
                let bc = b.get("chapterId").and_then(|v| v.as_i64()).unwrap_or(0);
                if ac != bc {
                    ac.cmp(&bc)
                } else {
                    let ai = a.get("idInBook").and_then(|v| v.as_i64()).unwrap_or(0);
                    let bi = b.get("idInBook").and_then(|v| v.as_i64()).unwrap_or(0);
                    ai.cmp(&bi)
                }
            });
            inner.hadiths = hadiths;
            log::info!("Loaded {} hadiths from object format", inner.hadiths.len());
        }

        let has_data = !inner.hadiths.is_empty();
        if has_data {
            log::info!("Hadith data loaded successfully: {} hadiths", inner.hadiths.len());
        } else {
            log::warn!("No hadiths found in data file");
        }
        has_data
    }

    pub fn load_quran_data(&self, data_dir: &Path) {
        let external_path = data_dir.join("quran.json");
        let raw: std::borrow::Cow<'_, str> = if external_path.exists() {
            match std::fs::read_to_string(&external_path) {
                Ok(s) => {
                    log::info!("Loaded quran data from disk: {:?}", external_path);
                    std::borrow::Cow::Owned(s)
                }
                Err(e) => {
                    log::warn!("Failed to read {:?}: {}, falling back to embedded data", external_path, e);
                    std::borrow::Cow::Borrowed(include_str!("../../data/quran.json"))
                }
            }
        } else {
            log::debug!("External quran file not found at {:?}, using embedded data", external_path);
            std::borrow::Cow::Borrowed(include_str!("../../data/quran.json"))
        };
        
        if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
            if let Some(arr) = parsed.as_array().cloned() {
                log::info!("Loaded {} ayahs from quran.json", arr.len());
                self.inner.write().quran = arr;
            } else {
                log::error!("Quran data is not an array format");
            }
        } else {
            log::error!("Failed to parse quran JSON: {}", raw.len());
        }
    }

    pub fn build_search_index(&self) {
        let mut inner = self.inner.write();
        let chapter_map = inner.chapter_map.clone();
        let hadiths = inner.hadiths.clone();
        inner.search_index = hadiths
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let text = h
                    .get("arabic")
                    .or_else(|| h.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let narrator = h.get("narrator").and_then(|v| v.as_str()).unwrap_or("");
                let chapter = h
                    .get("chapterId")
                    .and_then(|v| v.as_i64())
                    .and_then(|id| chapter_map.get(&id).cloned())
                    .unwrap_or_default();
                let haystack = format!("{} {}", text, narrator).to_lowercase();
                let preview: String = text.chars().take(100).collect();
                SearchEntry {
                    index: i,
                    haystack,
                    preview,
                    narrator: narrator.to_string(),
                    chapter,
                }
            })
            .collect();
    }

    pub fn hadiths_len(&self) -> usize {
        self.inner.read().hadiths.len()
    }

    #[cfg(test)]
    pub fn get_hadith(&self, index: usize) -> Option<Value> {
        self.inner.read().hadiths.get(index).cloned()
    }

    #[cfg(test)]
    pub fn chapter_for(&self, chapter_id: i64) -> Option<String> {
        self.inner.read().chapter_map.get(&chapter_id).cloned()
    }

    pub fn search_hadiths(&self, query: &str) -> Vec<SearchHit> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        let q = query.to_lowercase();
        let inner = self.inner.read();
        if inner.search_index.is_empty() {
            return Vec::new();
        }
        inner
            .search_index
            .iter()
            .filter(|e| e.haystack.contains(&q))
            .take(30)
            .map(|e| SearchHit {
                index: e.index,
                preview: e.preview.clone(),
                narrator: e.narrator.clone(),
                chapter: e.chapter.clone(),
            })
            .collect()
    }

    /// Mirrors `getPageAyahs(pageNumber)` in src/main/data-loader.js.
    pub fn get_page_ayahs(&self, page_number: i64) -> Option<PageAyahs> {
        let inner = self.inner.read();
        let page_ayahs: Vec<&Value> = inner
            .quran
            .iter()
            .filter(|a| a.get("page").and_then(|v| v.as_i64()) == Some(page_number))
            .collect();
        if page_ayahs.is_empty() {
            return None;
        }

        let mut surah_names: Vec<String> = Vec::new();
        for a in &page_ayahs {
            let n = a
                .get("sura_name_ar")
                .or_else(|| a.get("surah_name_ar"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !n.is_empty() && !surah_names.iter().any(|x| x == n) {
                surah_names.push(n.to_string());
            }
        }
        let surah_title = surah_names.join(" - ");

        let ayah_text_html = page_ayahs
            .iter()
            .map(|a| {
                a.get("aya_text")
                    .or_else(|| a.get("text"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ");

        let first_ayah_html = page_ayahs
            .first()
            .and_then(|a| {
                a.get("aya_text")
                    .or_else(|| a.get("text"))
                    .and_then(|v| v.as_str())
            })
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        Some(PageAyahs {
            surah_title,
            ayah_text_html,
            first_ayah_html,
        })
    }

    /// Mirrors the renderer payload that `windows.getCurrent` builds in JS.
    pub fn build_widget_payload(
        &self,
        index: usize,
        is_review: bool,
        cfg: &Value,
    ) -> Option<Value> {
        let inner = self.inner.read();
        if inner.hadiths.is_empty() {
            return None;
        }
        let total = inner.hadiths.len();
        let idx = index.min(total.saturating_sub(1));
        let h = &inner.hadiths[idx];
        let chapter = h
            .get("chapterId")
            .and_then(|v| v.as_i64())
            .and_then(|id| inner.chapter_map.get(&id).cloned())
            .or_else(|| h.get("chapter").and_then(|v| v.as_str()).map(String::from))
            .unwrap_or_default();
        Some(json!({
            "index": idx,
            "isReview": is_review,
            "total": total,
            "text": h.get("arabic").or_else(|| h.get("text")).and_then(|v| v.as_str()).unwrap_or(""),
            "sanad": h.get("sanad").and_then(|v| v.as_str()).unwrap_or(""),
            "matn": h.get("matn").and_then(|v| v.as_str()).unwrap_or(""),
            "takhrij": h.get("takhrij").and_then(|v| v.as_str()).unwrap_or(""),
            "sharh": h.get("sharh").and_then(|v| v.as_str()).unwrap_or(""),
            "narrator": h.get("narrator").and_then(|v| v.as_str()).unwrap_or(""),
            "chapter": chapter,
            "fontSize": cfg.get("fontSize").cloned().unwrap_or(json!(22)),
            "fontFamily": cfg.get("fontFamily").cloned().unwrap_or(json!("'QuranFont', 'Traditional Arabic'")),
            "cSanad": cfg.get("cSanad").cloned().unwrap_or(json!("#5d7a69")),
            "cMatn": cfg.get("cMatn").cloned().unwrap_or(json!("#182820")),
            "cTakhrij": cfg.get("cTakhrij").cloned().unwrap_or(json!("#1a9850")),
            "cSharh": cfg.get("cSharh").cloned().unwrap_or(json!("#b35900")),
            "theme": cfg.get("theme").cloned().unwrap_or(json!("light")),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn write_temp_data(json: &str, name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("zad-rs-test-{}", uuid_like()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = std::fs::File::create(dir.join(name)).unwrap();
        f.write_all(json.as_bytes()).unwrap();
        dir
    }

    fn uuid_like() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        format!(
            "{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    }

    #[test]
    fn loads_object_form_with_chapters() {
        let dir = write_temp_data(
            r#"{
              "chapters": [{"id": 1, "arabic": "باب"}],
              "hadiths": [
                {"arabic": "abc", "narrator": "n1", "chapterId": 1, "idInBook": 2},
                {"arabic": "xyz", "narrator": "n2", "chapterId": 1, "idInBook": 1}
              ]
            }"#,
            "Riyadh_AlSaliheen_V2.json",
        );
        let dl = DataLoader::new();
        assert!(dl.load_hadith_data(&dir));
        assert_eq!(dl.hadiths_len(), 2);
        // Sorted by idInBook within chapter
        assert_eq!(
            dl.get_hadith(0).unwrap().get("arabic").unwrap().as_str().unwrap(),
            "xyz"
        );
        assert_eq!(dl.chapter_for(1).unwrap(), "باب");
    }

    #[test]
    fn search_caps_at_30_and_filters_case_insensitive() {
        let dir = write_temp_data(
            &serde_json::to_string(&json!({
                "chapters": [],
                "hadiths": (0..40).map(|i| json!({"arabic": format!("test hadith {}", i), "narrator": "n"})).collect::<Vec<_>>()
            })).unwrap(),
            "Riyadh_AlSaliheen_V2.json"
        );
        let dl = DataLoader::new();
        assert!(dl.load_hadith_data(&dir));
        dl.build_search_index();
        let hits = dl.search_hadiths("TEST");
        assert_eq!(hits.len(), 30);
    }

    #[test]
    fn page_ayahs_concat_text_and_dedup_surah_names() {
        let dir = write_temp_data(
            r#"[
                {"page": 1, "sura_name_ar": "الفاتحة", "aya_text": "ayah1 "},
                {"page": 1, "sura_name_ar": "الفاتحة", "aya_text": " ayah2"},
                {"page": 2, "sura_name_ar": "البقرة", "aya_text": "ayah3"}
            ]"#,
            "quran.json",
        );
        let dl = DataLoader::new();
        dl.load_quran_data(&dir);
        let p = dl.get_page_ayahs(1).unwrap();
        assert_eq!(p.surah_title, "الفاتحة");
        assert_eq!(p.ayah_text_html, "ayah1 ayah2");
        assert_eq!(p.first_ayah_html, "ayah1");
        assert!(dl.get_page_ayahs(99).is_none());
    }
}
