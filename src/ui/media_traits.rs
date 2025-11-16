/// Trait for items that can be played
pub trait Playable {
    fn play(&self);
    fn get_id(&self) -> String;
}

/// Trait for models that can be displayed in media cards
pub trait MediaDisplayable {
    fn primary_text(&self) -> String;
    fn secondary_text(&self) -> Option<String>;
    fn image_id(&self) -> String;
    fn supports_play(&self) -> bool;
}
