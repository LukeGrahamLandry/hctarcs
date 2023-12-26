use crate::ast::{Func, Project, Sprite, Stmt, Trigger};
use crate::scratch_schema::{Block, ScratchProject};

impl From<ScratchProject> for Project {
    fn from(value: ScratchProject) -> Self {
        Parser::new().parse(value)
    }
}

struct Parser {

}

impl Parser {
    fn parse(&mut self, value: ScratchProject) -> Project {
        let mut proj = Project { targets: vec![] };

        for target in &value.targets {
            println!("Parse Sprite {}", target.name);
            let entry = target.blocks.iter().filter(|(k, v)| v.opcode.starts_with("event_"));
            let mut functions = vec![];
            for (name, block) in entry {
                println!("Parse Func {name}");
                let start = self.parse_trigger(block);
                let mut body = vec![];
                let mut next = block.next.as_ref();
                while next.is_some() {
                    let val = target.blocks.get(next.unwrap()).unwrap();
                    body.push(self.parse_stmt(val));
                    next = val.next.as_ref();
                }
                functions.push(Func {
                    start,
                    body,
                })
            }
            proj.targets.push(Sprite {
                functions,
            })
        }

        proj
    }

    fn parse_stmt(&mut self, block: &Block) -> Stmt {
        match block.opcode.as_str() {
            _ => Stmt::UnknownOpcode(block.opcode.clone())
        }
    }

    fn parse_trigger(&mut self, block: &Block) -> Trigger {
        match block.opcode.as_str() {
            "event_whenflagclicked" => Trigger::FlagClicked,
            _ => todo!("Unknown trigger {}", block.opcode)
        }
    }

    fn new() -> Self {
        Parser {}
    }
}
