use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

use anyhow::{bail, Context, Result};
use reqwest::Client;
use zip::ZipArchive;

use crate::model::AozoraCatalogEntry;

pub const AOZORA_CATALOG_ZIP_URL: &str =
    "https://www.aozora.gr.jp/index_pages/list_person_all_extended_utf8.zip";

pub struct AozoraCatalog {
    entries: Vec<AozoraCatalogEntry>,
}

impl AozoraCatalog {
    /// Load catalog from local cache or fetch from Aozora Bunko
    pub async fn load_or_fetch(
        client: &Client,
        cache_dir: &Path,
        force_refresh: bool,
    ) -> Result<Self> {
        tokio::fs::create_dir_all(cache_dir)
            .await
            .with_context(|| format!("failed to create cache dir {:?}", cache_dir))?;

        let cached_csv = cache_dir.join("list_person_all_extended_utf8.csv");

        if !cached_csv.exists() || force_refresh {
            println!("Fetching Aozora Bunko catalog from {}...", AOZORA_CATALOG_ZIP_URL);
            let resp = client
                .get(AOZORA_CATALOG_ZIP_URL)
                .send()
                .await
                .with_context(|| format!("failed to request catalog from {}", AOZORA_CATALOG_ZIP_URL))?;

            if !resp.status().is_success() {
                bail!("failed to download catalog: HTTP {}", resp.status());
            }

            let bytes = resp.bytes().await.context("failed to read catalog zip bytes")?;

            let mut zip = ZipArchive::new(Cursor::new(bytes))
                .context("failed to parse catalog zip archive")?;

            let mut csv_data = None;
            for i in 0..zip.len() {
                let mut file = zip.by_index(i).context("failed to inspect zip entry")?;
                if file.name().ends_with(".csv") {
                    let mut content = Vec::new();
                    file.read_to_end(&mut content)
                        .context("failed to read csv from zip")?;
                    csv_data = Some(content);
                    break;
                }
            }

            let csv_bytes = csv_data.context("no CSV file found in catalog zip archive")?;
            tokio::fs::write(&cached_csv, &csv_bytes)
                .await
                .with_context(|| format!("failed to write cached catalog to {:?}", cached_csv))?;
            println!("Cached catalog to {:?}", cached_csv);
        }

        Self::load_from_csv(&cached_csv)
    }

    pub fn load_from_csv(csv_path: &Path) -> Result<Self> {
        let file = File::open(csv_path)
            .with_context(|| format!("failed to open catalog csv {:?}", csv_path))?;
        Self::load_from_reader(file)
    }

    pub fn load_from_reader<R: Read>(reader: R) -> Result<Self> {
        let mut rdr = csv::ReaderBuilder::new()
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(reader);

        let mut entries = Vec::new();
        for result in rdr.deserialize() {
            match result {
                Ok(entry) => {
                    let e: AozoraCatalogEntry = entry;
                    entries.push(e);
                }
                Err(err) => {
                    // Skip malformed rows gracefully
                    tracing::debug!("skipping malformed catalog row: {}", err);
                }
            }
        }

        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[AozoraCatalogEntry] {
        &self.entries
    }

    /// Find an entry by exact numeric ID
    pub fn find_by_id(&self, id: u64) -> Option<&AozoraCatalogEntry> {
        self.entries
            .iter()
            .find(|e| e.id_u64() == Some(id) && e.has_text())
    }

    /// Find entries matching title (exact matches first, then partial matches)
    pub fn find_by_title(&self, title: &str) -> Vec<&AozoraCatalogEntry> {
        let trimmed = title.trim();
        let norm_query = normalize_for_search(trimmed);
        let mut exact = Vec::new();
        let mut partial = Vec::new();

        for e in &self.entries {
            if !e.has_text() {
                continue;
            }
            if e.title == trimmed {
                exact.push(e);
            } else {
                let norm_title = normalize_for_search(&e.title);
                if norm_title.contains(&norm_query) || e.title_yomi.contains(trimmed) {
                    partial.push(e);
                }
            }
        }

        if !exact.is_empty() {
            exact
        } else {
            partial
        }
    }

    /// Find entries matching author name (partial or exact, supporting variant kanji like 龍/竜)
    pub fn find_by_author(&self, author: &str) -> Vec<&AozoraCatalogEntry> {
        let trimmed = author.trim();
        let norm_query = normalize_for_search(trimmed);
        self.entries
            .iter()
            .filter(|e| {
                if !e.has_text() {
                    return false;
                }
                let norm_author = normalize_for_search(&e.author());
                norm_author.contains(&norm_query) || e.author_yomi().contains(trimmed)
            })
            .collect()
    }

    /// Pick N random entries that have downloadable text
    pub fn find_random(&self, count: usize) -> Vec<&AozoraCatalogEntry> {
        use std::collections::HashSet;

        let available: Vec<&AozoraCatalogEntry> =
            self.entries.iter().filter(|e| e.has_text() && e.is_author()).collect();

        if available.is_empty() {
            return Vec::new();
        }

        let mut chosen = Vec::new();
        let mut chosen_ids = HashSet::new();
        let total = available.len();

        // Pseudo-random sampling using timestamp seed
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as usize)
            .unwrap_or(12345);

        let target_count = count.min(total);
        let mut attempts = 0;

        while chosen.len() < target_count && attempts < total * 2 {
            attempts += 1;
            // Linear congruential generator step
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let idx = (seed >> 32) % total;
            let entry = available[idx];

            if let Some(id) = entry.id_u64()
                && chosen_ids.insert(id) {
                    chosen.push(entry);
            }
        }

        chosen
    }
}

pub fn normalize_for_search(s: &str) -> String {
    s.trim()
        .replace('龍', "竜")
        .replace('國', "国")
        .replace('體', "体")
        .replace('舊', "旧")
        .replace('邊', "辺")
        .replace([' ', '　'], "")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CSV: &str = "\u{feff}作品ID,作品名,作品名読み,ソート用読み,副題,副題読み,原題,初出,分類番号,文字遣い種別,作品著作権フラグ,公開日,最終更新日,図書カードURL,人物ID,姓,名,姓読み,名読み,姓読みソート用,名読みソート用,姓ローマ字,名ローマ字,役割フラグ,生年月日,没年月日,人物著作権フラグ,底本名1,底本出版社名1,底本初版発行年1,入力に使用した版1,校正に使用した版1,底本の親本名1,底本の親本出版社名1,底本の親本初版発行年1,底本名2,底本出版社名2,底本初版発行年2,入力に使用した版2,校正に使用した版2,底本の親本名2,底本の親本出版社名2,底本の親本初版発行年2,入力者,校正者,テキストファイルURL,テキストファイル最終更新日,テキストファイル符号化方式,テキストファイル文字集合,テキストファイル修正回数,XHTML/HTMLファイルURL,XHTML/HTMLファイル最終更新日,XHTML/HTMLファイル符号化方式,XHTML/HTMLファイル文字集合,XHTML/HTMLファイル修正回数\n\"059898\",\"ウェストミンスター寺院\",\"ウェストミンスターじいん\",\"うえすとみんすたあしいん\",\"\",\"\",\"\",\"\",\"NDC 933\",\"新字新仮名\",\"なし\",2020-04-03,2020-03-28,\"https://www.aozora.gr.jp/cards/001257/card59898.html\",\"001257\",\"アーヴィング\",\"ワシントン\",\"アーヴィング\",\"ワシントン\",\"ああういんく\",\"わしんとん\",\"Irving\",\"Washington\",\"著者\",\"1783-04-03\",\"1859-11-28\",\"なし\",\"スケッチ・ブック\",\"新潮文庫、新潮社\",\"1957（昭和32）年5月20日\",\"2000（平成12）年2月20日33刷改版\",\"2000（平成12）年2月20日33刷改版\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"えにしだ\",\"砂場清隆\",\"https://www.aozora.gr.jp/cards/001257/files/59898_ruby_70679.zip\",\"2020-03-28\",\"ShiftJIS\",\"JIS X 0208\",\"0\",\"https://www.aozora.gr.jp/cards/001257/files/59898_70731.html\",\"2020-03-28\",\"ShiftJIS\",\"JIS X 0208\",\"0\"\n\"056078\",\"駅伝馬車\",\"えきでんばしゃ\",\"えきてんはしや\",\"\",\"\",\"\",\"\",\"NDC 933\",\"旧字旧仮名\",\"なし\",2013-09-20,2014-09-16,\"https://www.aozora.gr.jp/cards/001257/card56078.html\",\"001257\",\"アーヴィング\",\"ワシントン\",\"アーヴィング\",\"ワシントン\",\"ああういんく\",\"わしんとん\",\"Irving\",\"Washington\",\"著者\",\"1783-04-03\",\"1859-11-28\",\"なし\",\"スケッチ・ブック\",\"岩波文庫、岩波書店\",\"1935（昭和10）年9月15日\",\"2010（平成22）年2月23日第31刷\",\"1992（平成4）年2月26日第30刷\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"雀\",\"小林繁雄\",\"https://www.aozora.gr.jp/cards/001257/files/56078_ruby_51155.zip\",\"2013-09-03\",\"ShiftJIS\",\"JIS X 0208\",\"0\",\"https://www.aozora.gr.jp/cards/001257/files/56078_51422.html\",\"2013-09-03\",\"ShiftJIS\",\"JIS X 0208\",\"0\"\n";

    #[test]
    fn test_load_from_reader_and_queries() {
        let catalog = AozoraCatalog::load_from_reader(SAMPLE_CSV.as_bytes()).unwrap();
        assert_eq!(catalog.entries().len(), 2);

        // Find by ID
        let book = catalog.find_by_id(59898).unwrap();
        assert_eq!(book.title, "ウェストミンスター寺院");
        assert_eq!(book.author(), "アーヴィングワシントン");

        // Find by title
        let matches = catalog.find_by_title("駅伝馬車");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].book_id, "056078");

        // Find by author
        let author_matches = catalog.find_by_author("アーヴィング");
        assert_eq!(author_matches.len(), 2);

        // Find random
        let random_picks = catalog.find_random(1);
        assert_eq!(random_picks.len(), 1);
    }
}

