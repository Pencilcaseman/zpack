use color_eyre::{Result, eyre::eyre};

#[derive(Debug, Clone, Copy)]
pub struct Cursor<'a> {
    text: &'a str,
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text, offset: 0 }
    }

    pub fn peek(&self) -> Result<char> {
        self.text.chars().next().ok_or(eyre!(
            "Stepped past end of string; offset={} length={}",
            self.offset,
            self.text.len()
        ))
    }

    pub fn remaining(&self) -> &'a str {
        &self.text[self.offset..]
    }

    pub fn step_mut(&mut self, chars: usize) -> Result<&'a str> {
        let offset = self.offset;
        if let Some((n, _)) = self.text[self.offset..].char_indices().nth(chars)
        {
            self.offset += n;
            Ok(&self.text[offset..self.offset])
        } else {
            Err(eyre!(
                "Stepped past end of string; offset={} step={chars} length={}",
                self.offset,
                self.remaining().len()
            ))
        }
    }

    pub fn step(mut self, chars: usize) -> Result<(&'a str, Self)> {
        match self.step_mut(chars) {
            Ok(s) => Ok((s, self)),
            Err(e) => Err(e),
        }
    }
}
