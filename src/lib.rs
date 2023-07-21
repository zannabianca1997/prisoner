#![feature(const_trait_impl)]
#![feature(map_many_mut)]

use std::borrow::Cow;

use rand::{distributions::Uniform, prelude::Distribution, Rng};
use serde::Deserialize;

/// Possible choices
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Choice {
    Defect,
    Collab,
}
impl From<bool> for Choice {
    fn from(value: bool) -> Self {
        match value {
            true => Self::Collab,
            false => Self::Defect,
        }
    }
}

/// Setup for the game outcomes
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Deserialize)]
pub struct Weights {
    pub defect_defect: usize,
    pub defect_collab: (usize, usize),
    pub collab_collab: usize,
}
impl Weights {
    #[must_use]
    #[inline]
    const fn outcome(&self, ch1: Choice, ch2: Choice) -> (usize, usize) {
        match (ch1, ch2) {
            (Choice::Defect, Choice::Defect) => (self.defect_defect, self.defect_defect),
            (Choice::Defect, Choice::Collab) => (self.defect_collab.0, self.defect_collab.1),
            (Choice::Collab, Choice::Defect) => (self.defect_collab.1, self.defect_collab.0),
            (Choice::Collab, Choice::Collab) => (self.collab_collab, self.collab_collab),
        }
    }
    #[must_use]
    #[inline]
    const fn max_diff(&self) -> usize {
        self.defect_collab.0.max(self.defect_collab.1)
            - self.defect_collab.0.min(self.defect_collab.1)
    }
}
impl Default for Weights {
    fn default() -> Self {
        Self {
            defect_defect: 2,
            defect_collab: (3, 0),
            collab_collab: 1,
        }
    }
}

/// A type of player
#[derive(Debug, Clone)]
pub enum PlayerFactory {
    Defector,
    Collaborator,
    Random(f64),
    RandomFixed(f64),
    TitForTat,
    TitFotTatS,
    Mean,
    Pavlov,
    Grim,
}
impl PlayerFactory {
    fn gen(&self, _weights: &Weights, rng: &mut impl Rng) -> Player {
        match self {
            PlayerFactory::Defector => Player::Defector,
            PlayerFactory::Collaborator => Player::Collaborator,
            PlayerFactory::Random(p) => Player::Random(*p),
            PlayerFactory::TitForTat => Player::TitForTat,
            PlayerFactory::TitFotTatS => Player::TitForTat2,
            PlayerFactory::RandomFixed(p) => match rng.gen_bool(*p) {
                true => Player::Collaborator,
                false => Player::Defector,
            },
            PlayerFactory::Mean => Player::Mean,
            PlayerFactory::Pavlov => Player::Pavlov,
            PlayerFactory::Grim => Player::Grim(false),
        }
    }
    pub fn name(&self) -> Cow<'_, str> {
        match self {
            PlayerFactory::Defector => "Defector".into(),
            PlayerFactory::Collaborator => "Collaborator".into(),
            PlayerFactory::Random(p) => format!("Random {:.0}%", 100. * p).into(),
            PlayerFactory::TitForTat => "TitForTat".into(),
            PlayerFactory::TitFotTatS => "TitFotTatS".into(),
            PlayerFactory::RandomFixed(p) => format!("RandomFixed {:.0}%", 100. * p).into(),
            PlayerFactory::Mean => "Mean    ".into(),
            PlayerFactory::Pavlov => "Pavlov  ".into(),
            PlayerFactory::Grim => "Grim    ".into(),
        }
    }
    pub fn description(&self) -> Cow<'_, str> {
        match self {
            PlayerFactory::Defector => "Always defect".into(),
            PlayerFactory::Collaborator => "Always collaborate".into(),
            PlayerFactory::Random(p) => format!("Collaborate {:.0}% of times", 100. * p).into(),
            PlayerFactory::TitForTat => "Collaborate, then answer with the last move".into(),
            PlayerFactory::TitFotTatS => "Defect, then answer with the last move".into(),
            PlayerFactory::RandomFixed(p) => format!(
                "Choose the move at the start (collaborate {}%), then stick with it",
                100. * p
            )
            .into(),
            PlayerFactory::Mean => {
                "Mean the other moves, then answer with the same distribution".into()
            }
            PlayerFactory::Pavlov => "Cooperate if the opponent moved alike".into(),
            PlayerFactory::Grim => "Cooperate until defected".into(),
        }
    }

    fn all() -> impl IntoIterator<Item = Self> {
        [
            Self::Defector,
            Self::Collaborator,
            Self::Random(0.5),
            Self::Random(0.9),
            Self::Random(0.1),
            Self::RandomFixed(0.5),
            Self::RandomFixed(0.9),
            Self::RandomFixed(0.1),
            Self::TitForTat,
            Self::TitFotTatS,
            Self::Mean,
            Self::Pavlov,
            Self::Grim,
        ]
    }
}

/// A player
#[derive(Debug, Clone)]
pub enum Player {
    Defector,
    Collaborator,
    Random(f64),
    TitForTat,
    TitForTat2,
    Mean,
    Pavlov,
    Grim(bool),
}
impl Player {
    fn play(&mut self, hist: (&[Choice], &[Choice]), rng: &mut impl Rng) -> Choice {
        match self {
            Player::Defector => Choice::Defect,
            Player::Collaborator => Choice::Collab,
            Player::Random(p) => rng.gen_bool(*p).into(),
            Player::TitForTat => hist.1.first().copied().unwrap_or(Choice::Collab),
            Player::TitForTat2 => hist.1.first().copied().unwrap_or(Choice::Defect),
            Player::Mean => {
                let m = if hist.1.is_empty() {
                    0.5
                } else {
                    hist.1
                        .iter()
                        .map(|c| match c {
                            Choice::Defect => 0,
                            Choice::Collab => 1,
                        })
                        .sum::<usize>() as f64
                        / hist.1.len() as f64
                };
                rng.gen_bool(m).into()
            }
            Player::Pavlov => (hist.0.last() == hist.1.last()).into(),
            Player::Grim(defected) => {
                if let Some(Choice::Defect) = hist.1.last() {
                    *defected = true;
                }
                if defected {
                    Choice::Defect
                } else {
                    Choice::Collab
                }
            }
        }
    }
}

/// Play a game between two types of players. Return -1 for total vicory of p1,
fn play(
    p1: &PlayerFactory,
    p2: &PlayerFactory,
    weights: &Weights,
    turns: usize,
    rng: &mut impl Rng,
) -> f64 {
    let mut points = 0;
    let mut hist = (Vec::with_capacity(turns), Vec::with_capacity(turns));

    let mut p1 = p1.gen(weights, rng);
    let mut p2 = p2.gen(weights, rng);

    for _ in 0..turns {
        let m1 = p1.play((&hist.0, &hist.1), rng);
        let m2 = p2.play((&hist.1, &hist.0), rng);
        hist.0.push(m1);
        hist.1.push(m2);
        let (o1, o2) = weights.outcome(m1, m2);
        points += o1 as isize - o2 as isize;
    }

    points as f64 / (weights.max_diff() * turns) as f64
}

pub struct EloPool<TD>
where
    TD: Distribution<usize>,
{
    players: Vec<(PlayerFactory, usize)>,
    weights: Weights,
    turn_distr: TD,

    /// Approximate minimum distance of two player, where one would dominate the other
    scale: f64,
    /// Correction factor
    k_factor: f64,
}
impl<TD> EloPool<TD>
where
    TD: Distribution<usize>,
{
    pub fn new(
        weights: Weights,
        turn_distr: TD,
        starting_pts: usize,
        scale: f64,
        k_factor: f64,
    ) -> Self {
        Self {
            players: PlayerFactory::all()
                .into_iter()
                .map(|p| (p, starting_pts))
                .collect(),
            weights,
            turn_distr,
            scale,
            k_factor,
        }
    }

    pub fn play(&mut self, rng: &mut impl Rng) {
        if self.players.len() < 2 {
            return;
        }
        let [i1, i2] = {
            let i1 = rng.gen_range(0..self.players.len());
            let mut i2 = rng.gen_range(0..self.players.len());
            while i1 == i2 {
                i2 = rng.gen_range(0..self.players.len());
            }
            [i1, i2]
        };
        let outcome = play(
            &self.players[i1].0,
            &self.players[i2].0,
            &self.weights,
            self.turn_distr.sample(rng),
            rng,
        );
        let rating_diff = self.players[i1].1 as f64 - self.players[i2].1 as f64;
        let expected = (rating_diff / self.scale).tanh();
        let correction = (self.k_factor * (outcome - expected)) as isize;
        // correcting the players strenght
        self.players[i1].1 = self.players[i1].1.saturating_add_signed(correction);
        self.players[i2].1 = self.players[i2].1.saturating_add_signed(-correction);
    }

    pub fn ratings(&self) -> &[(PlayerFactory, usize)] {
        &self.players
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct EloPoolConfig {
    pub weights: Weights,
    pub starting_pts: usize,
    pub scale: f64,
    pub k_factor: f64,
    pub min_turns: usize,
    pub max_turns: usize,
}

impl Default for EloPoolConfig {
    fn default() -> Self {
        Self {
            weights: Default::default(),
            starting_pts: 700,
            scale: 100.,
            k_factor: 16.,
            min_turns: 100,
            max_turns: 200,
        }
    }
}

impl From<EloPoolConfig> for EloPool<Uniform<usize>> {
    fn from(
        EloPoolConfig {
            weights,
            starting_pts,
            scale,
            k_factor,
            min_turns,
            max_turns,
        }: EloPoolConfig,
    ) -> Self {
        Self::new(
            weights,
            Uniform::new(min_turns, max_turns + 1),
            starting_pts,
            scale,
            k_factor,
        )
    }
}
