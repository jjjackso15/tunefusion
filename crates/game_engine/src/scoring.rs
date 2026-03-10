//! Scoring engine for pitch-based singing game.

use serde::{Deserialize, Serialize};

use crate::pitch_detector::cents_deviation;

/// Hit rating based on pitch accuracy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitRating {
    Perfect,
    Great,
    Good,
    Ok,
    Miss,
}

impl HitRating {
    /// Get the base points for this rating.
    pub fn points(&self) -> u32 {
        match self {
            HitRating::Perfect => 100,
            HitRating::Great => 75,
            HitRating::Good => 50,
            HitRating::Ok => 25,
            HitRating::Miss => 0,
        }
    }

    /// Check if this rating continues a streak.
    pub fn continues_streak(&self) -> bool {
        matches!(self, HitRating::Perfect | HitRating::Great)
    }
}

/// Real-time score update event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreUpdate {
    /// Current total score.
    pub score: u64,
    /// Current streak of Perfect/Great hits.
    pub streak: u32,
    /// Maximum streak achieved.
    pub max_streak: u32,
    /// Running accuracy percentage [0.0, 100.0].
    pub accuracy_pct: f64,
    /// The rating of the most recent hit.
    pub last_rating: Option<HitRating>,
    /// Count of each rating type.
    pub perfect_count: u32,
    pub great_count: u32,
    pub good_count: u32,
    pub ok_count: u32,
    pub miss_count: u32,
}

/// Configuration for the scoring engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// Cents threshold for Perfect rating.
    pub perfect_cents: f64,
    /// Cents threshold for Great rating.
    pub great_cents: f64,
    /// Cents threshold for Good rating.
    pub good_cents: f64,
    /// Cents threshold for OK rating.
    pub ok_cents: f64,
    /// Streak multiplier (applied at streak >= this threshold).
    pub streak_threshold: u32,
    /// Multiplier value for streaks.
    pub streak_multiplier: f64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            perfect_cents: 25.0,
            great_cents: 50.0,
            good_cents: 100.0,
            ok_cents: 200.0,
            streak_threshold: 1,
            streak_multiplier: 1.5,
        }
    }
}

/// Scoring engine that tracks score, streak, and accuracy.
#[derive(Debug, Clone)]
pub struct ScoringEngine {
    config: ScoringConfig,
    score: u64,
    streak: u32,
    max_streak: u32,
    perfect_count: u32,
    great_count: u32,
    good_count: u32,
    ok_count: u32,
    miss_count: u32,
    total_hits: u32,
}

impl ScoringEngine {
    /// Create a new scoring engine with default configuration.
    pub fn new() -> Self {
        Self::with_config(ScoringConfig::default())
    }

    /// Create a new scoring engine with custom configuration.
    pub fn with_config(config: ScoringConfig) -> Self {
        Self {
            config,
            score: 0,
            streak: 0,
            max_streak: 0,
            perfect_count: 0,
            great_count: 0,
            good_count: 0,
            ok_count: 0,
            miss_count: 0,
            total_hits: 0,
        }
    }

    /// Reset the scoring engine to initial state.
    pub fn reset(&mut self) {
        self.score = 0;
        self.streak = 0;
        self.max_streak = 0;
        self.perfect_count = 0;
        self.great_count = 0;
        self.good_count = 0;
        self.ok_count = 0;
        self.miss_count = 0;
        self.total_hits = 0;
    }

    /// Calculate hit rating from cents deviation.
    pub fn rate_hit(&self, cents: f64) -> HitRating {
        let abs_cents = cents.abs();
        if abs_cents <= self.config.perfect_cents {
            HitRating::Perfect
        } else if abs_cents <= self.config.great_cents {
            HitRating::Great
        } else if abs_cents <= self.config.good_cents {
            HitRating::Good
        } else if abs_cents <= self.config.ok_cents {
            HitRating::Ok
        } else {
            HitRating::Miss
        }
    }

    /// Process a hit with the given user and target frequencies.
    /// Returns the rating for this hit.
    pub fn process_hit(&mut self, user_hz: f64, target_hz: f64) -> HitRating {
        let cents = cents_deviation(user_hz, target_hz);
        let rating = self.rate_hit(cents);
        self.apply_rating(rating);
        rating
    }

    /// Process a miss (no pitch detected or too far off).
    pub fn process_miss(&mut self) {
        self.apply_rating(HitRating::Miss);
    }

    /// Apply a rating to update score and streak.
    fn apply_rating(&mut self, rating: HitRating) {
        self.total_hits += 1;

        // Update rating counts
        match rating {
            HitRating::Perfect => self.perfect_count += 1,
            HitRating::Great => self.great_count += 1,
            HitRating::Good => self.good_count += 1,
            HitRating::Ok => self.ok_count += 1,
            HitRating::Miss => self.miss_count += 1,
        }

        // Update streak
        if rating.continues_streak() {
            self.streak += 1;
            self.max_streak = self.max_streak.max(self.streak);
        } else {
            self.streak = 0;
        }

        // Calculate points with streak multiplier
        let base_points = rating.points() as u64;
        let multiplier = if self.streak >= self.config.streak_threshold {
            self.config.streak_multiplier
        } else {
            1.0
        };

        self.score += (base_points as f64 * multiplier) as u64;
    }

    /// Get the current score update state.
    pub fn get_update(&self) -> ScoreUpdate {
        ScoreUpdate {
            score: self.score,
            streak: self.streak,
            max_streak: self.max_streak,
            accuracy_pct: self.accuracy_pct(),
            last_rating: None,
            perfect_count: self.perfect_count,
            great_count: self.great_count,
            good_count: self.good_count,
            ok_count: self.ok_count,
            miss_count: self.miss_count,
        }
    }

    /// Get the current score.
    pub fn score(&self) -> u64 {
        self.score
    }

    /// Get the current streak.
    pub fn streak(&self) -> u32 {
        self.streak
    }

    /// Get the maximum streak.
    pub fn max_streak(&self) -> u32 {
        self.max_streak
    }

    /// Calculate accuracy percentage.
    pub fn accuracy_pct(&self) -> f64 {
        if self.total_hits == 0 {
            return 100.0;
        }

        // Weight: Perfect=100%, Great=75%, Good=50%, OK=25%, Miss=0%
        let weighted_sum = self.perfect_count as f64 * 100.0
            + self.great_count as f64 * 75.0
            + self.good_count as f64 * 50.0
            + self.ok_count as f64 * 25.0;

        weighted_sum / self.total_hits as f64
    }

    /// Get counts for each rating.
    pub fn rating_counts(&self) -> (u32, u32, u32, u32, u32) {
        (
            self.perfect_count,
            self.great_count,
            self.good_count,
            self.ok_count,
            self.miss_count,
        )
    }
}

impl Default for ScoringEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_hit() {
        let engine = ScoringEngine::new();

        assert_eq!(engine.rate_hit(0.0), HitRating::Perfect);
        assert_eq!(engine.rate_hit(20.0), HitRating::Perfect);
        assert_eq!(engine.rate_hit(-20.0), HitRating::Perfect);

        assert_eq!(engine.rate_hit(30.0), HitRating::Great);
        assert_eq!(engine.rate_hit(50.0), HitRating::Great);

        assert_eq!(engine.rate_hit(60.0), HitRating::Good);
        assert_eq!(engine.rate_hit(100.0), HitRating::Good);

        assert_eq!(engine.rate_hit(150.0), HitRating::Ok);
        assert_eq!(engine.rate_hit(200.0), HitRating::Ok);

        assert_eq!(engine.rate_hit(250.0), HitRating::Miss);
    }

    #[test]
    fn test_scoring() {
        let mut engine = ScoringEngine::new();

        // Perfect hit = 100 pts (no multiplier yet)
        engine.process_hit(440.0, 440.0);
        assert_eq!(engine.score(), 100);
        assert_eq!(engine.streak(), 1);

        // Another perfect = 150 pts (1.5x multiplier)
        engine.process_hit(440.0, 440.0);
        assert_eq!(engine.score(), 250);
        assert_eq!(engine.streak(), 2);
    }

    #[test]
    fn test_streak_breaks() {
        let mut engine = ScoringEngine::new();

        engine.process_hit(440.0, 440.0); // Perfect
        engine.process_hit(440.0, 440.0); // Perfect
        assert_eq!(engine.streak(), 2);

        engine.process_miss(); // Miss breaks streak
        assert_eq!(engine.streak(), 0);
        assert_eq!(engine.max_streak(), 2);
    }

    #[test]
    fn test_accuracy() {
        let mut engine = ScoringEngine::new();

        // All perfect = 100%
        engine.process_hit(440.0, 440.0);
        engine.process_hit(440.0, 440.0);
        assert!((engine.accuracy_pct() - 100.0).abs() < 0.01);

        // Add a miss
        engine.process_miss();
        // (100 + 100 + 0) / 3 = 66.67%
        assert!((engine.accuracy_pct() - 66.67).abs() < 0.1);
    }
}
