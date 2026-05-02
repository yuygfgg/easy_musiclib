mod artist_merge;
mod catalog;
mod library_write;
mod refs;
mod relations;
mod search_index;

#[cfg(test)]
mod tests;

pub use artist_merge::{
    add_artist_alias, auto_merge, import_alias_csv, merge_artists, repair_event_dates_and_artwork,
};
pub use catalog::{
    fetch_album_detail, fetch_album_summary, fetch_artist_detail, fetch_artist_summary,
    fetch_event_detail, fetch_event_summary, fetch_track_detail, list_albums, list_artists,
    list_events, list_tracks, search,
};
pub use library_write::{
    NewTrack, NewTrackAudioSource, create_artist, delete_cue_sheet_for_file,
    delete_tracks_for_media_file, discard_unknown_events, ensure_artist, ensure_artwork_source,
    ensure_event, find_or_create_album, insert_cue_sheet, insert_track, insert_track_audio_source,
    set_liked, upsert_media_file,
};
pub use refs::{entity_ref, is_unknown_event_name, now_ms, resolve_id};
pub use relations::{rebuild_relations, relation_graph};
pub use search_index::{
    refresh_album_search, refresh_artist_search, refresh_event_search, refresh_track_search,
};
