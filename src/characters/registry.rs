use crate::characters::types::Character;

pub struct Registry {
    characters: Vec<Character>,
    current_index: usize,
}

impl Registry {
    pub fn new(characters: Vec<Character>) -> Self {
        Self {
            characters,
            current_index: 0,
        }
    }

    pub fn from_builtins() -> Self {
        Self::new(crate::characters::builtins::load_builtins())
    }

    pub fn current(&self) -> Option<&Character> {
        self.characters.get(self.current_index)
    }

    pub fn next(&mut self) -> Option<&Character> {
        if self.characters.is_empty() {
            return None;
        }
        self.current_index = (self.current_index + 1) % self.characters.len();
        self.current()
    }

    pub fn prev(&mut self) -> Option<&Character> {
        if self.characters.is_empty() {
            return None;
        }
        if self.current_index == 0 {
            self.current_index = self.characters.len() - 1;
        } else {
            self.current_index -= 1;
        }
        self.current()
    }

    pub fn select_by_name(&mut self, name: &str) -> bool {
        if let Some(i) = self.characters.iter().position(|c| c.name == name || c.id == name) {
            self.current_index = i;
            return true;
        }
        false
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Character> {
        self.characters.get(index)
    }

    pub fn count(&self) -> usize {
        self.characters.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characters::types::{Character, Source, CharacterKind};

    #[test]
    fn test_registry_next_cycles() {
        let c1 = Character { id: "c1".into(), name: "c1".into(), source: Source::CowFiles, kind: CharacterKind::Grid(vec![]), height: 0, width: 0 };
        let c2 = Character { id: "c2".into(), name: "c2".into(), source: Source::CowFiles, kind: CharacterKind::Grid(vec![]), height: 0, width: 0 };
        let mut reg = Registry::new(vec![c1, c2]);
        
        assert_eq!(reg.current().unwrap().name, "c1");
        assert_eq!(reg.next().unwrap().name, "c2");
        assert_eq!(reg.next().unwrap().name, "c1");
    }

    #[test]
    fn test_registry_prev_cycles() {
        let c1 = Character { id: "c1".into(), name: "c1".into(), source: Source::CowFiles, kind: CharacterKind::Grid(vec![]), height: 0, width: 0 };
        let c2 = Character { id: "c2".into(), name: "c2".into(), source: Source::CowFiles, kind: CharacterKind::Grid(vec![]), height: 0, width: 0 };
        let mut reg = Registry::new(vec![c1, c2]);
        
        assert_eq!(reg.current().unwrap().name, "c1");
        assert_eq!(reg.prev().unwrap().name, "c2");
        assert_eq!(reg.prev().unwrap().name, "c1");
    }

    #[test]
    fn test_registry_loads_builtins() {
        let reg = Registry::from_builtins();
        assert!(reg.count() > 0);
    }
}
