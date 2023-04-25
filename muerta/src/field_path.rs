use std::fmt::{Display, Write};

#[derive(Debug, Clone)]
pub struct FieldPath {
    pub data: [i32; 7],
    pub position: usize,
    pub finished: bool,
}

impl FieldPath {
    pub fn new() -> Self {
        Self {
            data: [-1, 0, 0, 0, 0, 0, 0],
            position: 0,
            finished: false,
        }
    }

    pub fn push_back(&mut self, value: i32) {
        self.position += 1;
        self.data[self.position] = value;
    }

    pub fn pop(&mut self, n: usize) {
        for _ in 0..n {
            self.data[self.position] = 0;
            self.position -= 1;
        }
    }
}

impl Display for FieldPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..=self.position {
            f.write_str(&self.data[i].to_string())?;
            if i < self.position {
                f.write_char('/')?;
            }
        }
        Ok(())
    }
}
