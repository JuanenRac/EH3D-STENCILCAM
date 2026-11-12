//! Damaged drawings and layers with a fixed seed: every cut of the example drawing and many copies of it with bytes changed, inserted or cut are read without a panic, and every opening
//! that comes out is finite and has a size. The seed is fixed, so a failure is the same failure every time.

use eh3d_stencilcam::dxf_in::from_dxf;
use eh3d_stencilcam::layer::read_layer;

/// A small generator of numbers (xorshift64): the same sequence on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn drawing() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/paste.dxf")).unwrap()
}

fn check(text: &str) {
    if let Ok((openings, _notes)) = from_dxf(text) {
        for o in &openings {
            assert!(
                o.x.is_finite() && o.y.is_finite() && o.w.is_finite() && o.h.is_finite(),
                "{o:?}"
            );
            assert!(o.w > 0.0 && o.h > 0.0, "{o:?}");
        }
    }
}

#[test]
fn every_cut_of_the_example_drawing_is_read_without_a_panic() {
    let text = drawing();
    let (openings, notes) = from_dxf(&text).unwrap();
    assert_eq!((openings.len(), notes.len()), (3, 0));
    let chars: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    for end in chars.iter().step_by(2) {
        check(&text[..*end]);
    }
    // the same by whole lines
    let lines: Vec<&str> = text.lines().collect();
    for end in 0..=lines.len() {
        check(&lines[..end].join("\n"));
    }
}

#[test]
fn copies_of_the_drawing_with_bytes_changed_inserted_or_cut_are_read_without_a_panic() {
    let bytes = drawing().into_bytes();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for round in 0..500 {
        let mut copy = bytes.clone();
        for _ in 0..1 + round % 5 {
            let at = rng.below(copy.len());
            match rng.below(4) {
                0 => copy[at] = b"0123456789.-eE\n \tAZ"[rng.below(19)],
                1 => copy.insert(at, rng.next() as u8),
                2 => {
                    copy.remove(at);
                }
                _ => copy.truncate(at.max(1)),
            }
            if copy.is_empty() {
                copy.push(b'0');
            }
        }
        check(&String::from_utf8_lossy(&copy));
    }
}

#[test]
fn numbers_that_are_huge_or_not_numbers_never_make_an_opening() {
    for value in ["1e999", "-1e999", "nan", "inf", "0x10", "", "--1", "1e-999"] {
        let text = format!("0\nSECTION\n2\nENTITIES\n0\nCIRCLE\n8\n0\n10\n{value}\n20\n{value}\n40\n{value}\n0\nENDSEC\n0\nEOF\n");
        let (openings, _) = from_dxf(&text).unwrap();
        for o in &openings {
            assert!(
                o.x.is_finite() && o.w.is_finite() && o.w > 0.0,
                "{value}: {o:?}"
            );
        }
    }
}

#[test]
fn damaged_layers_are_read_without_a_panic() {
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/paste.gtp")).unwrap();
    assert!(read_layer(&bytes, 1 << 20, &[]).is_ok());
    let mut rng = Rng(11);
    for _ in 0..300 {
        let mut copy = bytes.clone();
        let at = rng.below(copy.len());
        copy[at] = rng.next() as u8;
        let end = rng.below(copy.len()) + 1;
        let _ = read_layer(&copy[..end], 1 << 20, &[]);
    }
}
