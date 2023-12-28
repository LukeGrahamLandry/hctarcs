#[derive(Clone, Default, Debug)]
pub struct SpriteBase {
    pub x: f64,
    pub y: f64,
    pub direction: f64,
    pub speed: f64,
    pub pen: Pen,
    pub lines: Vec<Line>,  // TODO: run length encoding?
}

#[derive(Clone, Default, Debug)]
pub struct Line {
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub size: f64,
    pub colour: Argb,
}

#[derive(Clone, Default, Debug)]
pub struct Pen {
    pub size: f64,
    pub down: bool,
    pub colour: Argb
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Argb(pub u32);

pub trait Sprite<Msg, Globals> {
    fn receive(&mut self, sprite: &mut SpriteBase, globals: &mut Globals, msg: Msg);
}
