#[derive(Clone, Debug)]
pub enum SpecOption {
    Str(String),
    Int(i128),
    Float(f64),
    Bool(bool),
    List(Vec<SpecOption>),
}
