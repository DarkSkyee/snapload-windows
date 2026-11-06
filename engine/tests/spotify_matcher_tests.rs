use snapload_engine::downloader::DownloadManager;
use snapload_engine::spotify::{CandidateVideo, SpotifyResolver, TrackMatcher};

#[test]
fn test_clean_title_for_search() {
    // 1. Bracketed features
    assert_eq!(
        TrackMatcher::clean_title_for_search("Industry Baby (feat. Jack Harlow)"),
        "Industry Baby"
    );
    assert_eq!(
        TrackMatcher::clean_title_for_search("Save Your Tears [with Ariana Grande]"),
        "Save Your Tears"
    );
    assert_eq!(
        TrackMatcher::clean_title_for_search("Song Title {prod. Metro Boomin}"),
        "Song Title"
    );

    // 2. Trailing feat not in brackets
    assert_eq!(
        TrackMatcher::clean_title_for_search("Stay feat. Justin Bieber"),
        "Stay"
    );

    // 3. Official video, audio, teledysk tags
    assert_eq!(
        TrackMatcher::clean_title_for_search("Never Gonna Give You Up (Official Video)"),
        "Never Gonna Give You Up"
    );
    assert_eq!(
        TrackMatcher::clean_title_for_search("Blinding Lights (Official Music Video) [4K]"),
        "Blinding Lights"
    );
    assert_eq!(
        TrackMatcher::clean_title_for_search("Polski Hit (Oficjalny Teledysk)"),
        "Polski Hit"
    );
    assert_eq!(
        TrackMatcher::clean_title_for_search("Calm Down (Lyrics)"),
        "Calm Down"
    );

    // 4. Apostrophe stripping and punctuation removal
    assert_eq!(
        TrackMatcher::clean_title_for_search("Don't Stop Believin'"),
        "Dont Stop Believin"
    );
    assert_eq!(
        TrackMatcher::clean_title_for_search("“Hello, World”"),
        "Hello World"
    );
}

#[test]
fn test_clean_artist_for_search() {
    assert_eq!(TrackMatcher::clean_artist_for_search("Guns N' Roses"), "Guns N Roses");
    assert_eq!(TrackMatcher::clean_artist_for_search("Tyler, The Creator"), "Tyler The Creator");
}

#[test]
fn test_parse_artist_list() {
    let artists = TrackMatcher::parse_artist_list("Drake, 21 Savage & Travis Scott");
    assert_eq!(artists, vec!["Drake", "21 Savage", "Travis Scott"]);

    let artists_feat = TrackMatcher::parse_artist_list("Calvin Harris feat. Dua Lipa");
    assert_eq!(artists_feat, vec!["Calvin Harris", "Dua Lipa"]);
}

#[test]
fn test_build_search_query() {
    let query = TrackMatcher::build_search_query(
        "Never Gonna Give You Up (Official Video)",
        &["Rick Astley".to_string()],
        "Rick Astley",
    );
    assert_eq!(query, "ytsearch10:Rick Astley Never Gonna Give You Up");

    let query_duo = TrackMatcher::build_search_query(
        "Save Your Tears (feat. Ariana Grande)",
        &["The Weeknd".to_string(), "Ariana Grande".to_string()],
        "",
    );
    assert_eq!(query_duo, "ytsearch10:The Weeknd Ariana Grande Save Your Tears");
}

#[test]
fn test_score_candidate_original_vs_remix() {
    let orig = CandidateVideo {
        id: "orig_1".to_string(),
        title: "Rick Astley - Never Gonna Give You Up (Official Music Video)".to_string(),
        uploader: "Rick Astley".to_string(),
        duration_seconds: 213,
    };

    let remix = CandidateVideo {
        id: "remix_1".to_string(),
        title: "Rick Astley - Never Gonna Give You Up (Club Remix Bass Boosted)".to_string(),
        uploader: "BassDrop Records".to_string(),
        duration_seconds: 213,
    };

    let slowed = CandidateVideo {
        id: "slowed_1".to_string(),
        title: "Rick Astley - Never Gonna Give You Up (Slowed + Reverb)".to_string(),
        uploader: "ChillVibes".to_string(),
        duration_seconds: 260,
    };

    let artists = vec!["Rick Astley".to_string()];
    let target_title = "Never Gonna Give You Up";
    let target_dur = 214;

    let score_orig = TrackMatcher::score_candidate(&orig, target_title, &artists, target_dur);
    let score_remix = TrackMatcher::score_candidate(&remix, target_title, &artists, target_dur);
    let score_slowed = TrackMatcher::score_candidate(&slowed, target_title, &artists, target_dur);

    assert!(score_orig > 100.0, "Original track score should be high, was {}", score_orig);
    assert!(
        score_orig > score_remix + 60.0,
        "Original must decisively beat remix: orig={} vs remix={}",
        score_orig,
        score_remix
    );
    assert!(
        score_orig > score_slowed + 80.0,
        "Original must decisively beat slowed/reverb: orig={} vs slowed={}",
        score_orig,
        score_slowed
    );
}

#[test]
fn test_score_candidate_duration_tolerance() {
    let cand_exact = CandidateVideo {
        id: "1".to_string(),
        title: "Song Title".to_string(),
        uploader: "Artist".to_string(),
        duration_seconds: 200,
    };
    let cand_3s_diff = CandidateVideo {
        id: "2".to_string(),
        title: "Song Title".to_string(),
        uploader: "Artist".to_string(),
        duration_seconds: 203,
    };
    let cand_wrong_dur = CandidateVideo {
        id: "3".to_string(),
        title: "Song Title".to_string(),
        uploader: "Artist".to_string(),
        duration_seconds: 600, // 10 min loop
    };

    let artists = vec!["Artist".to_string()];
    let score_exact = TrackMatcher::score_candidate(&cand_exact, "Song Title", &artists, 200);
    let score_close = TrackMatcher::score_candidate(&cand_3s_diff, "Song Title", &artists, 200);
    let score_wrong = TrackMatcher::score_candidate(&cand_wrong_dur, "Song Title", &artists, 200);

    assert_eq!(score_exact, score_close, "Within 3s diff should yield same max duration points (+45)");
    assert!(score_exact > score_wrong + 80.0, "Drastically wrong duration must be penalized heavily");
}

#[test]
fn test_find_best_candidate_selection() {
    let candidates = vec![
        CandidateVideo {
            id: "wrong_artist".to_string(),
            title: "Someone Else - Never Gonna Give You Up (Cover)".to_string(),
            uploader: "CoverArtist".to_string(),
            duration_seconds: 213,
        },
        CandidateVideo {
            id: "nightcore".to_string(),
            title: "Rick Astley - Never Gonna Give You Up (Nightcore)".to_string(),
            uploader: "NightcoreHub".to_string(),
            duration_seconds: 180,
        },
        CandidateVideo {
            id: "official_win".to_string(),
            title: "Rick Astley - Never Gonna Give You Up (Official Music Video)".to_string(),
            uploader: "Rick Astley".to_string(),
            duration_seconds: 213,
        },
    ];

    let best = TrackMatcher::find_best_candidate(
        &candidates,
        "Never Gonna Give You Up",
        &["Rick Astley".to_string()],
        214,
        Some(25.0),
    );

    assert!(best.is_some());
    assert_eq!(best.unwrap().id, "official_win");
}

#[test]
fn test_parse_candidates_ndjson() {
    let ndjson = r#"
{"id": "vid1", "title": "Track 1", "uploader": "Artist 1", "duration": 180}
{"id": "vid2", "title": "Track 2", "channel": "Artist 2", "duration": 240}
"#;
    let list = TrackMatcher::parse_candidates(ndjson);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "vid1");
    assert_eq!(list[0].duration_seconds, 180);
    assert_eq!(list[1].id, "vid2");
    assert_eq!(list[1].uploader, "Artist 2");
}

#[test]
fn test_domain_recognition() {
    // Spotify URLs
    assert!(DownloadManager::is_spotify_url("https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT"));
    assert!(DownloadManager::is_spotify_url("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M"));
    assert!(DownloadManager::is_spotify_url("https://spotify.link/AbCdEf123"));
    assert!(DownloadManager::is_spotify_url("spotify:track:4cOdK2wGLETKBW3PvgPWqT"));
    assert!(!DownloadManager::is_spotify_url("https://youtube.com/watch?v=123"));

    // SoundCloud URLs
    assert!(DownloadManager::is_soundcloud_url("https://soundcloud.com/artist/track-name"));
    assert!(DownloadManager::is_soundcloud_url("https://m.soundcloud.com/artist/sets/playlist-name"));
    assert!(!DownloadManager::is_soundcloud_url("https://youtube.com/watch?v=123"));
    assert!(!DownloadManager::is_soundcloud_url("https://open.spotify.com/track/123"));
}

#[test]
fn test_clean_title_remaster_cleaning() {
    assert_eq!(
        TrackMatcher::clean_title_for_search("Bohemian Rhapsody - Remastered 2011"),
        "Bohemian Rhapsody"
    );
    assert_eq!(
        TrackMatcher::clean_title_for_search("In The End (2020 Remaster)"),
        "In The End"
    );
    assert_eq!(
        TrackMatcher::clean_title_for_search("Hotel California (40th Anniversary Edition)"),
        "Hotel California"
    );
}

#[test]
fn test_score_candidate_no_false_positive_penalties() {
    // Oliver Tree should not trigger penalty for "live"
    let oliver = CandidateVideo {
        id: "oliver_1".to_string(),
        title: "Oliver Tree - Hurt (Official Music Video)".to_string(),
        uploader: "Oliver Tree".to_string(),
        duration_seconds: 145,
    };
    let score_oliver = TrackMatcher::score_candidate(&oliver, "Hurt", &["Oliver Tree".to_string()], 145);
    assert!(score_oliver >= 100.0, "Oliver Tree must not be penalized for 'live' substring; score was {}", score_oliver);

    // Discovery should not trigger penalty for "cover"
    let discovery = CandidateVideo {
        id: "disc_1".to_string(),
        title: "Daft Punk - Discovery (Full Album Audio)".to_string(),
        uploader: "Daft Punk".to_string(),
        duration_seconds: 3600,
    };
    let score_disc = TrackMatcher::score_candidate(&discovery, "Discovery", &["Daft Punk".to_string()], 3600);
    assert!(score_disc >= 100.0, "Discovery must not be penalized for 'cover' substring; score was {}", score_disc);
}

#[test]
fn test_to_embed_url_conversion() {
    // Standard URL
    assert_eq!(
        SpotifyResolver::to_embed_url("https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT?si=abc123"),
        Some("https://open.spotify.com/embed/track/4cOdK2wGLETKBW3PvgPWqT".to_string())
    );
    // International URLs (Polish, German, Spanish, French, etc.)
    assert_eq!(
        SpotifyResolver::to_embed_url("https://open.spotify.com/intl-pl/track/4cOdK2wGLETKBW3PvgPWqT"),
        Some("https://open.spotify.com/embed/track/4cOdK2wGLETKBW3PvgPWqT".to_string())
    );
    assert_eq!(
        SpotifyResolver::to_embed_url("https://open.spotify.com/intl-de/album/37i9dQZF1DXcBWIGoYBM5M"),
        Some("https://open.spotify.com/embed/album/37i9dQZF1DXcBWIGoYBM5M".to_string())
    );
    assert_eq!(
        SpotifyResolver::to_embed_url("https://open.spotify.com/intl-es/playlist/37i9dQZF1DXcBWIGoYBM5M"),
        Some("https://open.spotify.com/embed/playlist/37i9dQZF1DXcBWIGoYBM5M".to_string())
    );
    // Spotify URIs
    assert_eq!(
        SpotifyResolver::to_embed_url("spotify:track:4cOdK2wGLETKBW3PvgPWqT"),
        Some("https://open.spotify.com/embed/track/4cOdK2wGLETKBW3PvgPWqT".to_string())
    );
    assert_eq!(
        SpotifyResolver::to_embed_url("spotify:episode:72iH1e6k1bNqC6yX5L3m8A"),
        Some("https://open.spotify.com/embed/episode/72iH1e6k1bNqC6yX5L3m8A".to_string())
    );
}
