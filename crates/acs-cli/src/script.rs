// SPDX-License-Identifier: GPL-3.0-or-later

//! The playthrough script of `acs play` (`docs/play.md`): a header of
//! `player N key=value ...` lines, then one step a line, `#` comments.

use acs_core::model::{Adventure, MagicDefense, ThingId, ThingKind};
use acs_engine::{Dir, PlayerSetup, NAME_MAX};

/// Who a step is for: a player index (0-based) or every player in turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sel {
    One(usize),
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    Ge,
    Le,
    Gt,
    Lt,
    Eq,
}

impl Cmp {
    pub fn holds(self, a: u8, b: u8) -> bool {
        match self {
            Cmp::Ge => a >= b,
            Cmp::Le => a <= b,
            Cmp::Gt => a > b,
            Cmp::Lt => a < b,
            Cmp::Eq => a == b,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Cmp::Ge => ">=",
            Cmp::Le => "<=",
            Cmp::Gt => ">",
            Cmp::Lt => "<",
            Cmp::Eq => "==",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expect {
    /// Region, room, x, y; region 0 room 0 is the world map.
    At(u8, u8, u8, u8),
    Owns(ThingId, bool),
    Message(u16),
    Alive,
    Dead,
    Life(Cmp, u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    As(Sel),
    Walk(u8, u8),
    Door(u8, u8),
    Goto {
        region: u8,
        room: u8,
        at: Option<(u8, u8)>,
    },
    Take(u8, u8),
    Bump(u8, u8),
    Attack(u8, u8, Option<u32>),
    Use(ThingId, Option<(u8, u8)>),
    Cast(ThingId, Option<(u8, u8)>),
    Drop(ThingId, Option<Dir>),
    Ready(ThingId),
    Wait(u32),
    Idle(u32),
    Buy(ThingId),
    Sell(ThingId),
    Leave,
    Save,
    Retire(bool),
    Restore,
    Expect(Expect),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// 1-based line in the script file.
    pub line: usize,
    pub text: String,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct Script {
    /// Player setups by 0-based index, as the header gave them.
    pub players: Vec<(usize, PlayerSetup)>,
    pub steps: Vec<Step>,
}

/// The default player: a sturdy test character, not a rolled one.
pub fn sturdy(index: usize) -> PlayerSetup {
    PlayerSetup {
        name: format!("PLAYER {}", index + 1),
        speed: 7,
        strength: 15,
        constitution: 20,
        life_force: 20,
        dexterity: 15,
        wisdom: 15,
        size: 12,
        dodge: 40,
        parry: 40,
        armor_skill: 40,
        melee: 40,
        missile: 30,
        ..PlayerSetup::default()
    }
}

/// Splits a line into words; `"..."` keeps spaces inside one word (and
/// may sit after `key=`).
fn words(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if quoted {
        return Err("unclosed quote".into());
    }
    if any {
        out.push(cur);
    }
    Ok(out)
}

fn num<T: std::str::FromStr>(s: &str, what: &str) -> Result<T, String> {
    s.parse()
        .map_err(|_| format!("{what} should be a number, not \"{s}\""))
}

/// A master thing by number or by name (case-insensitive).
pub fn thing(adv: &Adventure, words: &[String]) -> Result<ThingId, String> {
    if words.is_empty() {
        return Err("a thing is missing".into());
    }
    let text = words.join(" ");
    if let Ok(n) = text.parse::<ThingId>() {
        return match adv.things.get((n as usize).wrapping_sub(1)) {
            Some(_) if n >= 1 => Ok(n),
            _ => Err(format!("there is no thing {n} (1..={})", adv.things.len())),
        };
    }
    let want = text.trim().to_ascii_uppercase();
    adv.things
        .iter()
        .filter(|t| !matches!(t.kind, ThingKind::Undefined))
        .find(|t| t.name.trim().to_ascii_uppercase() == want)
        .map(|t| t.number)
        .ok_or_else(|| format!("no thing is named \"{text}\""))
}

fn dir(s: &str) -> Option<Dir> {
    Some(match s.to_ascii_lowercase().as_str() {
        "n" => Dir::N,
        "ne" => Dir::NE,
        "e" => Dir::E,
        "se" => Dir::SE,
        "s" => Dir::S,
        "sw" => Dir::SW,
        "w" => Dir::W,
        "nw" => Dir::NW,
        _ => return None,
    })
}

fn xy(w: &[String]) -> Result<(u8, u8), String> {
    match w {
        [x, y] => Ok((num(x, "X")?, num(y, "Y")?)),
        _ => Err("expected X Y".into()),
    }
}

/// THING [X Y]: the last two words are the square when both are numbers
/// and a thing is left before them.
fn thing_at(adv: &Adventure, w: &[String]) -> Result<(ThingId, Option<(u8, u8)>), String> {
    if w.len() >= 3 {
        let (a, b) = (&w[w.len() - 2], &w[w.len() - 1]);
        if let (Ok(x), Ok(y)) = (a.parse::<u8>(), b.parse::<u8>()) {
            return Ok((thing(adv, &w[..w.len() - 2])?, Some((x, y))));
        }
    }
    Ok((thing(adv, w)?, None))
}

fn count(w: &[String]) -> Result<u32, String> {
    match w {
        [] => Ok(1),
        [n] => num(n, "N"),
        _ => Err("expected at most one count".into()),
    }
}

fn setup(adv: &Adventure, w: &[String]) -> Result<(usize, PlayerSetup), String> {
    let n: usize = num(w.first().ok_or("player needs a number")?, "player number")?;
    if !(1..=acs_engine::MAX_PLAYERS).contains(&n) {
        return Err(format!(
            "player number {n} is not 1..={}",
            acs_engine::MAX_PLAYERS
        ));
    }
    let mut s = sturdy(n - 1);
    let mut life_given = false;
    for kv in &w[1..] {
        let (k, v) = kv
            .split_once('=')
            .ok_or_else(|| format!("\"{kv}\" is not key=value"))?;
        let k = k.to_ascii_lowercase();
        let b = |v: &str| num::<u8>(v, &k);
        match k.as_str() {
            "name" => {
                if v.chars().count() > NAME_MAX {
                    return Err(format!("name \"{v}\" is longer than {NAME_MAX}"));
                }
                s.name = v.to_string();
            }
            "speed" => s.speed = b(v)?,
            "strength" => s.strength = b(v)?,
            "constitution" => s.constitution = b(v)?,
            "life_force" | "life" => {
                s.life_force = b(v)?;
                life_given = true;
            }
            "dexterity" => s.dexterity = b(v)?,
            "wisdom" => s.wisdom = b(v)?,
            "power" => s.power = Some(b(v)?),
            "size" => s.size = b(v)?,
            "dodge" => s.dodge = b(v)?,
            "parry" => s.parry = b(v)?,
            "armor_skill" => s.armor_skill = b(v)?,
            "melee" => s.melee = b(v)?,
            "missile" => s.missile = b(v)?,
            "magic_defense" => s.magic_defense = MagicDefense::from_code(b(v)?),
            "gold" => s.gold = num(v, "gold")?,
            "possessions" => {
                s.possessions = v
                    .split(',')
                    .filter(|t| !t.trim().is_empty())
                    .map(|t| thing(adv, &[t.trim().to_string()]))
                    .collect::<Result<_, _>>()?
            }
            _ => return Err(format!("unknown player key \"{k}\"")),
        }
    }
    // A sturdy character starts at its Constitution unless told otherwise.
    if !life_given {
        s.life_force = s.constitution;
    }
    Ok((n - 1, s))
}

fn step(adv: &Adventure, w: &[String]) -> Result<Action, String> {
    let (verb, rest) = (w[0].to_ascii_lowercase(), &w[1..]);
    Ok(match verb.as_str() {
        "as" => match rest {
            [s] if s.eq_ignore_ascii_case("both") => Action::As(Sel::Both),
            [n] => {
                let n: usize = num(n, "player")?;
                if !(1..=acs_engine::MAX_PLAYERS).contains(&n) {
                    return Err(format!("player {n} is not 1..={}", acs_engine::MAX_PLAYERS));
                }
                Action::As(Sel::One(n - 1))
            }
            _ => return Err("as 1|2|..|both".into()),
        },
        "walk" => {
            let (x, y) = xy(rest)?;
            Action::Walk(x, y)
        }
        "door" => {
            let (x, y) = xy(rest)?;
            Action::Door(x, y)
        }
        "take" => {
            let (x, y) = xy(rest)?;
            Action::Take(x, y)
        }
        "bump" => {
            let (x, y) = xy(rest)?;
            Action::Bump(x, y)
        }
        "goto" => match rest {
            [r, m] => Action::Goto {
                region: num(r, "REGION")?,
                room: num(m, "ROOM")?,
                at: None,
            },
            [r, m, x, y] => Action::Goto {
                region: num(r, "REGION")?,
                room: num(m, "ROOM")?,
                at: Some((num(x, "X")?, num(y, "Y")?)),
            },
            _ => return Err("goto REGION ROOM [X Y]".into()),
        },
        "attack" => match rest {
            [x, y] => Action::Attack(num(x, "X")?, num(y, "Y")?, None),
            [x, y, n] => Action::Attack(num(x, "X")?, num(y, "Y")?, Some(num(n, "N")?)),
            _ => return Err("attack X Y [N]".into()),
        },
        "use" => {
            let (t, at) = thing_at(adv, rest)?;
            Action::Use(t, at)
        }
        "cast" => {
            let (t, at) = thing_at(adv, rest)?;
            Action::Cast(t, at)
        }
        "drop" => match rest.split_last() {
            Some((d, head)) if !head.is_empty() && dir(d).is_some() => {
                Action::Drop(thing(adv, head)?, dir(d))
            }
            _ => Action::Drop(thing(adv, rest)?, None),
        },
        "ready" => Action::Ready(thing(adv, rest)?),
        "buy" => Action::Buy(thing(adv, rest)?),
        "sell" => Action::Sell(thing(adv, rest)?),
        "wait" => Action::Wait(count(rest)?),
        "idle" => Action::Idle(count(rest)?),
        "leave" if rest.is_empty() => Action::Leave,
        "save" if rest.is_empty() => Action::Save,
        "restore" if rest.is_empty() => Action::Restore,
        "retire" => match rest {
            [s] if s.eq_ignore_ascii_case("save") => Action::Retire(true),
            [s] if s.eq_ignore_ascii_case("destroy") => Action::Retire(false),
            _ => return Err("retire save|destroy".into()),
        },
        "expect" => Action::Expect(expect(adv, rest)?),
        "player" => return Err("player lines go in the header, before the first step".into()),
        _ => return Err(format!("unknown step \"{}\"", w[0])),
    })
}

fn expect(adv: &Adventure, w: &[String]) -> Result<Expect, String> {
    let verb = w.first().map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    let rest = w.get(1..).unwrap_or_default();
    Ok(match verb.as_str() {
        "at" => match rest {
            [r, m, x, y] => Expect::At(num(r, "REGION")?, num(m, "ROOM")?, num(x, "X")?, num(y, "Y")?),
            _ => return Err("expect at REGION ROOM X Y".into()),
        },
        "owns" => Expect::Owns(thing(adv, rest)?, true),
        "not" if rest.first().is_some_and(|s| s.eq_ignore_ascii_case("owns")) => {
            Expect::Owns(thing(adv, &rest[1..])?, false)
        }
        "message" => match rest {
            [n] => Expect::Message(num(n, "message number")?),
            _ => return Err("expect message N".into()),
        },
        "alive" if rest.is_empty() => Expect::Alive,
        "dead" if rest.is_empty() => Expect::Dead,
        "life" => match rest {
            [op, n] => {
                let op = match op.as_str() {
                    ">=" => Cmp::Ge,
                    "<=" => Cmp::Le,
                    ">" => Cmp::Gt,
                    "<" => Cmp::Lt,
                    "=" | "==" => Cmp::Eq,
                    _ => return Err(format!("unknown comparison \"{op}\"")),
                };
                Expect::Life(op, num(n, "N")?)
            }
            _ => return Err("expect life >= N".into()),
        },
        _ => return Err("expect at|owns|not owns|message|alive|dead|life ...".into()),
    })
}

/// Parses a script; thing names are looked up in `adv`.
pub fn parse(text: &str, adv: &Adventure) -> Result<Script, String> {
    let mut players = Vec::new();
    let mut steps: Vec<Step> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let body = raw.split('#').next().unwrap_or("").trim();
        if body.is_empty() {
            continue;
        }
        let err = |e: String| format!("line {line}: {e}: {body}");
        let w = words(body).map_err(err)?;
        if w[0].eq_ignore_ascii_case("player") {
            if !steps.is_empty() {
                return Err(err("player lines go in the header, before the first step".into()));
            }
            let (n, s) = setup(adv, &w[1..]).map_err(err)?;
            players.retain(|(m, _)| *m != n);
            players.push((n, s));
            continue;
        }
        let action = step(adv, &w).map_err(err)?;
        steps.push(Step {
            line,
            text: body.to_string(),
            action,
        });
    }
    Ok(Script { players, steps })
}
