pub trait ItemModel {
    fn display_name(&self) -> String;
    fn item_id(&self) -> String;
}
