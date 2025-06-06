use super::{
    instruction::parse_instruction,
    reader::Reader,
    utils::{parse_regex, read_integer, skip_line, skip_whitespace},
    Error,
};
use crate::command::Command::{self, *};
use anyhow::{anyhow, bail, Result};

pub(crate) fn parse<R: Reader>(reader: &mut R) -> Result<Vec<Command>> {
    let mut cmds = Vec::new();
    while let Some(c) = reader.next()? {
        let cmd = match c {
            ';' => break,
            '.' => {
                cmds.push(Break);
                break;
            }
            'b' => {
                skip_whitespace(reader);
                reader.expect(';')?;
                cmds.push(Break);
                break;
            }
            'p' => Println,
            'P' => Print,
            'n' => Insert("\n".to_string()),
            't' => Insert("\t".to_string()),
            'l' => Escapeln,
            's' => parse_substitute(reader)?,
            'k' => {
                skip_whitespace(reader);
                parse_keep(reader)?
            }
            '=' => LineNumber,
            'd' => Delete,
            'z' => Reset,
            'h' => Hold,
            'g' => Get,
            'x' => Exchange,
            'j' => Joinln,
            'J' => Join,
            'e' => Eval,
            'r' => {
                skip_whitespace(reader);
                let s = read_integer(reader)?;
                let num = if s.is_empty() { 1 } else { s.parse()? };
                Readln(num)
            }
            'R' => ReadReplace,
            'q' => {
                skip_whitespace(reader);
                let s = read_integer(reader)?;
                let code = if s.is_empty() { 0 } else { s.parse()? };
                Quit(code)
            }
            ':' => parse_loop(reader)?,
            '\'' | '"' => {
                let msg = unescape(read_until(reader, c)?)?;
                Insert(msg)
            }
            '#' => {
                skip_line(reader);
                continue;
            }
            c if c.is_whitespace() => continue,
            _ => bail!(Error::Unexpected(c)),
        };
        cmds.push(cmd);

        skip_whitespace(reader);
        if let Some('}') = reader.peek()? {
            break;
        }
    }
    Ok(cmds)
}

fn parse_substitute<R: Reader>(reader: &mut R) -> Result<Command> {
    if reader.peek()? != Some('/') {
        bail!(Error::Missing('/'));
    }

    // Parse: s/src/dst/[limit]
    let Some(src) = parse_regex(reader)? else {
        bail!("empty regular expression");
    };
    let dst = read_template(reader)?;

    let mut limit = 0;
    if let Some(c) = reader.peek()? {
        if c == 'g' {
            reader.skip();
            // g is default, no need to update the limit
        } else if c.is_ascii_digit() {
            limit = read_integer(reader)?.parse()?;
        }
    }

    Ok(Substitute(src, dst, limit))
}

fn read_template<R: Reader>(reader: &mut R) -> Result<String> {
    let delim = '/';
    let mut acc = String::new();
    while let Some(c) = reader.peek()? {
        match c {
            c if c == delim => {
                reader.skip();
                return unescape(acc);
            }
            c if c.is_ascii_digit() => {
                // replace $N with ${N}
                // "$123something" string is interpreted as "${123}something" rather than "${123something}"
                acc.push('{');
                acc.push_str(&read_integer(reader)?);
                acc.push('}');
            }
            '\\' => {
                reader.skip();
                if let Some(e) = reader.next()? {
                    if e != delim {
                        acc.push(c);
                    }
                    acc.push(e);
                } else {
                    break;
                }
            }
            _ => {
                reader.skip();
                acc.push(c)
            }
        }
    }
    bail!(Error::Missing(delim))
}

fn parse_keep<R: Reader>(reader: &mut R) -> Result<Command> {
    let s = read_integer(reader)?;
    let lhs = if s.is_empty() {
        0
    } else {
        let lhs: usize = s.parse()?;
        if lhs == 0 {
            bail!("character indexes need to be >0");
        }
        lhs - 1
    };

    if !reader.next_is('-')? {
        return Ok(Keep(lhs, Some(1)));
    };

    let s = read_integer(reader)?;
    let rhs = if s.is_empty() {
        None
    } else {
        let rhs: usize = s.parse()?;
        if rhs == 0 || lhs > rhs {
            bail!(
                "invalid character index range: {} > {} in {}-{}",
                lhs + 1,
                rhs,
                lhs + 1,
                rhs,
            );
        }
        Some(rhs - lhs)
    };
    Ok(Keep(lhs, rhs))
}

fn parse_loop<R: Reader>(reader: &mut R) -> Result<Command> {
    reader.expect('{')?;
    let mut body = Vec::new();
    let mut finally = Vec::new();
    loop {
        skip_whitespace(reader);
        match reader.peek()? {
            Some('}') => {
                reader.skip();
                break;
            }
            Some(_) => parse_instruction(reader, &mut body, &mut finally)?,
            None => bail!(Error::Missing('}')),
        }
    }
    if !finally.is_empty() {
        bail!("loops cannot contain the final block ($)")
    }
    Ok(Loop(body))
}

fn read_until<R: Reader>(reader: &mut R, delim: char) -> Result<String> {
    let mut acc = String::new();
    while let Some(c) = reader.next()? {
        match c {
            c if c == delim => return Ok(acc),
            '\\' => {
                if let Some(e) = reader.next()? {
                    if e != delim {
                        acc.push(c);
                    }
                    acc.push(e);
                } else {
                    break;
                }
            }
            _ => acc.push(c),
        }
    }
    bail!(Error::Missing(delim))
}

fn unescape(s: String) -> Result<String> {
    unescape::unescape(&s).ok_or(anyhow!("unrecognized escape characters in '{}'", s))
}
