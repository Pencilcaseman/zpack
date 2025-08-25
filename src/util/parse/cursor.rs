#[derive(Debug, Clone, Copy)]
pub struct Cursor<'a> {
    text: &'a str,
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text, offset: 0 }
    }

    pub fn peek(&self) -> Option<char> {
        self.text.chars().next()
    }

    pub fn remaining(&self) -> &'a str {
        &self.text[self.offset..]
    }

    pub fn step_mut(&mut self, chars: usize) -> Option<&'a str> {
        let offset = self.offset;

        let rem = self.remaining();
        if chars > rem.len() {
            None
        } else {
            self.offset +=
                rem.chars().take(chars).map(|c| c.len_utf8()).sum::<usize>();
            Some(&self.text[offset..self.offset])
        }
    }

    pub fn step(mut self, chars: usize) -> Option<(&'a str, Self)> {
        match self.step_mut(chars) {
            Some(s) => Some((s, self)),
            None => None,
        }
    }

    pub fn take_while(
        self,
        predicate: fn(&char) -> bool,
    ) -> Option<(&'a str, Self)> {
        let to_take = self.remaining().chars().take_while(predicate).count();
        self.step(to_take)
    }

    pub fn take_while_non_zero(
        self,
        predicate: fn(&char) -> bool,
    ) -> Option<(&'a str, Self)> {
        let to_take = self.remaining().chars().take_while(predicate).count();
        if to_take == 0 { None } else { self.step(to_take) }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_cursor() {
        let sample_text = "12345";
        let sample_cursor = Cursor::new(sample_text);
        assert!(sample_cursor.step(4).is_some());
    }
}
