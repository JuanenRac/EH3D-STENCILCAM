// EH3D-STENCILCAM - solder paste stencils from the paste layer of a board.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! The simulation of the laser and of its link: a host twin of the controller that reads the G-code this program writes, a link that loses and damages lines
//! by chance (with a seed), a sender that retries, stops safely and resumes, so that a cut can be tried on every failure of the link with no laser. Nothing here
//! has driven a real controller: the twin knows only the few words that `output::gcode` writes.

/// A small deterministic generator (xorshift64*). It reads no clock.
#[derive(Debug, Clone)]
pub struct SimRng(u64);

impl SimRng {
    /// A generator for the seed (zero is replaced, because xorshift cannot start from zero).
    pub fn new(seed: u64) -> SimRng {
        SimRng(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number below `n` (`n` must be above zero).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// What the controller answers to a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The line was understood and done.
    Ok,
    /// The line was refused, with the reason.
    Error(String),
}

/// The twin of the controller of a laser: its position, the laser, the feed and what went wrong.
#[derive(Debug, Clone, PartialEq)]
pub struct SimLaser {
    /// X position in millimetres.
    pub x: f64,
    /// Y position in millimetres.
    pub y: f64,
    /// Whether the laser is on.
    pub laser_on: bool,
    /// The feed in millimetres per minute (0 until an `F` word).
    pub feed: f64,
    /// Whether the program has ended (`M2`).
    pub ended: bool,
    /// Lines done.
    pub lines_done: usize,
    /// Lines refused.
    pub lines_refused: usize,
    /// Length cut with the laser on, in millimetres.
    pub cut_mm: f64,
    /// Length travelled with the laser off, in millimetres.
    pub travel_mm: f64,
    /// Seconds waited for the laser (`G4`).
    pub waited_s: f64,
    /// Cutting moves made with the laser off.
    pub cuts_with_laser_off: usize,
    /// Rapid moves made with the laser on.
    pub rapids_with_laser_on: usize,
}

impl Default for SimLaser {
    fn default() -> SimLaser {
        SimLaser::new()
    }
}

fn word(token: &str) -> Result<(char, f64), String> {
    let mut chars = token.chars();
    let letter = chars.next().ok_or("an empty word")?;
    if !letter.is_ascii_uppercase() {
        return Err(format!("`{token}` does not start with a letter"));
    }
    let value: f64 = chars
        .as_str()
        .parse()
        .map_err(|_| format!("`{token}` has no number"))?;
    if !value.is_finite() {
        return Err(format!("`{token}` is not finite"));
    }
    Ok((letter, value))
}

impl SimLaser {
    /// A controller at the origin with the laser off and no feed.
    pub fn new() -> SimLaser {
        SimLaser {
            x: 0.0,
            y: 0.0,
            laser_on: false,
            feed: 0.0,
            ended: false,
            lines_done: 0,
            lines_refused: 0,
            cut_mm: 0.0,
            travel_mm: 0.0,
            waited_s: 0.0,
            cuts_with_laser_off: 0,
            rapids_with_laser_on: 0,
        }
    }

    /// The safe stop (the enable line of the laser, apart from the link): the laser goes off whatever the link does.
    pub fn safe_stop(&mut self) {
        self.laser_on = false;
    }

    /// Start a new program after a stop: the end flag is cleared, the position and the counters stay.
    pub fn restart_program(&mut self) {
        self.ended = false;
        self.laser_on = false;
        self.feed = 0.0;
    }

    /// Give the controller one line and say what it answers. A refused line changes nothing.
    pub fn line(&mut self, text: &str) -> Reply {
        match self.apply(text) {
            Ok(()) => {
                self.lines_done += 1;
                Reply::Ok
            }
            Err(why) => {
                self.lines_refused += 1;
                Reply::Error(why)
            }
        }
    }

    fn apply(&mut self, text: &str) -> Result<(), String> {
        let text = text.split(';').next().unwrap_or("").trim();
        if text.is_empty() {
            return Ok(());
        }
        let words = text
            .split_whitespace()
            .map(word)
            .collect::<Result<Vec<_>, _>>()?;
        let (letter, value) = words[0];
        let rest = &words[1..];
        let whole = value.fract() == 0.0;
        match (letter, value) {
            ('G', v) if whole && v == 21.0 && rest.is_empty() => Ok(()),
            ('G', v) if whole && v == 90.0 && rest.is_empty() => Ok(()),
            ('F', v) if rest.is_empty() => {
                if v <= 0.0 {
                    return Err("a feed that is not above zero".into());
                }
                self.feed = v;
                Ok(())
            }
            ('G', v) if whole && (v == 0.0 || v == 1.0) => {
                if self.ended {
                    return Err("the program has ended".into());
                }
                let (mut x, mut y) = (self.x, self.y);
                for &(axis, target) in rest {
                    match axis {
                        'X' => x = target,
                        'Y' => y = target,
                        other => return Err(format!("a move cannot have a `{other}` word")),
                    }
                }
                let length = ((x - self.x).powi(2) + (y - self.y).powi(2)).sqrt();
                if v == 1.0 {
                    if self.feed <= 0.0 {
                        return Err("a cutting move with no feed".into());
                    }
                    if self.laser_on {
                        self.cut_mm += length;
                    } else {
                        self.cuts_with_laser_off += 1;
                        self.travel_mm += length;
                    }
                } else {
                    if self.laser_on {
                        self.rapids_with_laser_on += 1;
                    }
                    self.travel_mm += length;
                }
                self.x = x;
                self.y = y;
                Ok(())
            }
            ('G', v) if whole && v == 4.0 => match rest {
                [('P', seconds)] if *seconds >= 0.0 => {
                    self.waited_s += seconds;
                    Ok(())
                }
                _ => Err("a wait needs one `P` word".into()),
            },
            ('M', v) if whole && v == 3.0 => {
                if self.ended {
                    return Err("the program has ended".into());
                }
                match rest {
                    [('S', power)] if *power > 0.0 => {
                        self.laser_on = true;
                        Ok(())
                    }
                    _ => Err("the laser needs one `S` word above zero".into()),
                }
            }
            ('M', v) if whole && v == 5.0 && rest.is_empty() => {
                self.laser_on = false;
                Ok(())
            }
            ('M', v) if whole && v == 2.0 && rest.is_empty() => {
                self.laser_on = false;
                self.ended = true;
                Ok(())
            }
            _ => Err(format!("`{text}` is not a line this controller knows")),
        }
    }
}

/// What the link does with a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery {
    /// The line never arrived (the sender waits and hits its time-out).
    Lost,
    /// The line arrived as this text (a damaged line too).
    Arrived(String),
}

/// A link that loses and damages lines by chance (per thousand) from a seed, and may die after a number of lines.
#[derive(Debug, Clone)]
pub struct SimLink {
    rng: SimRng,
    /// Chance in a thousand that a line is lost.
    pub lose_per_mille: u64,
    /// Chance in a thousand that a line arrives with one character damaged.
    pub damage_per_mille: u64,
    /// After this many deliveries every line is lost (the cable is out); `None` never.
    pub dies_after: Option<usize>,
    /// Lines carried so far, lost ones included.
    pub carried: usize,
}

impl SimLink {
    /// A link with no faults.
    pub fn perfect() -> SimLink {
        SimLink::lossy(1, 0, 0)
    }

    /// A link that loses and damages lines by chance.
    pub fn lossy(seed: u64, lose_per_mille: u64, damage_per_mille: u64) -> SimLink {
        SimLink {
            rng: SimRng::new(seed),
            lose_per_mille,
            damage_per_mille,
            dies_after: None,
            carried: 0,
        }
    }

    /// Carry one line.
    pub fn carry(&mut self, line: &str) -> Delivery {
        self.carried += 1;
        if self.dies_after.is_some_and(|n| self.carried > n) {
            return Delivery::Lost;
        }
        if self.rng.below(1000) < self.lose_per_mille {
            return Delivery::Lost;
        }
        if !line.is_empty() && self.rng.below(1000) < self.damage_per_mille {
            let at = self.rng.below(line.chars().count() as u64) as usize;
            let damaged: String = line
                .chars()
                .enumerate()
                .map(|(i, c)| if i == at { '?' } else { c })
                .collect();
            return Delivery::Arrived(damaged);
        }
        Delivery::Arrived(line.to_string())
    }
}

/// How a stream went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamReport {
    /// Lines the controller accepted.
    pub accepted: usize,
    /// Times a line had to be sent again (lost, or refused because it arrived damaged).
    pub resends: usize,
    /// The index of the line that could not be delivered, if the stream gave up.
    pub gave_up_at: Option<usize>,
}

/// Send the lines one at a time, each sent again up to `max_resends` times when it is lost or refused. When it gives up, the safe stop is made (the laser goes
/// off) and the report says which line it stopped at.
pub fn stream(
    lines: &[String],
    link: &mut SimLink,
    laser: &mut SimLaser,
    max_resends: usize,
) -> StreamReport {
    let mut report = StreamReport {
        accepted: 0,
        resends: 0,
        gave_up_at: None,
    };
    for (index, line) in lines.iter().enumerate() {
        let mut tries = 0;
        loop {
            let reply = match link.carry(line) {
                Delivery::Lost => None,
                Delivery::Arrived(text) => Some(laser.line(&text)),
            };
            if reply == Some(Reply::Ok) {
                report.accepted += 1;
                break;
            }
            if tries >= max_resends {
                laser.safe_stop();
                report.gave_up_at = Some(index);
                return report;
            }
            tries += 1;
            report.resends += 1;
        }
    }
    report
}

/// The lines to send to carry on a job that stopped at line `gave_up_at`: the three lines that set the units, the mode and the feed, and the job from the
/// start of the cut that was under way (the last travel move at or before the stop), so that nothing is left half cut. A cut is cut again from its start.
pub fn resume_lines(lines: &[String], gave_up_at: usize) -> Vec<String> {
    let head = 3.min(lines.len());
    let start = (head..=gave_up_at.min(lines.len().saturating_sub(1)))
        .rev()
        .find(|&i| lines[i].starts_with("G0 "))
        .unwrap_or(head);
    lines[..head]
        .iter()
        .chain(lines[start..].iter())
        .cloned()
        .collect()
}

/// The lines of a G-code text.
pub fn lines_of(text: &str) -> Vec<String> {
    text.lines().map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{gcode, gcode_with_wait, Path};

    fn square(x: f64, y: f64, side: f64) -> Path {
        vec![
            (x, y),
            (x + side, y),
            (x + side, y + side),
            (x, y + side),
            (x, y),
        ]
    }

    fn job() -> (Vec<String>, f64) {
        let paths = vec![
            square(0.0, 0.0, 1.0),
            square(5.0, 5.0, 2.0),
            square(9.0, 0.0, 1.5),
        ];
        let nominal = 4.0 + 8.0 + 6.0;
        (
            lines_of(&gcode_with_wait(&paths, 600.0, 1000, 0.5).unwrap()),
            nominal,
        )
    }

    #[test]
    fn a_perfect_link_runs_the_job_with_the_laser_off_at_the_end_and_no_unsafe_move() {
        let (lines, nominal) = job();
        let mut laser = SimLaser::new();
        let report = stream(&lines, &mut SimLink::perfect(), &mut laser, 0);
        assert_eq!(report.gave_up_at, None);
        assert_eq!(report.accepted, lines.len());
        assert!(laser.ended && !laser.laser_on);
        assert!((laser.cut_mm - nominal).abs() < 1e-6, "{}", laser.cut_mm);
        assert_eq!(laser.cuts_with_laser_off, 0);
        assert_eq!(laser.rapids_with_laser_on, 0);
        assert!((laser.waited_s - 1.5).abs() < 1e-9);
    }

    #[test]
    fn the_twin_counts_a_cut_with_the_laser_off_and_refuses_what_it_does_not_know() {
        let mut laser = SimLaser::new();
        assert_eq!(laser.line("F600"), Reply::Ok);
        assert_eq!(laser.line("G1 X5"), Reply::Ok);
        assert_eq!(laser.cuts_with_laser_off, 1);
        assert!(matches!(laser.line("G1 X?"), Reply::Error(_)));
        assert!(matches!(laser.line("G1 Z5"), Reply::Error(_)));
        assert!(matches!(laser.line("M3"), Reply::Error(_)));
        assert!(matches!(laser.line("F0"), Reply::Error(_)));
        assert!(matches!(laser.line("G2 X1"), Reply::Error(_)));
        assert_eq!(laser.lines_refused, 5);
        assert_eq!(laser.line("M2"), Reply::Ok);
        assert!(matches!(laser.line("G0 X1"), Reply::Error(_)));
        assert!(matches!(laser.line("M3 S100"), Reply::Error(_)));
    }

    #[test]
    fn a_cut_without_a_feed_is_refused() {
        let mut laser = SimLaser::new();
        assert_eq!(laser.line("M3 S100"), Reply::Ok);
        assert!(matches!(laser.line("G1 X5"), Reply::Error(_)));
        assert_eq!(laser.cut_mm, 0.0);
    }

    #[test]
    fn a_lossy_link_with_resends_still_finishes_the_job_and_the_same_seed_repeats_exactly() {
        let (lines, nominal) = job();
        let run = |seed: u64| {
            let mut laser = SimLaser::new();
            let report = stream(&lines, &mut SimLink::lossy(seed, 200, 200), &mut laser, 40);
            (report, laser)
        };
        let mut total_resends = 0;
        for seed in 1..=30 {
            let (report, laser) = run(seed);
            assert_eq!(report.gave_up_at, None, "seed {seed}");
            assert!(laser.ended && !laser.laser_on, "seed {seed}");
            assert!((laser.cut_mm - nominal).abs() < 1e-6, "seed {seed}");
            assert_eq!(laser.cuts_with_laser_off, 0, "seed {seed}");
            total_resends += report.resends;
            assert_eq!(run(seed), (report, laser), "seed {seed}");
        }
        assert!(
            total_resends > 0,
            "the link never failed, so nothing was tried"
        );
    }

    #[test]
    fn a_link_that_is_never_answered_gives_up_and_leaves_the_laser_off() {
        let (lines, _) = job();
        for dies_after in 0..lines.len() {
            let mut laser = SimLaser::new();
            let mut link = SimLink::perfect();
            link.dies_after = Some(dies_after);
            let report = stream(&lines, &mut link, &mut laser, 3);
            assert_eq!(report.gave_up_at, Some(dies_after), "{dies_after}");
            assert!(!laser.laser_on, "{dies_after}");
            assert_eq!(laser.cuts_with_laser_off, 0, "{dies_after}");
        }
    }

    #[test]
    fn after_a_stop_at_any_line_the_job_resumes_from_the_start_of_its_cut_and_finishes() {
        let (lines, nominal) = job();
        for dies_after in 0..lines.len() {
            let mut laser = SimLaser::new();
            let mut dead = SimLink::perfect();
            dead.dies_after = Some(dies_after);
            let report = stream(&lines, &mut dead, &mut laser, 2);
            let at = report.gave_up_at.expect("the link is dead");
            laser.restart_program();
            let again = resume_lines(&lines, at);
            let done = stream(&again, &mut SimLink::perfect(), &mut laser, 0);
            assert_eq!(done.gave_up_at, None, "{dies_after}");
            assert!(laser.ended && !laser.laser_on, "{dies_after}");
            assert!(
                laser.cut_mm + 1e-6 >= nominal,
                "{dies_after}: {}",
                laser.cut_mm
            );
            assert_eq!(laser.cuts_with_laser_off, 0, "{dies_after}");
            assert_eq!(laser.rapids_with_laser_on, 0, "{dies_after}");
        }
    }

    #[test]
    fn the_resume_lines_start_with_the_setup_and_the_last_travel_move() {
        let lines =
            lines_of(&gcode(&[square(0.0, 0.0, 1.0), square(4.0, 4.0, 1.0)], 600.0, 1000).unwrap());
        let second = lines.iter().rposition(|l| l.starts_with("G0 ")).unwrap();
        let again = resume_lines(&lines, second + 2);
        assert_eq!(&again[..3], &lines[..3]);
        assert_eq!(again[3], lines[second]);
        assert_eq!(resume_lines(&lines, 0)[..3], lines[..3]);
    }

    #[test]
    fn the_generator_and_the_link_never_stall_on_a_zero_seed() {
        let mut rng = SimRng::new(0);
        assert_ne!(rng.next_u64(), rng.next_u64());
        let mut link = SimLink::lossy(0, 0, 1000);
        assert!(matches!(link.carry("M5"), Delivery::Arrived(text) if text != "M5"));
        assert!(matches!(link.carry(""), Delivery::Arrived(text) if text.is_empty()));
    }
}
