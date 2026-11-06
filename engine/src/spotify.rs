use crate::logger::Logger;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::process::Command;

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

const NOISE_WORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "of", "in", "on", "at", "to", "for", "with", "feat", "ft",
    "prod", "by", "x", "w",
];

const AUDIO_ALTERING_TAGS: &[&str] = &[
    "remix",
    "mashup",
    "vixa",
    "instrumental",
    "karaoke",
    "slowed",
    "reverb",
    "cover",
    "tribute",
    "parodia",
    "parody",
    "1 hour",
    "10 hour",
    "loop",
    "bass boosted",
    "bassboosted",
    "bass boost",
    "reaction",
    "live",
    "acoustic",
    "sped up",
    "speed up",
    "nightcore",
    "8d audio",
    "type beat",
    "acapella",
    "a cappella",
    "snippet",
    "teaser",
    "preview",
    "leak",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTrackMeta {
    pub title: String,
    pub artist: String,
    pub artists: Vec<String>,
    pub duration_seconds: u64,
    pub thumbnail_url: Option<String>,
    pub spotify_url: Option<String>,
    pub search_query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateVideo {
    pub id: String,
    pub title: String,
    pub uploader: String,
    pub duration_seconds: u64,
}

pub struct TrackMatcher;

impl TrackMatcher {
    pub fn clean_title_for_search(title: &str) -> String {
        let mut cleaned = title.to_string();

        // 1. Remove bracketed (feat. ...), [with ...], {prod. ...}
        if let Ok(re_feat_bracket) =
            Regex::new(r"(?i)[\(\[\{]\s*(?:feat\.|ft\.|with|prod\.)\s*[^)\]\}]*[\)\]\}]")
        {
            cleaned = re_feat_bracket.replace_all(&cleaned, "").to_string();
        }

        // 2. Remove trailing "feat. ..." or "- feat. ..." not in brackets
        if let Ok(re_feat_trailing) = Regex::new(r"(?i)\s+(?:feat\.|ft\.|with|prod\.)\s+.*$") {
            cleaned = re_feat_trailing.replace_all(&cleaned, "").to_string();
        }

        // 3. Remove (Official Video), (Official Audio), (Lyrics), (Visualizer), (Teledysk), etc.
        if let Ok(re_official) = Regex::new(
            r"(?i)[\(\[\{]\s*(?:(?:official|oficjalny)\s*(?:video|audio|music video|lyric video|visualizer|teledysk)?|lyrics?|audio|video|clip|teledysk|hd|4k)\s*[\)\]\}]",
        ) {
            cleaned = re_official.replace_all(&cleaned, "").to_string();
        }

        // 4. Strip apostrophes so contractions (don't -> dont) match normalize_text
        cleaned = cleaned.replace(['\'', '’', '`'], "");

        // 5. Remove quotes, commas, brackets
        if let Ok(re_punct) = Regex::new(r#"[,\"'“”„«»]"#) {
            cleaned = re_punct.replace_all(&cleaned, " ").to_string();
        }

        // 6. Normalize whitespace
        if let Ok(re_space) = Regex::new(r"\s+") {
            cleaned = re_space.replace_all(&cleaned, " ").trim().to_string();
        }

        if cleaned.is_empty() {
            title.trim().to_string()
        } else {
            cleaned
        }
    }

    pub fn clean_artist_for_search(artist: &str) -> String {
        let mut cleaned = artist.replace(['\'', '’', '`'], "");
        if let Ok(re_punct) = Regex::new(r#"[,\"'“”„«»]"#) {
            cleaned = re_punct.replace_all(&cleaned, " ").to_string();
        }
        if let Ok(re_space) = Regex::new(r"\s+") {
            cleaned = re_space.replace_all(&cleaned, " ").trim().to_string();
        }
        if cleaned.is_empty() {
            artist.trim().to_string()
        } else {
            cleaned
        }
    }

    pub fn normalize_text(text: &str) -> String {
        let lower = text.to_lowercase().replace(['\'', '’', '`'], "");
        let mut result = String::with_capacity(lower.len());
        for c in lower.chars() {
            if c.is_alphanumeric() || c.is_whitespace() {
                result.push(c);
            } else {
                result.push(' ');
            }
        }
        if let Ok(re_space) = Regex::new(r"\s+") {
            re_space.replace_all(&result, " ").trim().to_string()
        } else {
            result.trim().to_string()
        }
    }

    pub fn extract_words(text: &str) -> Vec<String> {
        let norm = Self::normalize_text(text);
        if norm.is_empty() {
            return Vec::new();
        }
        let all_words: Vec<String> = norm
            .split_whitespace()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let non_noise: Vec<String> = all_words
            .iter()
            .filter(|w| !NOISE_WORDS.contains(&w.as_str()))
            .cloned()
            .collect();

        if !non_noise.is_empty() {
            non_noise
        } else {
            all_words
        }
    }

    pub fn parse_artist_list(artist_str: &str) -> Vec<String> {
        if artist_str.trim().is_empty() {
            return Vec::new();
        }
        if let Ok(re_split) = Regex::new(r"(?i)[,;&/]|\s+(?:feat\.|ft\.|x|with)\s+") {
            re_split
                .split(artist_str)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            vec![artist_str.trim().to_string()]
        }
    }

    pub fn build_search_query(title: &str, artists: &[String], artist: &str) -> String {
        let clean_title = Self::clean_title_for_search(title);
        let all_artists = if !artists.is_empty() {
            artists.to_vec()
        } else {
            Self::parse_artist_list(artist)
        };

        let primary_artist = all_artists
            .first()
            .map(|a| Self::clean_artist_for_search(a))
            .filter(|a| !a.is_empty())
            .unwrap_or_default();

        let second_artist = all_artists
            .get(1)
            .map(|a| Self::clean_artist_for_search(a))
            .filter(|a| !a.is_empty() && a.len() < 25);

        let artist_part = if let Some(ref sec) = second_artist {
            if sec != &primary_artist {
                format!("{} {}", primary_artist, sec)
            } else {
                primary_artist
            }
        } else {
            primary_artist
        };

        let search_terms = if !artist_part.is_empty() {
            format!("{} {}", artist_part, clean_title)
        } else {
            clean_title
        };

        format!("ytsearch10:{}", search_terms.trim())
    }

    pub fn score_candidate(
        candidate: &CandidateVideo,
        track_title: &str,
        artists: &[String],
        target_duration_seconds: u64,
    ) -> f64 {
        let clean_target_title = Self::clean_title_for_search(track_title);
        let norm_target = Self::normalize_text(&clean_target_title);
        let norm_cand_title = Self::normalize_text(&candidate.title);
        let norm_cand_uploader = Self::normalize_text(&candidate.uploader);
        let cand_full_text = format!("{} {}", norm_cand_title, norm_cand_uploader);

        let target_words = Self::extract_words(&clean_target_title);
        let cand_title_words = Self::extract_words(&candidate.title);
        let cand_uploader_words = Self::extract_words(&candidate.uploader);

        let mut score = 0.0;

        // 1. Title matching with whole-phrase and word-boundary accuracy
        let is_exact_title_match = if !norm_target.is_empty() {
            let pattern = format!(r"(?:\b|^){}(?:\b|$)", regex::escape(&norm_target));
            Regex::new(&pattern)
                .map(|re| re.is_match(&norm_cand_title))
                .unwrap_or(false)
        } else {
            false
        };

        if is_exact_title_match {
            score += 65.0;
        } else if !target_words.is_empty() {
            let matched_count = target_words
                .iter()
                .filter(|w| cand_title_words.contains(w))
                .count();

            if matched_count == target_words.len() {
                score += 50.0;
            } else if matched_count > 0 {
                let ratio = matched_count as f64 / target_words.len() as f64;
                score += ratio * 35.0;
            } else {
                score -= 100.0;
            }
        } else {
            score -= 80.0;
        }

        // 2. Artist matching (prevents matching wrong artist with similar title)
        let primary_artist = artists.first().map(|s| s.trim()).unwrap_or("");
        let mut has_artist_match = false;

        if !primary_artist.is_empty() {
            let norm_primary = Self::normalize_text(primary_artist);
            let primary_regex_match = if !norm_primary.is_empty() {
                let pattern = format!(r"(?:\b|^){}(?:\b|$)", regex::escape(&norm_primary));
                Regex::new(&pattern)
                    .map(|re| re.is_match(&cand_full_text))
                    .unwrap_or(false)
            } else {
                false
            };

            let primary_words = Self::extract_words(primary_artist);

            if primary_regex_match {
                score += 35.0;
                has_artist_match = true;
            } else if !primary_words.is_empty()
                && primary_words.iter().all(|w| {
                    cand_title_words.contains(w) || cand_uploader_words.contains(w)
                })
            {
                score += 30.0;
                has_artist_match = true;
            } else if !primary_words.is_empty()
                && primary_words.iter().any(|w| {
                    cand_title_words.contains(w) || cand_uploader_words.contains(w)
                })
            {
                score += 15.0;
                has_artist_match = true;
            }
        }

        // Check secondary artists (up to 2)
        for sec in artists.iter().skip(1).take(2) {
            let norm_sec = Self::normalize_text(sec);
            if norm_sec.len() >= 3 {
                let pattern = format!(r"(?:\b|^){}(?:\b|$)", regex::escape(&norm_sec));
                if let Ok(re) = Regex::new(&pattern) {
                    if re.is_match(&cand_full_text) {
                        score += 10.0;
                        has_artist_match = true;
                    }
                }
            }
        }

        // Severe penalty if artists were specified but zero artist matched
        if !has_artist_match && artists.iter().any(|a| !a.trim().is_empty()) {
            score -= 80.0;
        }

        // 3. Duration match (distinguishes full song from snippets or extended versions)
        if target_duration_seconds > 0 && candidate.duration_seconds > 0 {
            let diff = if candidate.duration_seconds >= target_duration_seconds {
                candidate.duration_seconds - target_duration_seconds
            } else {
                target_duration_seconds - candidate.duration_seconds
            };

            if diff <= 3 {
                score += 45.0;
            } else if diff <= 7 {
                score += 35.0;
            } else if diff <= 15 {
                score += 20.0;
            } else if diff <= 30 {
                score += 5.0;
            } else if diff <= 60 {
                score -= 20.0;
            } else {
                score -= 50.0;
            }
        }

        // 4. Modifiers penalty (remix, type beat, slowed, nightcore, snippet, etc.)
        let target_lower = track_title.to_lowercase();
        let cand_lower = candidate.title.to_lowercase();

        for tag in AUDIO_ALTERING_TAGS {
            if cand_lower.contains(tag) && !target_lower.contains(tag) {
                if ["type beat", "snippet", "teaser", "preview", "leak"].contains(tag) {
                    score -= 60.0;
                } else {
                    score -= 40.0;
                }
            }
        }

        // 5. Official / Topic bonus
        if norm_cand_title.contains("official") || norm_cand_uploader.ends_with("topic") {
            score += 10.0;
        }

        score
    }

    pub fn parse_candidates(json_output: &str) -> Vec<CandidateVideo> {
        let mut list = Vec::new();

        // yt-dlp --dump-json outputs one JSON object per line for each candidate entry
        for line in json_output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
                if let Some(cand) = Self::candidate_from_value(&val) {
                    list.push(cand);
                } else if let Some(entries) = val.get("entries").and_then(|e| e.as_array()) {
                    for e in entries {
                        if let Some(cand) = Self::candidate_from_value(e) {
                            list.push(cand);
                        }
                    }
                }
            }
        }

        // Fallback for single multi-line JSON object
        if list.is_empty() {
            if let (Some(start), Some(end)) = (json_output.find('{'), json_output.rfind('}')) {
                if start < end {
                    if let Ok(val) = serde_json::from_str::<Value>(&json_output[start..=end]) {
                        if let Some(cand) = Self::candidate_from_value(&val) {
                            list.push(cand);
                        } else if let Some(entries) = val.get("entries").and_then(|e| e.as_array()) {
                            for e in entries {
                                if let Some(cand) = Self::candidate_from_value(e) {
                                    list.push(cand);
                                }
                            }
                        }
                    }
                }
            }
        }

        list
    }

    fn candidate_from_value(val: &Value) -> Option<CandidateVideo> {
        let id = val.get("id").and_then(|v| v.as_str())?.to_string();
        let title = val.get("title").and_then(|v| v.as_str())?.to_string();
        if id.is_empty() || title.is_empty() {
            return None;
        }
        let uploader = val
            .get("uploader")
            .and_then(|v| v.as_str())
            .or_else(|| val.get("channel").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();

        let duration_seconds = val
            .get("duration")
            .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
            .unwrap_or(0);

        Some(CandidateVideo {
            id,
            title,
            uploader,
            duration_seconds,
        })
    }

    pub fn find_best_candidate(
        candidates: &[CandidateVideo],
        track_title: &str,
        artists: &[String],
        duration_seconds: u64,
        min_score: Option<f64>,
    ) -> Option<CandidateVideo> {
        if candidates.is_empty() {
            return None;
        }

        let threshold = min_score.unwrap_or(25.0);

        let mut best_cand: Option<CandidateVideo> = None;
        let mut best_score = f64::NEG_INFINITY;

        for cand in candidates {
            let s = Self::score_candidate(cand, track_title, artists, duration_seconds);
            Logger::debug(&format!(
                "[TrackMatcher] Candidate '{:?}' by '{:?}' (dur {}s) -> score: {:.1}",
                cand.title, cand.uploader, cand.duration_seconds, s
            ));
            if s > best_score {
                best_score = s;
                best_cand = Some(cand.clone());
            }
        }

        if best_score >= threshold {
            Logger::info(&format!(
                "[TrackMatcher] Selected best candidate '{:?}' with score {:.1}",
                best_cand.as_ref().map(|c| &c.title),
                best_score
            ));
            best_cand
        } else {
            Logger::warn(&format!(
                "[TrackMatcher] Highest candidate score {:.1} below threshold {:.1}",
                best_score, threshold
            ));
            None
        }
    }
}

#[derive(Clone)]
pub struct SpotifyResolver {
    cache: Arc<Mutex<HashMap<String, SpotifyTrackMeta>>>,
}

impl Default for SpotifyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SpotifyResolver {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn is_spotify_url(url: &str) -> bool {
        let trimmed = url.trim().to_lowercase();
        trimmed.contains("open.spotify.com")
            || trimmed.contains("spotify.link")
            || trimmed.starts_with("spotify:track:")
            || trimmed.starts_with("spotify:album:")
            || trimmed.starts_with("spotify:playlist:")
    }

    pub fn is_soundcloud_url(url: &str) -> bool {
        let trimmed = url.trim().to_lowercase();
        trimmed.contains("soundcloud.com")
    }

    pub fn to_embed_url(url: &str) -> Option<String> {
        let clean = url.trim();
        if let Ok(re) = Regex::new(r"open\.spotify\.com/(track|album|playlist)/([a-zA-Z0-9]+)") {
            if let Some(caps) = re.captures(clean) {
                let item_type = caps.get(1)?.as_str();
                let item_id = caps.get(2)?.as_str();
                return Some(format!(
                    "https://open.spotify.com/embed/{}/{}",
                    item_type, item_id
                ));
            }
        }
        if let Ok(re_uri) = Regex::new(r"^spotify:(track|album|playlist):([a-zA-Z0-9]+)$") {
            if let Some(caps) = re_uri.captures(clean) {
                let item_type = caps.get(1)?.as_str();
                let item_id = caps.get(2)?.as_str();
                return Some(format!(
                    "https://open.spotify.com/embed/{}/{}",
                    item_type, item_id
                ));
            }
        }
        None
    }

    pub async fn fetch_http(url: &str) -> Option<String> {
        let mut cmd = Command::new("curl.exe");
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd.args([
            "-s",
            "-L",
            "-m",
            "12",
            "-A",
            USER_AGENT,
            "-H",
            "Accept: text/html,application/xhtml+xml,application/json",
            url,
        ]);
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());

        let output = cmd.output().await.ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            None
        }
    }

    pub async fn resolve_track(&self, url: &str) -> Option<SpotifyTrackMeta> {
        // Check cache
        {
            let map = self.cache.lock().unwrap();
            if let Some(hit) = map.get(url) {
                return Some(hit.clone());
            }
        }

        let embed_url = Self::to_embed_url(url);
        let mut title = "Spotify Track".to_string();
        let mut artist = "Spotify".to_string();
        let mut artists = Vec::new();
        let mut duration_seconds = 0u64;
        let mut thumbnail: Option<String> = None;

        if let Some(ref eu) = embed_url {
            if let Some(html) = Self::fetch_http(eu).await {
                if let Some(val) = Self::extract_next_data_json(&html) {
                    if let Some(entity) = val
                        .get("props")
                        .and_then(|p| p.get("pageProps"))
                        .and_then(|pp| pp.get("state"))
                        .and_then(|s| s.get("data"))
                        .and_then(|d| d.get("entity"))
                    {
                        if let Some(t) = entity
                            .get("name")
                            .and_then(|v| v.as_str())
                            .or_else(|| entity.get("title").and_then(|v| v.as_str()))
                        {
                            title = t.to_string();
                        }

                        if let Some(artist_arr) = entity.get("artists").and_then(|v| v.as_array()) {
                            for a in artist_arr {
                                if let Some(n) = a.get("name").and_then(|v| v.as_str()) {
                                    if !n.is_empty() {
                                        artists.push(n.to_string());
                                    }
                                }
                            }
                            if !artists.is_empty() {
                                artist = artists.join(", ");
                            }
                        } else if let Some(sub) = entity.get("subtitle").and_then(|v| v.as_str()) {
                            artist = sub.to_string();
                            artists.push(sub.to_string());
                        }

                        let dur_ms = entity
                            .get("duration")
                            .and_then(|v| v.as_u64())
                            .or_else(|| entity.get("duration_ms").and_then(|v| v.as_u64()))
                            .or_else(|| entity.get("durationMs").and_then(|v| v.as_u64()))
                            .unwrap_or(0);

                        if dur_ms > 0 {
                            duration_seconds = dur_ms / 1000;
                        }

                        thumbnail = Self::extract_thumbnail(entity);
                    }
                }
            }
        }

        // Fallback to oEmbed if title is still default or thumbnail missing
        if title == "Spotify Track" || thumbnail.is_none() {
            let oembed_url = format!(
                "https://open.spotify.com/oembed?url={}",
                urlencoding_simple(url)
            );
            if let Some(json_str) = Self::fetch_http(&oembed_url).await {
                if let Ok(ojson) = serde_json::from_str::<Value>(&json_str) {
                    if let Some(otitle) = ojson.get("title").and_then(|v| v.as_str()) {
                        if title == "Spotify Track" {
                            title = otitle.to_string();
                        }
                    }
                    if thumbnail.is_none() {
                        thumbnail = ojson
                            .get("thumbnail_url")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                }
            }
        }

        if title == "Spotify Track" && artists.is_empty() {
            return None;
        }

        let query = TrackMatcher::build_search_query(&title, &artists, &artist);
        let meta = SpotifyTrackMeta {
            title,
            artist,
            artists,
            duration_seconds,
            thumbnail_url: thumbnail,
            spotify_url: Some(url.to_string()),
            search_query: query,
        };

        // Cache result (capped at 200 items)
        {
            let mut map = self.cache.lock().unwrap();
            if map.len() > 200 {
                map.clear();
            }
            map.insert(url.to_string(), meta.clone());
            if let Some(ref sp) = meta.spotify_url {
                map.insert(sp.clone(), meta.clone());
            }
        }

        Some(meta)
    }

    #[allow(dead_code)]
    pub async fn fetch_playlist_tracks(&self, url: &str) -> Vec<SpotifyTrackMeta> {
        let embed_url = match Self::to_embed_url(url) {
            Some(u) => u,
            None => return Vec::new(),
        };

        let html = match Self::fetch_http(&embed_url).await {
            Some(h) => h,
            None => return Vec::new(),
        };

        let mut tracks = Vec::new();
        let next_data = Self::extract_next_data_json(&html);
        if let Some(val) = next_data {
            if let Some(entity) = val
                .get("props")
                .and_then(|p| p.get("pageProps"))
                .and_then(|pp| pp.get("state"))
                .and_then(|s| s.get("data"))
                .and_then(|d| d.get("entity"))
            {
                let author = entity
                    .get("subtitle")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Spotify");

                let thumb = Self::extract_thumbnail(entity);

                if let Some(track_list) = entity.get("trackList").and_then(|v| v.as_array()) {
                    for (i, t_obj) in track_list.iter().enumerate() {
                        let t_title = t_obj
                            .get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&format!("Track {}", i + 1))
                            .to_string();

                        let t_artist = t_obj
                            .get("subtitle")
                            .and_then(|v| v.as_str())
                            .unwrap_or(author)
                            .to_string();

                        let dur_ms = t_obj
                            .get("duration")
                            .and_then(|v| v.as_u64())
                            .or_else(|| t_obj.get("duration_ms").and_then(|v| v.as_u64()))
                            .or_else(|| t_obj.get("durationMs").and_then(|v| v.as_u64()))
                            .unwrap_or(0);

                        let raw_id = t_obj
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                t_obj.get("uri").and_then(|v| v.as_str()).and_then(|u| {
                                    if u.starts_with("spotify:track:") {
                                        Some(u["spotify:track:".len()..].to_string())
                                    } else {
                                        None
                                    }
                                })
                            });

                        let spotify_url = raw_id
                            .as_ref()
                            .map(|id| format!("https://open.spotify.com/track/{}", id));

                        let artists_list = TrackMatcher::parse_artist_list(&t_artist);
                        let query =
                            TrackMatcher::build_search_query(&t_title, &artists_list, &t_artist);

                        let sp_track = SpotifyTrackMeta {
                            title: t_title,
                            artist: t_artist,
                            artists: artists_list,
                            duration_seconds: dur_ms / 1000,
                            thumbnail_url: thumb.clone(),
                            spotify_url,
                            search_query: query,
                        };
                        tracks.push(sp_track);
                    }
                }
            }
        }

        tracks
    }

    pub async fn find_best_youtube_match(
        ytdlp_path: &Path,
        track: &SpotifyTrackMeta,
    ) -> Option<CandidateVideo> {
        Logger::info(&format!(
            "[SpotifyResolver] Phase 1 YouTube search for '{}' - '{}' with query: {}",
            track.artist, track.title, track.search_query
        ));

        // Phase 1: Search standard query
        let phase1_candidates = Self::query_youtube_candidates(ytdlp_path, &track.search_query).await;
        if let Some(best) = TrackMatcher::find_best_candidate(
            &phase1_candidates,
            &track.title,
            &track.artists,
            track.duration_seconds,
            Some(25.0),
        ) {
            return Some(best);
        }

        // Phase 2: Exact quotes retry
        let clean_artist = track
            .artists
            .first()
            .map(|a| TrackMatcher::clean_artist_for_search(a))
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| TrackMatcher::clean_artist_for_search(&track.artist));

        let clean_title = TrackMatcher::clean_title_for_search(&track.title);

        let quote_query = format!("ytsearch10:\"{}\" \"{}\"", clean_artist, clean_title);
        Logger::info(&format!(
            "[SpotifyResolver] Phase 2 YouTube exact quote retry search: {}",
            quote_query
        ));

        let phase2_candidates = Self::query_youtube_candidates(ytdlp_path, &quote_query).await;
        TrackMatcher::find_best_candidate(
            &phase2_candidates,
            &track.title,
            &track.artists,
            track.duration_seconds,
            Some(25.0),
        )
    }

    async fn query_youtube_candidates(ytdlp_path: &Path, search_query: &str) -> Vec<CandidateVideo> {
        let mut cmd = Command::new(ytdlp_path);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd.args([
            "--dump-json",
            "--flat-playlist",
            "--no-playlist",
            "--no-warnings",
            "--ignore-errors",
            search_query,
        ]);
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());

        if let Ok(output) = cmd.output().await {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return TrackMatcher::parse_candidates(&stdout);
            }
        }
        Vec::new()
    }

    fn extract_thumbnail(entity: &Value) -> Option<String> {
        if let Some(visual) = entity.get("visualIdentity") {
            if let Some(img_arr) = visual.get("image").and_then(|v| v.as_array()) {
                if let Some(first) = img_arr.first() {
                    if let Some(u) = first.get("url").and_then(|v| v.as_str()) {
                        if !u.is_empty() {
                            return Some(u.to_string());
                        }
                    }
                }
            }
        }
        if let Some(cover_art) = entity.get("coverArt") {
            if let Some(sources) = cover_art.get("sources").and_then(|v| v.as_array()) {
                if let Some(first) = sources.first() {
                    if let Some(u) = first.get("url").and_then(|v| v.as_str()) {
                        if !u.is_empty() {
                            return Some(u.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    fn extract_next_data_json(html: &str) -> Option<Value> {
        let pattern = r#"<script id="__NEXT_DATA__"[^>]*>(.*?)</script>"#;
        let re = Regex::new(pattern).ok()?;
        let caps = re.captures(html)?;
        let json_text = caps.get(1)?.as_str();
        serde_json::from_str::<Value>(json_text).ok()
    }
}

fn urlencoding_simple(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{:02X}", b));
        }
    }
    encoded
}
