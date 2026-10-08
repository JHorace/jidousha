//! `door.json` read whole (SPEC §16, §23; CONSTANTS §12): the three locks in Might, Wits,
//! Spirit order — each lock's demand, title, mid-sentence name, premise and four endings —
//! the four verdict titles and verdicts by locks opened, and the closed house's title and
//! verdict.
//!
//! Every number the file repeats is held to CONSTANTS here, at load: the years, the seats,
//! the danger, the renown and the three demands. A file that disagrees stops the game with
//! the path that disagrees, rather than a Door that asks for something CONSTANTS does not.

use crate::constants::{DOOR_DANGER, DOOR_LOCKS, DOOR_RENOWN, DOOR_SEATS, DOOR_YEARS};
use crate::content::{id_at, strings, text_at};
use crate::ids::{Aptitude, Outcome};
use crate::json::{At, SchemaError};

/// One of the Door's three locks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lock {
    /// What it asks for: Might, Wits, Spirit, in order.
    pub aptitude: Aptitude,
    /// What it asks for of that aptitude (34).
    pub demand: i32,
    /// "The lock of iron".
    pub title: String,
    /// "the lock of iron", mid-sentence.
    pub name: String,
    /// "The first lock is a bar of black iron ...".
    pub premise: String,
    /// The page's story by outcome (`Outcome::index`): `%1` the standing, `%2` the bearer.
    pub endings: [String; 4],
}

/// The Door's words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoorLore {
    /// The three locks, in the order they are tried.
    pub locks: Vec<Lock>,
    /// "The Door stayed shut" ... "The Door is open", by locks opened (0..=3).
    pub verdict_titles: Vec<String>,
    /// The verdict's text, by locks opened.
    pub verdicts: Vec<String>,
    /// "The house closed its doors".
    pub closed_title: String,
    /// "Too many roads went unanswered. ...".
    pub closed_verdict: String,
}

/// Read `door.json`.
pub fn read_door(at: &At<'_>) -> Result<DoorLore, SchemaError> {
    let numbers = [
        ("years", DOOR_YEARS),
        ("seats", DOOR_SEATS),
        ("danger", DOOR_DANGER),
        ("renown", DOOR_RENOWN),
    ];
    for (key, want) in numbers {
        let got = at.key(key)?.int()?;
        if got != want {
            return Err(at.key(key)?.reject(format!(
                "{key} is {got} where CONSTANTS.md §1/§12 says {want}"
            )));
        }
    }
    let mut locks = Vec::new();
    let items = at.key("locks")?.items()?;
    if items.len() != Aptitude::ALL.len() {
        return Err(at.reject(format!("{} locks where the Door has three", items.len())));
    }
    for ((aptitude, lock), want) in Aptitude::ALL.iter().zip(&items).zip(DOOR_LOCKS) {
        let named = id_at(lock, "aptitude", Aptitude::find)?;
        if named != *aptitude {
            return Err(lock.reject("locks are not in Might, Wits, Spirit order".into()));
        }
        let demand = lock.key("demand")?.int()?;
        if demand != want {
            return Err(lock.reject(format!(
                "demand {demand} where CONSTANTS.md §12 says {want}"
            )));
        }
        let endings_at = lock.key("endings")?;
        let ending = |outcome: Outcome| text_at(&endings_at, outcome.id());
        locks.push(Lock {
            aptitude: *aptitude,
            demand,
            title: text_at(lock, "title")?,
            name: text_at(lock, "name")?,
            premise: text_at(lock, "premise")?,
            endings: [
                ending(Outcome::Disaster)?,
                ending(Outcome::Setback)?,
                ending(Outcome::Success)?,
                ending(Outcome::Triumph)?,
            ],
        });
    }
    let verdict_titles = strings(at, "verdict_titles")?;
    let verdicts = strings(at, "verdicts")?;
    if verdict_titles.len() != locks.len() + 1 || verdicts.len() != locks.len() + 1 {
        return Err(at.reject(format!(
            "{} verdict titles and {} verdicts where 0 to 3 locks opened needs four of each",
            verdict_titles.len(),
            verdicts.len()
        )));
    }
    Ok(DoorLore {
        locks,
        verdict_titles,
        verdicts,
        closed_title: text_at(at, "closed_title")?,
        closed_verdict: text_at(at, "closed_verdict")?,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_door_reads_three_locks_of_thirty_four_and_four_verdicts() {
        let content = crate::content::load().expect("the content loads");
        let door = &content.door;
        let demands: Vec<i32> = door.locks.iter().map(|l| l.demand).collect();
        assert_eq!(demands, [34, 34, 34]);
        let names: Vec<&str> = door.locks.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "the lock of iron",
                "the lock of riddles",
                "the lock of breath"
            ]
        );
        assert_eq!(
            door.verdict_titles,
            [
                "The Door stayed shut",
                "The Door opened a hand's breadth",
                "The Door stood half open",
                "The Door is open"
            ]
        );
        assert_eq!(door.closed_title, "The house closed its doors");
        assert!(door.locks[0].endings[3].starts_with("%1 came down the last steps"));
    }
}
