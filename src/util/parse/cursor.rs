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

        let rem = self.remaining();
        if chars > rem.len() {
            Err(eyre!(
                "Stepped past end of string; offset={} step={chars} length={}",
                self.offset,
                self.remaining().len()
            ))
        } else {
            self.offset +=
                rem.chars().take(chars).map(|c| c.len_utf8()).sum::<usize>();
            Ok(&self.text[offset..self.offset])
        }
    }

    pub fn step(mut self, chars: usize) -> Result<(&'a str, Self)> {
        match self.step_mut(chars) {
            Ok(s) => Ok((s, self)),
            Err(e) => Err(e),
        }
    }

    pub fn take_while(
        self,
        predicate: fn(&char) -> bool,
    ) -> Result<(&'a str, Self)> {
        let to_take = self.remaining().chars().take_while(predicate).count();
        println!("self: {self:?} to_take: {to_take}");
        self.step(to_take)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_cursor() {
        let sample_text = "12345";
        let sample_cursor = Cursor::new(sample_text);
        assert!(sample_cursor.step(4).is_ok());
    }
}
