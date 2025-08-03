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

    pub fn remaining(&self) -> &'a str {
        &self.text[self.offset..]
    }

    pub fn grab(self, chars: usize) -> Result<(&'a str, Self)> {
        let offset = self.offset;
        let rem = self.remaining();
        match self.step(chars) {
            Ok(new_self) => Ok((&rem[offset..new_self.offset], new_self)),
            Err(e) => Err(e),
        }
    }

    pub fn step_mut(&mut self, chars: usize) -> Result<()> {
        if let Some((n, _)) = self.text[self.offset..].char_indices().nth(chars)
        {
            self.offset += n;
            Ok(())
        } else {
            Err(eyre!(
                "Stepped past end of string; offset={} step={chars} length={}",
                self.offset,
                self.remaining().len()
            ))
        }
    }

    pub fn step(mut self, chars: usize) -> Result<Self> {
        if let Some((n, _)) = self.text[self.offset..].char_indices().nth(chars)
        {
            self.offset += n;
            Ok(self)
        } else {
            Err(eyre!(
                "Stepped past end of string; offset={} step={chars} length={}",
                self.offset,
                self.remaining().len()
            ))
        }
    }
}
