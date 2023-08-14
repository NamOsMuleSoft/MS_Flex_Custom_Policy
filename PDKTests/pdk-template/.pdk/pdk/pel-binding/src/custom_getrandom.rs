// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::{
    cell::RefCell,
    time::{SystemTime, UNIX_EPOCH},
};

use classy::Host;
use getrandom::{register_custom_getrandom, Error};
use oorandom::Rand32;

thread_local! {
    static RANDOMIZER: Randomizer = Randomizer::new(classy::DefaultHost{}.get_current_time());
}

struct Randomizer {
    rand: RefCell<Rand32>,
}

impl Randomizer {
    fn new(seed: SystemTime) -> Self {
        let seed = seed
            .duration_since(UNIX_EPOCH)
            .expect("Invalid timer epoch")
            .as_micros();

        Self {
            rand: RefCell::new(Rand32::new(seed as u64)),
        }
    }

    fn randomize(&self, buffer: &mut [u8]) {
        let mut generator = self.rand.borrow_mut();

        // Generates a fixed size array of random bytes
        let mut random_array = || generator.rand_u32().to_le_bytes();

        // Infinite iterator of random bytes
        let randoms = std::iter::from_fn(|| Some(random_array())).flatten();

        buffer
            .iter_mut()
            .zip(randoms)
            .for_each(|(byte, random)| *byte = random);
    }
}

// Fallback for unavailable random generator.
// This generator is not cryptographically safe, and can be avoided by compiling for WASI target.
pub fn non_wasi_prand(buffer: &mut [u8]) -> Result<(), Error> {
    RANDOMIZER.with(|r| r.randomize(buffer));

    Ok(())
}

register_custom_getrandom!(non_wasi_prand);

#[cfg(test)]
mod tests {
    use super::Randomizer;

    #[test]
    fn randomize_buffer() {
        let randomizer = Randomizer::new(std::time::SystemTime::now());

        let original_bytes = [0; 20];
        let mut randomized_bytes = original_bytes;

        randomizer.randomize(&mut randomized_bytes);

        assert_ne!(original_bytes, randomized_bytes);
    }
}
