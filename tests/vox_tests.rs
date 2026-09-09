use std::path::Path;
use vox::audio::{CustomSymphoniaDecoder, Player};
use vox::tui::theme::ThemeKind;
use vox::utils;
use vox::utils::lrc::LrcFile;
use vox::utils::text;

#[test]
fn test_96khz_playback_speed_and_specs() {
    let p = Path::new("/home/madara/Music/Songs/Mugamoodi (Original Motion Picture Soundtrack) - EP - 1877624027 - 24B-96.0kHz - ALAC/01. Vaaya Moodi Summa Iru Da.m4a");
    if !p.exists() {
        eprintln!("Skipping 96kHz test: file not found");
        return;
    }

    let dec = CustomSymphoniaDecoder::new(p).expect("Failed to initialize CustomSymphoniaDecoder");
    assert_eq!(rodio::Source::sample_rate(&dec), 96000, "Must report 96000 Hz, not 1 Hz");
    assert_eq!(rodio::Source::channels(&dec), 2, "Must report stereo (2 channels)");

    let player = Player::new().expect("Failed to create player");
    player.play(p, Some(273.48), 96000).expect("Player::play failed");

    std::thread::sleep(std::time::Duration::from_millis(1000));
    let pos = player.current_position();
    player.stop();

    println!("96kHz playback: elapsed {pos:.2}s after 1000ms real time");
    assert!(pos >= 0.5 && pos <= 1.5, "Playback position must progress in real-time (got {pos}s)");
}

#[test]
fn test_lrc_synced_vs_unsynced() {
    // 1. Synchronized LRC
    let synced_lrc = r#"
[00:12.50]Line one
[00:15.80]Line two
[00:20.10]Line three
"#;
    let parsed_synced = LrcFile::parse(synced_lrc);
    assert!(parsed_synced.is_synced, "Should detect synced timestamps");
    assert_eq!(parsed_synced.lines.len(), 3);
    assert!((parsed_synced.lines[0].time_secs - 12.5).abs() < 0.01);
    assert!((parsed_synced.lines[1].time_secs - 15.8).abs() < 0.01);

    // 2. Unsynchronized embedded lyrics
    let unsynced_lrc = r#"
First line of lyrics
Second line of lyrics
Third line of lyrics
"#;
    let parsed_unsynced = LrcFile::parse(unsynced_lrc);
    assert!(!parsed_unsynced.is_synced, "Should detect unsynced plain text lyrics");
    assert_eq!(parsed_unsynced.lines.len(), 3);
}

#[test]
fn test_formatters() {
    assert_eq!(utils::format_duration(0.0), "0:00");
    assert_eq!(utils::format_duration(65.0), "1:05");
    assert_eq!(utils::format_duration(3665.0), "1:01:05");

    assert_eq!(utils::format_bitrate(320), "320 kbps");
    assert_eq!(utils::format_sample_rate(44100), "44.1 kHz");
    assert_eq!(utils::format_sample_rate(96000), "96.0 kHz");
    assert_eq!(utils::format_sample_rate(192000), "192.0 kHz");
}

#[test]
fn test_theme_parsing() {
    assert_eq!(ThemeKind::from_str("nord"), ThemeKind::Nord);
    assert_eq!(ThemeKind::from_str("DRACULA"), ThemeKind::Dracula);
    assert_eq!(ThemeKind::from_str("tokyonight"), ThemeKind::TokyoNight);
    assert_eq!(ThemeKind::from_str("gruvbox"), ThemeKind::Gruvbox);
    assert_eq!(ThemeKind::from_str("cyberpunk"), ThemeKind::Cyberpunk);
    assert_eq!(ThemeKind::from_str("catppuccin"), ThemeKind::Catppuccin);
    assert_eq!(ThemeKind::from_str("unknown"), ThemeKind::Catppuccin);

    assert_eq!(ThemeKind::Nord.to_str(), "nord");
    assert_eq!(ThemeKind::Catppuccin.to_str(), "catppuccin");
}

#[test]
fn test_unicode_grapheme_width() {
    // Tamil cluster width should be 1 column
    assert_eq!(text::grapheme_width("கா"), 1);
    assert_eq!(text::grapheme_width("ழ்"), 1);

    // ASCII should be 1 column
    assert_eq!(text::grapheme_width("A"), 1);

    // CJK should be 2 columns
    assert_eq!(text::grapheme_width("中"), 2);

    let path = "/home/madara/Music/VeryLongFolderName/AnotherFolder/song.flac";
    let truncated = text::truncate_path_safe(path, 25);
    assert!(text::str_display_width(&truncated) <= 25);
    assert!(truncated.starts_with(".."));
}

#[test]
fn test_track_numbers_and_multiword_search() {
    let tmp_dir = std::env::temp_dir().join(format!("vox_test_db_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let db = vox::db::Database::open(&tmp_dir).expect("open db");

    let t1 = vox::metadata::Track {
        path: "/music/queen/02. Bohemian Rhapsody.flac".into(),
        title: "Bohemian Rhapsody".into(),
        artist: "Queen".into(),
        album: "A Night at the Opera".into(),
        genre: "Rock".into(),
        year: 1975,
        duration: 354.0,
        bitrate: 900,
        sample_rate: 44100,
        codec: "FLAC".into(),
        track_number: Some(2),
        disc_number: Some(1),
    };

    let t2 = vox::metadata::Track {
        path: "/music/queen/01. Death on Two Legs.flac".into(),
        title: "Death on Two Legs".into(),
        artist: "Queen".into(),
        album: "A Night at the Opera".into(),
        genre: "Rock".into(),
        year: 1975,
        duration: 223.0,
        bitrate: 920,
        sample_rate: 44100,
        codec: "FLAC".into(),
        track_number: Some(1),
        disc_number: Some(1),
    };

    db.store_track(&t1).unwrap();
    db.store_track(&t2).unwrap();

    // Verify album tracks are sorted by track_number (01 before 02)
    let album_tracks = db.tracks_by_album("A Night at the Opera").unwrap();
    assert_eq!(album_tracks.len(), 2);
    assert_eq!(album_tracks[0].track_number, Some(1));
    assert_eq!(album_tracks[0].title, "Death on Two Legs");
    assert_eq!(album_tracks[1].track_number, Some(2));
    assert_eq!(album_tracks[1].title, "Bohemian Rhapsody");

    // Verify multi-word search matches across artist + title
    let results = db.search_tracks("queen rhapsody").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Bohemian Rhapsody");

    let results_case = db.search_tracks("NIGHT BOHEMIAN").unwrap();
    assert_eq!(results_case.len(), 1);
    assert_eq!(results_case[0].title, "Bohemian Rhapsody");

    let results_artist = db.search_tracks("queen opera").unwrap();
    assert_eq!(results_artist.len(), 2);

    drop(db);
    let _ = std::fs::remove_dir_all(&tmp_dir);
}
