pub struct Event(usize);

pub trait Loop {
    fn is_done(&self) -> bool;
    /// Called once every frame until is_done returns true.
    fn tick(&mut self) -> Vec<Event>;
}

pub trait EventListener {
    /// Allows the instance to spawn new tasks in response to an event.
    fn on_event(&mut self, event: &Event);
}

pub struct SpriteInstance {
    pub x: i32,
    pub y: i32,
    pub direction: f32,
    pub speed: f32,
    pub tasks: Vec<Box<dyn Loop>>
}

impl EventListener for SpriteInstance {
    fn on_event(&mut self, event: &Event) {

    }
}

pub struct State {
    pub instances: Vec<SpriteInstance>,
    pub to_remove: Vec<usize>
}

fn test(){
    let mut state = State {
        instances: vec![],
        to_remove: vec![]
    };

    let mut events = vec![];
    for (i, inst) in state.instances.iter_mut().enumerate() {
        let mut finished = vec![];
        for (t, task) in inst.tasks.iter_mut().enumerate() {
            events.append(&mut task.tick());
            if task.is_done() {
                finished.push(t);
            }
        }

        for (i, x) in finished.iter().enumerate() {
            inst.tasks.remove(x - i);
        }
    }

    for inst in state.instances.iter_mut() {
        for event in events.iter() {
            inst.on_event(event);
        }
    }
}
