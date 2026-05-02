mod albums;
mod artists;
mod artwork;
mod events;
mod likes;
mod media;
mod tracks;

pub use albums::find_or_create_album;
pub use artists::{create_artist, ensure_artist};
pub use artwork::ensure_artwork_source;
pub use events::{discard_unknown_events, ensure_event};
pub use likes::set_liked;
pub use media::{
    delete_cue_sheet_for_file, delete_tracks_for_media_file, insert_cue_sheet, upsert_media_file,
};
pub use tracks::{NewTrack, NewTrackAudioSource, insert_track, insert_track_audio_source};
