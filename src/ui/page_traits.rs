pub trait DetailPage {
    type Model;

    fn title(&self) -> String;
    fn id(&self) -> String;
    fn set_model(&self, model: &Self::Model);
}
