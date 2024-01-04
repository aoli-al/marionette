use rand::{Rng, seq::SliceRandom, rngs::StdRng, SeedableRng};

pub struct RandomSource {
    pub choices: Vec<usize>,
    rng: StdRng,
    pub index: usize,
}

impl RandomSource {
    pub fn new() -> Self {
        Self {
            choices: Vec::new(),
            index: 0,
            rng: StdRng::seed_from_u64(0),
        }
    }

    pub fn from_choices(choices: Vec<usize>) -> Self {
        log::info!("Choices initialized: {:?}", choices);
        Self {
            choices,
            index: 0,
            rng: StdRng::seed_from_u64(0),
        }
    }

    pub fn next_choice(&mut self) -> usize {
        if self.choices.len() == self.index {
            self.choices.push(self.rng.gen());
        }
        self.index += 1;
        self.choices[self.index - 1]
    }
}