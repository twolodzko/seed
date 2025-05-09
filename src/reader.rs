use crate::Error;
use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
    iter::Peekable,
    path::PathBuf,
    vec::IntoIter,
};

pub trait Reader {
    fn next(&mut self) -> Result<Option<char>, Error>;
    fn peek(&mut self) -> Result<Option<char>, Error>;
}

pub struct StringReader(Peekable<IntoIter<char>>);

impl From<String> for StringReader {
    fn from(value: String) -> Self {
        StringReader(value.chars().collect::<Vec<char>>().into_iter().peekable())
    }
}

impl Reader for StringReader {
    fn next(&mut self) -> Result<Option<char>, Error> {
        Ok(self.0.next())
    }

    fn peek(&mut self) -> Result<Option<char>, Error> {
        Ok(self.0.peek().cloned())
    }
}

pub struct FileReader {
    file: Lines<BufReader<File>>,
    chars: StringReader,
}

impl TryFrom<PathBuf> for FileReader {
    type Error = Error;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let file = BufReader::new(File::open(value).map_err(Error::Io)?).lines();
        let chars = StringReader::from(String::new());
        Ok(FileReader { file, chars })
    }
}

impl Reader for FileReader {
    fn next(&mut self) -> Result<Option<char>, Error> {
        loop {
            if let c @ Some(_) = self.chars.next()? {
                return Ok(c);
            }
            if !self.next_line()? {
                return Ok(None);
            }
        }
    }

    fn peek(&mut self) -> Result<Option<char>, Error> {
        loop {
            if let c @ Some(_) = self.chars.peek()? {
                return Ok(c);
            }
            if !self.next_line()? {
                return Ok(None);
            }
        }
    }
}

impl FileReader {
    fn next_line(&mut self) -> Result<bool, Error> {
        if let Some(res) = self.file.next() {
            let line = res.map_err(Error::Io)?;
            self.chars = StringReader::from(line);
            return Ok(true);
        }
        Ok(false)
    }
}
